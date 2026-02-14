use libforge::{Color, LibContext, Rect, TextureId};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    ctx: Option<LibContext<Arc<Window>>>,
    texture: Option<TextureId>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - simple_texture")
            .with_inner_size(PhysicalSize::new(1024, 768));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        // Create context and load texture once
        let mut ctx = LibContext::new_from_window(window.clone()).unwrap();
        let bytes = include_bytes!("tennis-clay-court.png");
        let tex = ctx
            .load_texture_from_bytes("tennis_court", bytes)
            .expect("Failed to load texture");

        self.texture = Some(tex);
        self.ctx = Some(ctx);

        // Request initial redraw
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(ctx) = &mut self.ctx {
                    ctx.resize(size.width, size.height);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ctx) = &mut self.ctx
                    && let Some(tex) = self.texture {
                        ctx.clear_background(Color([0.1, 0.1, 0.15, 1.0]));

                        // Draw the texture
                        ctx.draw_texture(
                            tex,
                            Rect {
                                x: 100.0,
                                y: 100.0,
                                w: 800.0,
                                h: 533.0, // Maintain 1536:1024 aspect ratio (3:2)
                            },
                            Color([1.0, 1.0, 1.0, 1.0]), // white = no tint
                        );

                        ctx.end_frame().expect("end_frame failed");
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None,
        ctx: None,
        texture: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
