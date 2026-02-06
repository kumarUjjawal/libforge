use bytemuck::{Pod, Zeroable};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use thiserror::Error;
use wgpu::util::DeviceExt;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("wgpu error")]
    Wgpu(#[from] wgpu::RequestDeviceError),

    #[error("swapchain error: {0}")]
    Swapchain(String),
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x4  // color
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

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
            .map_err(|_| RendererError::Swapchain("failed to create surface".into()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RendererError::Swapchain("no suitable adapter".into()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("libforge_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;

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
            clear_color: None,
        })
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
                return Err(RendererError::Swapchain(format!("{:?}", e)));
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // create vertex buffer (init with data)
        let vertex_data = bytemuck::cast_slice(&self.vertices);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: vertex_data,
                usage: wgpu::BufferUsages::VERTEX,
            });

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
                rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
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
