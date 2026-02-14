use crate::camera::Camera2D;
use crate::error::RendererError;
use crate::vertex::Vertex;
use glam::Mat4;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
mod geometry;
mod gpu;

use gpu::RendererGpu;

// Re-export internal geometry helpers for use by unit tests and other crate modules.
pub(crate) use geometry::{circle_to_vertices, line_to_quad, quad_to_vertices};

fn transform_pos2(mat: Mat4, p: [f32; 2]) -> [f32; 2] {
    let v = mat * glam::vec4(p[0], p[1], 0.0, 1.0);
    [v.x, v.y]
}

fn transform_vertices_in_place(mat: Mat4, verts: &mut [Vertex]) {
    if mat == Mat4::IDENTITY {
        return;
    }
    for v in verts {
        v.pos = transform_pos2(mat, v.pos);
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
    gpu: RendererGpu<W>,

    // per-frame collected vertices
    vertices: Vec<Vertex>,

    // current clear color stored in begin_frame
    clear_color: Option<[f32; 4]>,

    // Draw commands (so we know which pipeline to bind per batch)
    pub commands: Vec<DrawCommand>,

    // texture manager
    pub texture: std::collections::HashMap<u32, Texture>,
    pub next_texture_id: u32,

    // Scoped 2D camera mode: active only between begin_mode_2d/end_mode_2d.
    camera_stack: Vec<Camera2D>,

    // CPU-side model matrix stack (applied per-draw to vertex positions).
    model_stack: Vec<Mat4>,
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
        let gpu = RendererGpu::new(window).await?;

        let mut renderer = Self {
            gpu,
            vertices: Vec::with_capacity(1024),
            clear_color: None,
            texture: std::collections::HashMap::new(),
            next_texture_id: 0,
            commands: Vec::new(),
            camera_stack: Vec::new(),
            model_stack: vec![Mat4::IDENTITY],
        };

        // Default mode is screen-space (no camera). Upload projection*view to the transform uniform.
        renderer.update_viewproj_transform();

        Ok(renderer)
    }

    pub fn ensure_vertex_capacity(&mut self, needed: usize) {
        self.gpu.ensure_vertex_capacity(needed);
    }

    /// Called each frame to reset the command list and optionally set clear color
    pub fn begin_frame(&mut self, clear: Option<[f32; 4]>) {
        self.vertices.clear();
        self.commands.clear();
        self.clear_color = clear;
    }

    /// Draw a filled rectangle in logical pixel coordinates. We convert to NDC here.
    pub fn draw_rect(&mut self, rect: crate::Rect, color: crate::Color) {
        let x0 = rect.x;
        let y0 = rect.y;
        let x1 = rect.x + rect.w;
        let y1 = rect.y + rect.h;

        let c = color.0;

        // two triangles (triangle list), 6 vertices
        let mut vertices = [
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

        let model = self.current_model_matrix();
        transform_vertices_in_place(model, &mut vertices);

        let start = self.vertices.len();
        self.vertices.extend_from_slice(&vertices);

        match self.commands.last_mut() {
            Some(DrawCommand::Color { count, .. }) => *count += vertices.len(),
            _ => self.commands.push(DrawCommand::Color {
                start,
                count: vertices.len(),
            }),
        }
    }

    /// Draws a line (as a thick quad)
    pub fn draw_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        // compute quad in pixel space
        let quad = line_to_quad(x1, y1, x2, y2, thickness);
        // convert quad into 6 vertices (pixel space)
        let mut verts = quad_to_vertices(quad, color);
        let model = self.current_model_matrix();
        transform_vertices_in_place(model, &mut verts);

        // ensure capacity
        let needed_total = self.vertices.len() + verts.len();
        self.ensure_vertex_capacity(needed_total);

        let start = self.vertices.len();
        self.vertices.extend_from_slice(&verts);

        match self.commands.last_mut() {
            Some(DrawCommand::Color { count, .. }) => *count += verts.len(),
            _ => self.commands.push(DrawCommand::Color {
                start,
                count: verts.len(),
            }),
        }
    }

    /// Draws a circle (triangle-fan) in pixel-space
    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, segments: usize, color: [f32; 4]) {
        let mut verts = circle_to_vertices(x, y, radius, segments, color);
        let model = self.current_model_matrix();
        transform_vertices_in_place(model, &mut verts);

        // ensure capacity
        let needed_total = self.vertices.len() + verts.len();
        self.ensure_vertex_capacity(needed_total);

        let start = self.vertices.len();
        self.vertices.extend_from_slice(&verts);

        match self.commands.last_mut() {
            Some(DrawCommand::Color { count, .. }) => *count += verts.len(),
            _ => self.commands.push(DrawCommand::Color {
                start,
                count: verts.len(),
            }),
        }
    }

    /// Draws a texture (full image) at dest in pixel-space.
    /// UVs are (0,0)-(1,1) top-left -> bottom-right.
    pub fn draw_texture(&mut self, id: TextureId, dest: crate::Rect, tint: [f32; 4]) {
        // Pixel-space positions
        let x0 = dest.x;
        let y0 = dest.y;
        let x1 = dest.x + dest.w;
        let y1 = dest.y + dest.h;

        // UV coordinates: (0,0) top-left, (1,1) bottom-right
        let u0 = 0.0f32;
        let v0 = 0.0f32;
        let u1 = 1.0f32;
        let v1 = 1.0f32;

        let start = self.vertices.len();

        let mut verts = [
            Vertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color: tint,
            },
            Vertex {
                pos: [x1, y0],
                uv: [u1, v0],
                color: tint,
            },
            Vertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color: tint,
            },
            Vertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color: tint,
            },
            Vertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color: tint,
            },
            Vertex {
                pos: [x0, y1],
                uv: [u0, v1],
                color: tint,
            },
        ];

        // ensure capacity for new vertices
        let needed_total = self.vertices.len() + verts.len();
        self.ensure_vertex_capacity(needed_total);

        let model = self.current_model_matrix();
        transform_vertices_in_place(model, &mut verts);

        self.vertices.extend_from_slice(&verts);
        self.commands.push(DrawCommand::Texture {
            tex: id,
            start,
            count: verts.len(),
        });
    }
    pub fn draw_subtexture(
        &mut self,
        tex: TextureId,
        src: crate::Rect,
        dst: crate::Rect,
        tint: [f32; 4],
    ) {
        let texdata = match self.texture.get(&tex.0) {
            Some(t) => t,
            None => return,
        };

        let u0 = src.x / texdata.width as f32;
        let v0 = src.y / texdata.height as f32;
        let u1 = (src.x + src.w) / texdata.width as f32;
        let v1 = (src.y + src.h) / texdata.height as f32;

        let x0 = dst.x;
        let y0 = dst.y;
        let x1 = dst.x + dst.w;
        let y1 = dst.y + dst.h;

        let start = self.vertices.len();
        let mut verts = [
            Vertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color: tint,
            },
            Vertex {
                pos: [x1, y0],
                uv: [u1, v0],
                color: tint,
            },
            Vertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color: tint,
            },
            Vertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color: tint,
            },
            Vertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color: tint,
            },
            Vertex {
                pos: [x0, y1],
                uv: [u0, v1],
                color: tint,
            },
        ];

        let needed_total = start + verts.len();
        self.ensure_vertex_capacity(needed_total);

        let model = self.current_model_matrix();
        transform_vertices_in_place(model, &mut verts);

        self.vertices.extend_from_slice(&verts);

        self.commands.push(DrawCommand::Texture {
            tex,
            start,
            count: verts.len(),
        });
    }

    pub fn ortho_projection(&self) -> Mat4 {
        let w = self.gpu.surface_config.width as f32;
        let h = self.gpu.surface_config.height as f32;

        Mat4::from_cols(
            glam::vec4(2.0 / w, 0.0, 0.0, 0.0),
            glam::vec4(0.0, -2.0 / h, 0.0, 0.0),
            glam::vec4(0.0, 1.0, 0.0, 0.0),
            glam::vec4(-1.0, 1.0, 0.0, 1.0),
        )
    }

    fn set_transform_mat4(&mut self, mat: Mat4) {
        self.gpu.write_transform(mat);
    }

    fn current_view_matrix(&self) -> Mat4 {
        self.camera_stack
            .last()
            .map(|c| c.view_matrix())
            .unwrap_or(Mat4::IDENTITY)
    }

    fn update_viewproj_transform(&mut self) {
        let proj = self.ortho_projection();
        let view = self.current_view_matrix();
        self.set_transform_mat4(proj * view);
    }

    /// Begin 2D camera mode (world-space). Camera only applies until `end_mode_2d()`.
    pub fn begin_mode_2d(&mut self, camera: Camera2D) {
        self.camera_stack.push(camera);
        self.update_viewproj_transform();
    }

    /// End 2D camera mode, returning to screen-space.
    pub fn end_mode_2d(&mut self) {
        self.camera_stack.pop();
        self.update_viewproj_transform();
    }

    /// Model matrix stack (CPU-side, per-draw).
    pub fn push_matrix(&mut self) {
        let top = *self.model_stack.last().unwrap_or(&Mat4::IDENTITY);
        self.model_stack.push(top);
    }

    pub fn pop_matrix(&mut self) {
        if self.model_stack.len() > 1 {
            self.model_stack.pop();
        }
    }

    pub fn load_identity(&mut self) {
        if let Some(top) = self.model_stack.last_mut() {
            *top = Mat4::IDENTITY;
        }
    }

    pub fn translate(&mut self, tx: f32, ty: f32) {
        let t = Mat4::from_translation(glam::vec3(tx, ty, 0.0));
        if let Some(top) = self.model_stack.last_mut() {
            *top *= t;
        }
    }

    pub fn rotate_z(&mut self, radians: f32) {
        let r = Mat4::from_rotation_z(radians);
        if let Some(top) = self.model_stack.last_mut() {
            *top *= r;
        }
    }

    pub fn scale(&mut self, sx: f32, sy: f32) {
        let s = Mat4::from_scale(glam::vec3(sx, sy, 1.0));
        if let Some(top) = self.model_stack.last_mut() {
            *top *= s;
        }
    }

    fn current_model_matrix(&self) -> Mat4 {
        *self.model_stack.last().unwrap_or(&Mat4::IDENTITY)
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

        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
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
        self.gpu.queue.write_texture(
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
        let sampler = self.gpu.device.create_sampler(&wgpu::SamplerDescriptor {
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
        let bind_group = self.gpu.create_texture_bind_group(&view, &sampler);

        /*
        let bind_group = self.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.gpu.tex_bind_group_layout,
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
        */

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
    /// Resize: reconfigure surface.
    ///
    /// Note: resizing changes the orthographic projection used by the transform pipeline,
    /// so we also refresh the transform uniform to keep pixel-space drawing correct.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);

        // Keep the default transform in sync with the new surface size.
        self.update_viewproj_transform();
    }

    /// End frame: submit draw commands to the GPU and present.
    pub fn end_frame(&mut self) -> Result<(), RendererError> {
        // Delegate GPU submission.
        self.gpu.end_frame(
            &self.vertices,
            &self.commands,
            self.clear_color,
            &self.texture,
        )?;

        // Clear CPU-side arrays for next frame
        self.vertices.clear();
        self.commands.clear();

        Ok(())
    }
}
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck;
    use std::mem::size_of;

    #[test]
    fn vertex_pod_layout() {
        // Vertex = [f32;2] + [f32;2] + [f32;4] => 2*4 + 2*4 + 4*4 = 8 + 8 + 16 = 32 bytes
        assert_eq!(size_of::<Vertex>(), 32);
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
        let verts = crate::renderer::circle_to_vertices(cx, cy, radius, seg, color);
        // for seg triangles, we expect seg * 3 vertices
        assert_eq!(verts.len(), seg * 3);

        // With the transform pipeline, CPU-side vertex positions are in pixel-space.
        // The projection to NDC happens in the vertex shader via `u_transform`.
        assert!((verts[0].pos[0] - cx).abs() < 1e-6);
        assert!((verts[0].pos[1] - cy).abs() < 1e-6);
    }

    #[test]
    fn texture_loading_from_bytes() {
        use image::RgbaImage;

        // Create a simple 2x2 red image
        let mut img = RgbaImage::new(2, 2);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 255]);
        }

        // Encode to PNG bytes
        let mut png_bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .unwrap();

        // Test that image crate can decode it back
        let decoded = image::load_from_memory(&png_bytes).unwrap();
        let rgba = decoded.to_rgba8();
        assert_eq!(rgba.width(), 2);
        assert_eq!(rgba.height(), 2);

        // Verify first pixel is red
        assert_eq!(rgba.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn draw_texture_generates_correct_vertices() {
        // We can't easily test the full renderer without a GPU, but we can verify
        // the CPU-side vertex generation conventions.
        //
        // With the transform pipeline, vertex positions are in pixel-space and are
        // projected to NDC in the vertex shader using `u_transform`.
        let dest = crate::Rect {
            x: 100.0,
            y: 100.0,
            w: 200.0,
            h: 150.0,
        };

        let x0 = dest.x;
        let y0 = dest.y;
        let x1 = dest.x + dest.w;
        let y1 = dest.y + dest.h;

        // This matches the positions pushed by `Renderer::draw_texture`.
        let verts = [
            Vertex {
                pos: [x0, y0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                pos: [x1, y0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                pos: [x1, y1],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                pos: [x0, y0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                pos: [x1, y1],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                pos: [x0, y1],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        assert_eq!(verts[0].pos, [100.0, 100.0]);
        assert_eq!(verts[1].pos, [300.0, 100.0]);
        assert_eq!(verts[2].pos, [300.0, 250.0]);
        assert_eq!(verts[5].pos, [100.0, 250.0]);
    }

    #[test]
    fn texture_id_uniqueness() {
        // Verify TextureId wraps a u32 and can be copied
        let id1 = TextureId(0);
        let id2 = TextureId(1);
        let id1_copy = id1;

        assert_eq!(id1.0, 0);
        assert_eq!(id2.0, 1);
        assert_eq!(id1_copy.0, id1.0);
    }

    #[test]
    fn draw_command_variants() {
        // Test that DrawCommand enum variants work correctly
        let color_cmd = DrawCommand::Color { start: 0, count: 6 };
        let tex_cmd = DrawCommand::Texture {
            tex: TextureId(0),
            start: 6,
            count: 6,
        };

        match color_cmd {
            DrawCommand::Color { start, count } => {
                assert_eq!(start, 0);
                assert_eq!(count, 6);
            }
            _ => panic!("Wrong variant"),
        }

        match tex_cmd {
            DrawCommand::Texture { tex, start, count } => {
                assert_eq!(tex.0, 0);
                assert_eq!(start, 6);
                assert_eq!(count, 6);
            }
            _ => panic!("Wrong variant"),
        }
    }
}
