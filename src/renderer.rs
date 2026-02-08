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

    // Draw commands (so we know which pipeline to bind per batch)
    pub commands: Vec<DrawCommand>,

    // texture manager
    pub texture: std::collections::HashMap<u32, Texture>,
    pub next_texture_id: u32,

    // bind group layout
    pub tex_bind_group_layout: wgpu::BindGroupLayout,

    // pipeline
    pub texture_pipeline: wgpu::RenderPipeline,
}

#[derive(Clone, Copy, Debug)]
pub struct TextureId(pub u32);

pub enum DrawCommand {
    Color {
        start: usize,
        count: usize,
    },
    Texture {
        tex: TextureId,
        start: usize,
        count: usize,
    },
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
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
                entry_point: Some("fs_color"),
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

        // create texture bind group layout: binding 0 = texture, binding 1 = sampler
        let tex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Create a pipeline layout that includes the tex bind group layout
        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("texture_pipeline_layout"),
                bind_group_layouts: &[&tex_bind_group_layout],
                push_constant_ranges: &[],
            });

        // re-use the same shader module but provide a different fragment entry point (see WGSL below)
        let texture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_pipeline"),
            layout: Some(&texture_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_texture"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
            texture: std::collections::HashMap::new(),
            next_texture_id: 0,
            tex_bind_group_layout,
            commands: Vec::new(),
            texture_pipeline,
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
                uv: [0.0, 0.0],
                color: c,
            },
            Vertex {
                pos: [x1, y0],
                uv: [0.0, 0.0],
                color: c,
            },
            Vertex {
                pos: [x1, y1],
                uv: [0.0, 0.0],
                color: c,
            },
            Vertex {
                pos: [x0, y0],
                uv: [0.0, 0.0],
                color: c,
            },
            Vertex {
                pos: [x1, y1],
                uv: [0.0, 0.0],
                color: c,
            },
            Vertex {
                pos: [x0, y1],
                uv: [0.0, 0.0],
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

    /// Draws a circle
    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, segments: usize, color: [f32; 4]) {
        let verts = circle_to_vertices(
            x,
            y,
            radius,
            segments,
            color,
            self.surface_config.width as u32,
            self.surface_config.height as u32,
        );

        // ensure capacity
        let needed_total = self.vertices.len() + verts.len();
        if needed_total > self.vertex_capacity {
            self.ensure_vertex_capacity(needed_total);
        }
        self.vertices.extend_from_slice(&verts);
    }

    pub fn draw_texture(&mut self, id: TextureId, dest: crate::Rect, tint: [f32; 4]) {
        // compute vertices (positions + uvs)
        let x0 = dest.x;
        let y0 = dest.y;
        let x1 = dest.x + dest.w;
        let y1 = dest.y + dest.h;

        // UV coordinates: (0,0) top-left, (1,1) bottom-right
        let u0 = 0.0f32;
        let v0 = 0.0f32;
        let u1 = 1.0f32;
        let v1 = 1.0f32;

        // convert to NDC and pack Vertex with uv
        let to_ndc = |x: f32, y: f32| {
            let w = self.surface_config.width as f32;
            let h = self.surface_config.height as f32;
            let nx = (x / w) * 2.0 - 1.0;
            let ny = 1.0 - (y / h) * 2.0;
            (nx, ny)
        };

        let (nx0, ny0) = to_ndc(x0, y0);
        let (nx1, ny1) = to_ndc(x1, y1);

        // Quad: p0 = (nx0, ny0) TL, p1 = (nx1, ny0) TR, p2 = (nx1, ny1) BR, p3 = (nx0, ny1) BL
        let start = self.vertices.len();

        let verts = [
            Vertex {
                pos: [nx0, ny0],
                uv: [u0, v0],
                color: tint,
            },
            Vertex {
                pos: [nx1, ny0],
                uv: [u1, v0],
                color: tint,
            },
            Vertex {
                pos: [nx1, ny1],
                uv: [u1, v1],
                color: tint,
            },
            Vertex {
                pos: [nx0, ny0],
                uv: [u0, v0],
                color: tint,
            },
            Vertex {
                pos: [nx1, ny1],
                uv: [u1, v1],
                color: tint,
            },
            Vertex {
                pos: [nx0, ny1],
                uv: [u0, v1],
                color: tint,
            },
        ];

        // ensure capacity for new vertices
        let needed_total = self.vertices.len() + verts.len();
        self.ensure_vertex_capacity(needed_total);

        self.vertices.extend_from_slice(&verts);
        self.commands.push(DrawCommand::Texture {
            tex: id,
            start,
            count: verts.len(),
        });
    }

    pub fn load_texture_from_bytes(
        &mut self,
        name: &str,
        bytes: &[u8],
    ) -> Result<TextureId, RendererError> {
        // decode with image crate
        let img = image::load_from_memory(bytes)
            .map_err(|e| RendererError::Internal(format!("{:?}", e)))?;
        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // upload data
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("libforge_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let id = {
            let id = self.next_texture_id;
            self.next_texture_id += 1;
            id
        };

        self.texture.insert(
            id,
            Texture {
                texture,
                view,
                sampler,
                bind_group,
                width,
                height,
            },
        );
        Ok(TextureId(id))
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
        //
        //  Acquire next surface texture
        //
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                // Try to recover by reconfiguring
                self.surface.configure(&self.device, &self.surface_config);
                return Err(RendererError::Surface(format!("{:?}", e)));
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        //
        //  Upload vertices to persistent GPU buffer
        //
        let needed = self.vertices.len();
        self.ensure_vertex_capacity(needed);

        if needed > 0 {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }

        //
        //  Begin command encoder + render pass
        //
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });

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

        //
        //  Bind vertex buffer
        //
        if !self.vertices.is_empty() {
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        }

        //
        //  Replay draw commands
        //
        for cmd in &self.commands {
            match *cmd {
                DrawCommand::Color { start, count } => {
                    rpass.set_pipeline(&self.pipeline); // solid color pipeline

                    let s = start as u32;
                    let e = s + count as u32;

                    rpass.draw(s..e, 0..1);
                }

                DrawCommand::Texture { tex, start, count } => {
                    rpass.set_pipeline(&self.texture_pipeline); // textured pipeline

                    if let Some(texdata) = self.texture.get(&tex.0) {
                        rpass.set_bind_group(0, &texdata.bind_group, &[]);
                    } else {
                        // Texture missing; skip draw instead of crashing
                        continue;
                    }

                    let s = start as u32;
                    let e = s + count as u32;

                    rpass.draw(s..e, 0..1);
                }
            }
        }

        drop(rpass); // drop borrow before submit

        //
        // Submit GPU commands + present frame
        //
        self.queue.submit(Some(encoder.finish()));
        output.present();

        //
        // 7. Clear CPU-side data for next frame
        //
        self.vertices.clear();
        self.commands.clear();

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

pub(crate) fn circle_to_vertices(
    cx: f32,
    cy: f32,
    radius: f32,
    segments: usize,
    color: [f32; 4],
    width: u32,
    height: u32,
) -> Vec<Vertex> {
    let mut verts = Vec::with_capacity(segments * 3);

    // convert pixel coords -> NDC
    let to_ndc = |x: f32, y: f32| {
        let w = width as f32;
        let h = height as f32;
        let nx = (x / w) * 2.0 - 1.0;
        let ny = 1.0 - (y / h) * 2.0;
        (nx, ny)
    };

    let (cx_ndc, cy_ndc) = to_ndc(cx, cy);

    let seg = std::cmp::max(2, segments);
    let two_pi = std::f32::consts::TAU;
    let angle_step = two_pi / seg as f32;

    for i in 0..seg {
        let a0 = i as f32 * angle_step;
        let a1 = ((i + 1) % seg) as f32 * angle_step;

        let x0 = cx + a0.cos() * radius;
        let y0 = cy + a0.sin() * radius;

        let x1 = cx + a1.cos() * radius;
        let y1 = cy + a1.sin() * radius;

        let (x0_ndc, y0_ndc) = to_ndc(x0, y0);
        let (x1_ndc, y1_ndc) = to_ndc(x1, y1);

        verts.push(Vertex {
            pos: [cx_ndc, cy_ndc],
            uv: [0.0, 0.0],
            color,
        });
        verts.push(Vertex {
            pos: [x0_ndc, y0_ndc],

            uv: [0.0, 0.0],
            color,
        });
        verts.push(Vertex {
            pos: [x1_ndc, y1_ndc],

            uv: [0.0, 0.0],
            color,
        });
    }
    verts
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

            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [x3, y3],

            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [x4, y4],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [x1, y1],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [x4, y4],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [x2, y2],
            uv: [0.0, 0.0],
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
            uv: [0.0, 0.0],
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

    #[test]
    fn circle_vertex_count_and_basic_positions() {
        // small segments count for deterministic test
        let seg = 4;
        let cx = 50.0f32;
        let cy = 40.0f32;
        let radius = 10.0f32;
        let color = [1.0, 0.0, 0.0, 1.0];
        let width = 200u32;
        let height = 100u32;

        let verts = crate::renderer::circle_to_vertices(cx, cy, radius, seg, color, width, height);
        // for seg triangles, we expect seg * 3 vertices
        assert_eq!(verts.len(), seg * 3);

        // center of first triangle should be center in NDC
        let center_ndc_x = (cx / width as f32) * 2.0 - 1.0;
        let center_ndc_y = 1.0 - (cy / height as f32) * 2.0;
        assert!((verts[0].pos[0] - center_ndc_x).abs() < 1e-6);
        assert!((verts[0].pos[1] - center_ndc_y).abs() < 1e-6);
    }
}
