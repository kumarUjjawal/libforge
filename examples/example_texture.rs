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
            .with_title("libforge - hello_texture")
            .with_inner_size(PhysicalSize::new(1024, 768));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        
        let mut ctx = LibContext::new_from_window(window).unwrap();
        // Initialize the transform pipeline to pixel-space orthographic projection.
        ctx.reset_transform();
        
        // Load texture once at startup
        let bytes = include_bytes!("tennis-clay-court.png");
        let tex = ctx.load_texture_from_bytes("tennis_court", bytes)
            .expect("Failed to load texture");
        
        self.texture = Some(tex);
        self.ctx = Some(ctx);
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
                if let Some(ctx) = &mut self.ctx {
                    if let Some(tex) = self.texture {
                        ctx.begin_frame(Some(Color([0.1, 0.1, 0.15, 1.0])));

                        // Draw texture filling most of the window
                        ctx.draw_texture(
                            tex,
                            Rect {
                                x: 50.0,
                                y: 50.0,
                                w: 924.0,
                                h: 668.0,
                            },
                            Color([1.0, 1.0, 1.0, 1.0]), // white tint (no modification)
                        );

                        ctx.end_frame().expect("end_frame failed");
                    }
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
