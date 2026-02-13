pub mod camera;
pub mod error;
pub mod renderer;
pub mod sprite_animation;
pub mod vertex;

pub use crate::camera::Camera2D;
pub use crate::renderer::TextureId;

use crate::sprite_animation::SpriteAnimation;
use error::LibforgeError;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use renderer::Renderer;

/// Simple RGBA color
#[derive(Clone, Copy, Debug)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Color = Color([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Color = Color([0.0, 0.0, 0.0, 1.0]);
}

/// Rectangle in logical pixels
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Public immediate-mode context
pub struct LibContext<W> {
    renderer: Renderer<W>,
}

impl<W> LibContext<W>
where
    W: HasWindowHandle + HasDisplayHandle + wgpu::WasmNotSendSync + Sync + Clone + 'static,
{
    /// Create a new `LibContext` from any window type that can provide raw window + display handles.
    ///
    /// In examples, this is typically a `winit::window::Window` wrapped in an `Arc`.
    pub fn new_from_window(window: W) -> Result<Self, LibforgeError> {
        let renderer = pollster::block_on(Renderer::new(window))?;
        Ok(LibContext { renderer })
    }

    /// Must be called at the start of each frame. Optional clear color.
    pub fn begin_frame(&mut self, clear: Option<Color>) {
        self.renderer.begin_frame(clear.map(|c| c.0));
    }

    /// Immediate draw a filled rectangle (in logical pixels)
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.renderer.draw_rect(rect, color);
    }

    /// Immediate draw a line
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        self.renderer.draw_line(x1, y1, x2, y2, thickness, color.0);
    }

    /// Draw a filled circle centered at (x, y) with given radius (in logical pixels).
    /// `segments` controls the tessellation (higher = smoother). Use ~32 for good quality.
    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, segments: usize, color: Color) {
        self.renderer.draw_circle(x, y, radius, segments, color.0);
    }

    pub fn draw_texture(&mut self, tex: TextureId, rect: Rect, tint: Color) {
        self.renderer.draw_texture(tex, rect, tint.0);
    }

    pub fn draw_subtexture(&mut self, tex: TextureId, src: Rect, dst: Rect, tint: Color) {
        self.renderer.draw_subtexture(tex, src, dst, tint.0);
    }

    pub fn draw_sprite_animation(
        &mut self,
        tex: TextureId,
        animation: &SpriteAnimation,
        time: f32,
        destination: Rect,
        tint: Color,
    ) {
        let src = animation.frame_at_time(time);
        self.renderer.draw_subtexture(tex, src, destination, tint.0);
    }

    pub fn load_texture_from_bytes(
        &mut self,
        name: &str,
        bytes: &[u8],
    ) -> Result<TextureId, LibforgeError> {
        Ok(self.renderer.load_texture_from_bytes(name, bytes)?)
    }

    // -------------------------------------------------------------------------
    //
    // Default drawing is in screen-space (pixels). To draw in world-space, enter
    // 2D camera mode with `begin_mode_2d()` and exit with `end_mode_2d()`.
    //
    // Per-draw transforms are handled via a simple matrix stack.
    // -------------------------------------------------------------------------

    /// Begin 2D camera mode (world-space). Camera affects subsequent draws until `end_mode_2d()`.
    pub fn begin_mode_2d(&mut self, camera: Camera2D) {
        self.renderer.begin_mode_2d(camera);
    }

    /// End 2D camera mode and return to screen-space drawing.
    pub fn end_mode_2d(&mut self) {
        self.renderer.end_mode_2d();
    }

    /// Push the current model transform.
    pub fn push_matrix(&mut self) {
        self.renderer.push_matrix();
    }

    /// Pop the current model transform.
    pub fn pop_matrix(&mut self) {
        self.renderer.pop_matrix();
    }

    /// Reset the current model transform to identity.
    pub fn load_identity(&mut self) {
        self.renderer.load_identity();
    }

    /// Apply a translation to the current model transform.
    pub fn translate(&mut self, tx: f32, ty: f32) {
        self.renderer.translate(tx, ty);
    }

    /// Apply a rotation (around Z) to the current model transform.
    pub fn rotate_z(&mut self, radians: f32) {
        self.renderer.rotate_z(radians);
    }

    /// Apply a scale to the current model transform.
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.renderer.scale(sx, sy);
    }

    /// Finish the frame, flush commands to GPU, and present.
    pub fn end_frame(&mut self) -> Result<(), LibforgeError> {
        self.renderer.end_frame()?;
        Ok(())
    }

    /// Handle window resize: pass the new logical size in pixels.
    ///
    /// Resizing updates the internal screen-space projection and any active camera mode.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }
}
