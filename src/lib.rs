pub mod renderer;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use renderer::Renderer;
use thiserror::Error;

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

/// Public error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("wgpu error: {0}")]
    Wgpu(String),

    #[error("platform error: {0}")]
    Platform(String),
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
    pub fn new_from_window(window: W) -> Result<Self, Error> {
        let renderer = pollster::block_on(Renderer::new(window))
            .map_err(|e| Error::Wgpu(format!("{:?}", e)))?;
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

    /// Finish the frame, flush commands to GPU, and present.
    pub fn end_frame(&mut self) -> Result<(), Error> {
        self.renderer
            .end_frame()
            .map_err(|e| Error::Wgpu(format!("{:?}", e)))
    }

    /// Handle window resize: pass the new logical size in pixels
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }
}
