use crate::error::RendererError;
use crate::vertex::Vertex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Internal renderer storing wgpu objects and a per-frame vertex list.
///
/// `wgpu::Surface` carries a lifetime parameter because (on some platforms) the surface
/// must not outlive the underlying windowing resources.
///
/// In practice, many apps store their window behind an `Arc` and keep it alive for the duration
/// of the renderer. We support that pattern by requiring `W: Clone` and creating the surface from
/// an owned clone of `window`, which allows us to store the surface as `'static`.
pub struct Renderer<W> {
    // wgpu objects
    // These fields are kept to ensure the underlying windowing resources outlive the surface.
    // (And they may be useful later for advanced features / diagnostics.)
    _window: W,
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,

    // per-frame collected vertices
    vertices: Vec<Vertex>,

    vertex_buffer: wgpu::Buffer,
    // number of vertices capacity
    vertex_capacity: usize,
    // current clear color stored in begin_frame
    clear_color: Option<[f32; 4]>,
}

impl<W> Renderer<W>
where
    W: HasWindowHandle + HasDisplayHandle + wgpu::WasmNotSendSync + Sync + Clone + 'static,
{
    /// Async init for the renderer
    pub async fn new(window: W) -> Result<Self, RendererError> {
        let backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Creating a surface ties it to the lifetime of the underlying windowing resources.
        // We create the surface from an owned clone (e.g. `Arc<Window>`) so the surface can be stored
        // with a `'static` lifetime while `self.window` keeps the resources alive.
        let surface = instance
            .create_surface(window.clone())
            .map_err(|_| RendererError::Surface("failed to create surface".into()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RendererError::Surface("no suitable adapter".into()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("libforge_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;

        let initial_capacity = 4096;

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("libforge_vertex_buffer"),
            size: (initial_capacity * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Choose a surface format
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
                )
            })
            .unwrap_or(caps.formats[0]);

        // default size: 800x600
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 800,
            height: 600,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basic_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/basic.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("basic_pipeline"),
            layout: Some(&pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Ok(Self {
            _window: window,
            _instance: instance,
            surface,
            _adapter: adapter,
            device,
            queue,
            surface_config,
            pipeline,
            vertices: Vec::with_capacity(1024),
            vertex_buffer,
            clear_color: None,
            vertex_capacity: initial_capacity,
        })
    }

    pub fn ensure_vertex_capacity(&mut self, needed: usize) {
        if needed <= self.vertex_capacity {
            return;
        }

        let new_capacity = needed.next_power_of_two();
        let new_size = (new_capacity * std::mem::size_of::<Vertex>()) as u64;

        self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("libforge_vertex_buffer"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.vertex_capacity = new_capacity;
    }

    /// Called each frame to reset the command list and optionally set clear color
    pub fn begin_frame(&mut self, clear: Option<[f32; 4]>) {
        self.vertices.clear();
        self.clear_color = clear;
    }

    /// Draw a filled rectangle in logical pixel coordinates. We convert to NDC here.
    pub fn draw_rect(&mut self, rect: crate::Rect, color: crate::Color) {
        // convert to NDC (x,y logical -> -1..1)
        let width = self.surface_config.width as f32;
        let height = self.surface_config.height as f32;

        // Note: wgpu and winit coordinate spaces: we'll treat y=0 at top and convert to NDC y with origin center.
        let x0 = (rect.x / width) * 2.0 - 1.0;
        let y0 = 1.0 - (rect.y / height) * 2.0; // flip y
        let x1 = ((rect.x + rect.w) / width) * 2.0 - 1.0;
        let y1 = 1.0 - ((rect.y + rect.h) / height) * 2.0;

        let c = color.0;

        // two triangles (triangle list), 6 vertices
        let vertices = [
            Vertex {
                pos: [x0, y0],
                color: c,
            },
            Vertex {
                pos: [x1, y0],
                color: c,
            },
            Vertex {
                pos: [x1, y1],
                color: c,
            },
            Vertex {
                pos: [x0, y0],
                color: c,
            },
            Vertex {
                pos: [x1, y1],
                color: c,
            },
            Vertex {
                pos: [x0, y1],
                color: c,
            },
        ];

        self.vertices.extend_from_slice(&vertices);
    }

    /// Draws a line
    pub fn draw_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        let quad = line_to_quad(x1, y1, x2, y2, thickness);
        let verts = quad_to_vertices(
            quad,
            color,
            self.surface_config.width as f32,
            self.surface_config.height as f32,
        );
        self.vertices.extend(&verts);
    }

    /// Resize: reconfigure surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// End frame: create buffers, record commands, submit, and present.
    pub fn end_frame(&mut self) -> Result<(), RendererError> {
        // acquire next texture
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                // Try reconfigure then error
                self.surface.configure(&self.device, &self.surface_config);
                return Err(RendererError::Surface(format!("{:?}", e)));
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // create vertex buffer (init with data)
        let needed = self.vertices.len();
        self.ensure_vertex_capacity(needed);

        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));

        // encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });

        {
            let clear = self.clear_color.unwrap_or([0.1, 0.1, 0.1, 1.0]);
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                occlusion_query_set: None,
                timestamp_writes: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0] as f64,
                            g: clear[1] as f64,
                            b: clear[2] as f64,
                            a: clear[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.pipeline);
            if !self.vertices.is_empty() {
                rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                let vertex_count = self.vertices.len() as u32;
                rpass.draw(0..vertex_count, 0..1);
            }
        }

        // submit
        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub(crate) fn rect_to_ndc_coords(rect: crate::Rect, width: u32, height: u32) -> [f32; 12] {
    let w = width as f32;
    let h = height as f32;

    let x0 = (rect.x / w) * 2.0 - 1.0;
    let y0 = 1.0 - (rect.y / h) * 2.0;

    let x1 = ((rect.x + rect.w) / w) * 2.0 - 1.0;
    let y1 = 1.0 - ((rect.y + rect.h) / h) * 2.0;

    [
        x0, y0, // TL
        x1, y0, // TR
        x1, y1, // BR
        x0, y0, // TL
        x1, y1, // BR
        x0, y1, // BL
    ]
}

pub(crate) fn line_to_quad(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32) -> [(f32, f32); 4] {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt().max(0.0001);

    // unit direction
    let ux = dx / len;
    let uy = dy / len;

    // perpendicular
    let px = -uy;
    let py = ux;

    let half = thickness / 2.0;
    let hx = px * half;
    let hy = py * half;

    [
        (x1 + hx, y1 + hy), // p1
        (x1 - hx, y1 - hy), // p2
        (x2 + hx, y2 + hy), // p3
        (x2 - hx, y2 - hy), // p4
    ]
}

pub(crate) fn quad_to_vertices(
    p: [(f32, f32); 4],
    color: [f32; 4],
    width: f32,
    height: f32,
) -> [Vertex; 6] {
    let to_ndc = |x: f32, y: f32| {
        let w = width as f32;
        let h = height as f32;
        let nx = (x / w) * 2.0 - 1.0;
        let ny = 1.0 - (y / h) * 2.0;
        (nx, ny)
    };

    let (x1, y1) = to_ndc(p[0].0, p[0].1);
    let (x2, y2) = to_ndc(p[1].0, p[1].1);
    let (x3, y3) = to_ndc(p[2].0, p[2].1);
    let (x4, y4) = to_ndc(p[3].0, p[3].1);

    [
        Vertex {
            pos: [x1, y1],
            color,
        },
        Vertex {
            pos: [x3, y3],
            color,
        },
        Vertex {
            pos: [x4, y4],
            color,
        },
        Vertex {
            pos: [x1, y1],
            color,
        },
        Vertex {
            pos: [x4, y4],
            color,
        },
        Vertex {
            pos: [x2, y2],
            color,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck;
    use std::mem::size_of;

    #[test]
    fn vertex_pod_layout() {
        // Vertex = [f32;2] + [f32;4] => 2*4 + 4*4 = 8 + 16 = 24 bytes
        assert_eq!(size_of::<Vertex>(), 24);
        let v = Vertex {
            pos: [0.0, 0.0],
            color: [1.0, 0.0, 0.0, 1.0],
        };
        // bytemuck::bytes_of is a compile-time checked cast to &[u8]
        let b = bytemuck::bytes_of(&v);
        assert_eq!(b.len(), size_of::<Vertex>());
    }

    #[test]
    fn rect_to_ndc_basic() {
        let rect = crate::Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        let coords = rect_to_ndc_coords(rect, 200, 100);

        // top-left
        assert_eq!(coords[0], -1.0);
        assert_eq!(coords[1], 1.0);

        // top-right
        assert_eq!(coords[2], 1.0);
        assert_eq!(coords[3], 1.0);

        // bottom-right
        assert_eq!(coords[4], 1.0);
        assert_eq!(coords[5], -1.0);

        // bottom-left
        assert_eq!(coords[10], -1.0);
        assert_eq!(coords[11], -1.0);
    }
}
