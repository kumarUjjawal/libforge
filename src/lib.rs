pub mod camera;
pub mod error;
mod input;
pub mod renderer;
pub mod sprite_animation;
pub mod vertex;

pub use crate::camera::Camera2D;
pub use crate::renderer::TextureId;
use crate::sprite_animation::SpriteAnimation;

use error::LibforgeError;
pub use input::{Key, MouseButton};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use renderer::Renderer;
use std::time::Instant;

/// RGBA color with values in the range `[0.0, 1.0]`.
///
/// Example: `Color([1.0, 0.0, 0.0, 1.0])` is opaque red.
#[derive(Clone, Copy, Debug)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Color = Color([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Color = Color([0.0, 0.0, 0.0, 1.0]);
}

/// Rectangle in logical pixels.
///
/// `(x, y)` is the top-left corner, `(w, h)` is the size.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The main context for immediate-mode rendering and input.
///
/// Create with `LibContext::new_from_window(window)`.
/// Each frame: `begin_drawing()`, draw calls, `end_drawing()`.
pub struct LibContext<W> {
    renderer: Renderer<W>,
    input: input::InputState,
    last_frame_instant: Instant,
    frame_dt: f32,
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
        Ok(LibContext {
            renderer,
            input: input::InputState::default(),
            last_frame_instant: Instant::now(),
            frame_dt: 1.0 / 60.0,
        })
    }

    /// Call once per frame before any draw calls
    pub fn begin_drawing(&mut self) {
        let now = Instant::now();
        self.frame_dt = (now - self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;

        self.input.begin_frame();

        // Start a frame with no implicit clear.
        self.renderer.begin_frame(None);
    }

    /// Finish the frame and present to the screen.
    ///
    /// This submits all draw commands to the GPU and displays the result.
    /// Call after all drawing is complete.
    pub fn end_drawing(&mut self) -> Result<(), crate::error::RendererError> {
        self.renderer.end_frame()
    }

    /// Time elapsed since the last frame (in seconds).
    ///
    /// Use this for smooth movement: `position += velocity * ctx.frame_time()`.
    pub fn frame_time(&self) -> f32 {
        self.frame_dt
    }

    /// Current frames per second.
    ///
    /// Computed as `1.0 / frame_time()`.
    pub fn fps(&self) -> f32 {
        if self.frame_dt > 0.0 {
            1.0 / self.frame_dt
        } else {
            0.0
        }
    }

    /// Feed winit window events into the input system.
    ///
    /// Call this from your event loop for each `WindowEvent`.
    /// The library tracks keyboard, mouse button, cursor position, and scroll wheel state.
    pub fn handle_window_event(&mut self, event: &winit::event::WindowEvent) {
        use winit::event::WindowEvent;

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.input
                    .handle_keyboard_input(event.physical_key, event.state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.handle_cursor_moved(*position);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.handle_mouse_button(*button, *state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.handle_mouse_wheel(*delta);
            }
            _ => {}
        }
    }

    /// Check if a key is currently held down.
    pub fn is_key_down(&self, key: Key) -> bool {
        self.input.is_key_down(key)
    }

    /// Check if a key was just pressed this frame (edge detection).
    ///
    /// Returns `true` only on the frame the key transitions from up to down.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.input.is_key_pressed(key)
    }

    /// Check if a mouse button is currently held down.
    pub fn is_mouse_button_down(&self, btn: MouseButton) -> bool {
        self.input.is_mouse_button_down(btn)
    }

    /// Check if a mouse button was just pressed this frame (edge detection).
    pub fn is_mouse_button_pressed(&self, btn: MouseButton) -> bool {
        self.input.is_mouse_button_pressed(btn)
    }

    /// Current mouse cursor position in screen pixels.
    ///
    /// Returns `(x, y)` where `(0, 0)` is the top-left corner.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.input.mouse_position()
    }

    /// Mouse wheel scroll delta for this frame.
    ///
    /// Returns `(horizontal, vertical)`. Positive vertical = scroll up.
    /// Resets to `(0, 0)` at the start of each frame.
    pub fn mouse_wheel(&self) -> (f32, f32) {
        self.input.mouse_wheel()
    }

    /// Clear the screen to a solid color. Call after `begin_drawing()` and before any draw calls.
    pub fn clear_background(&mut self, color: Color) {
        self.renderer.begin_frame(Some(color.0));
    }

    /// Must be called at the start of each frame. Optional clear color.
    /// 
    /// **Deprecated:** prefer `begin_drawing()` + `clear_background(color)` for clarity.
    #[deprecated(since = "0.1.0", note = "use `begin_drawing()` + `clear_background(color)` instead")]
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

    /// Draw a texture, scaled to fit the destination rectangle.
    ///
    /// The texture is tinted by multiplying with the `tint` color.
    /// Use `Color::WHITE` for no tint.
    pub fn draw_texture(&mut self, tex: TextureId, rect: Rect, tint: Color) {
        self.renderer.draw_texture(tex, rect, tint.0);
    }

    /// Draw a portion of a texture (subtexture/sprite).
    ///
    /// `src` defines the region in the source texture (in pixels).
    /// `dst` defines where to draw it on screen.
    pub fn draw_subtexture(&mut self, tex: TextureId, src: Rect, dst: Rect, tint: Color) {
        self.renderer.draw_subtexture(tex, src, dst, tint.0);
    }

    /// Draw an animated sprite by sampling the current frame from a sprite animation.
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

    /// Load a texture from PNG/JPEG bytes.
    ///
    /// Returns a `TextureId` that can be used with `draw_texture` and `draw_subtexture`.
    /// The `name` is for debugging only.
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
