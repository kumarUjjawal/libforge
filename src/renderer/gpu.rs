use crate::error::RendererError;
use crate::vertex::Vertex;
use glam::Mat4;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;

pub(crate) struct RendererGpu<W> {
    // These fields are kept to ensure the underlying windowing resources outlive the surface.
    _window: W,
    _instance: wgpu::Instance,
    pub(crate) surface: wgpu::Surface<'static>,
    _adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,

    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) texture_pipeline: wgpu::RenderPipeline,

    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) vertex_capacity: usize,

    pub(crate) tex_bind_group_layout: wgpu::BindGroupLayout,

    pub(crate) transform_buffer: wgpu::Buffer,
    pub(crate) transform_bind_group: wgpu::BindGroup,
}

impl<W> RendererGpu<W>
where
    W: HasWindowHandle + HasDisplayHandle + wgpu::WasmNotSendSync + Sync + Clone + 'static,
{
    pub(crate) fn end_frame(
        &mut self,
        vertices: &[Vertex],
        commands: &[super::DrawCommand],
        clear_color: Option<[f32; 4]>,
        textures: &std::collections::HashMap<u32, super::Texture>,
    ) -> Result<(), RendererError> {
        // acquire next texture
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                self.surface.configure(&self.device, &self.surface_config);
                return Err(RendererError::Surface(format!("{:?}", e)));
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // upload vertex data
        self.upload_vertices(vertices);

        // command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });

        let clear = clear_color.unwrap_or([0.1, 0.1, 0.1, 1.0]);

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

        // Bind the transform bind group at index 0 (applies to both pipelines).
        rpass.set_bind_group(0, &self.transform_bind_group, &[]);

        if !vertices.is_empty() {
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        }

        for cmd in commands {
            match *cmd {
                super::DrawCommand::Color { start, count } => {
                    rpass.set_pipeline(&self.pipeline); // color pipeline
                    let s = start as u32;
                    let e = s + count as u32;
                    rpass.draw(s..e, 0..1);
                }
                super::DrawCommand::Texture { tex, start, count } => {
                    rpass.set_pipeline(&self.texture_pipeline);
                    if let Some(texdata) = textures.get(&tex.0) {
                        rpass.set_bind_group(1, &texdata.bind_group, &[]);
                    } else {
                        continue;
                    }
                    let s = start as u32;
                    let e = s + count as u32;
                    rpass.draw(s..e, 0..1);
                }
            }
        }

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    pub(crate) async fn new(window: W) -> Result<Self, RendererError> {
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

        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("transform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new((16 * std::mem::size_of::<f32>()) as u64)
                                .unwrap(),
                        ),
                    },
                    count: None,
                }],
            });

        let identity = Mat4::IDENTITY;
        let identity_cols = identity.to_cols_array();

        let transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("transform_buffer"),
            contents: bytemuck::cast_slice(&identity_cols),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transform_bind_group"),
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
        });

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
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let (width, height) = (800u32, 600u32);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![surface_format],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basic_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/basic.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&transform_bind_group_layout],
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
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Texture pipeline setup
        let tex_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tex_bind_group_layout"),
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

        let texture_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("texture_pipeline_layout"),
            bind_group_layouts: &[&transform_bind_group_layout, &tex_bind_group_layout],
            push_constant_ranges: &[],
        });

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
            texture_pipeline,
            vertex_buffer,
            vertex_capacity: initial_capacity,
            tex_bind_group_layout,
            transform_buffer,
            transform_bind_group,
        })
    }

    pub(crate) fn ensure_vertex_capacity(&mut self, needed: usize) {
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

    pub(crate) fn upload_vertices(&mut self, vertices: &[Vertex]) {
        let needed = vertices.len();
        self.ensure_vertex_capacity(needed);
        if needed > 0 {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    pub(crate) fn write_transform(&mut self, mat: Mat4) {
        let cols = mat.to_cols_array();
        self.queue
            .write_buffer(&self.transform_buffer, 0, bytemuck::cast_slice(&cols));
    }

    pub(crate) fn create_texture_bind_group(
        &self,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("texture_bind_group"),
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}
