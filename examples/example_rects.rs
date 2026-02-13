use libforge::{Color, LibContext, Rect};
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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - hello_rects")
            .with_inner_size(PhysicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        // Create the library context from the winit window.
        // Passing Arc<Window> results in a static surface lifetime.
        let mut ctx = LibContext::new_from_window(window).unwrap();
        // Initialize the transform pipeline to pixel-space orthographic projection.
        ctx.reset_transform();
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
                    // Resize changes the projection, so update the transform uniform.
                    ctx.reset_transform();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ctx) = &mut self.ctx {
                    // Immediate-mode usage
                    ctx.begin_frame(Some(Color([0.2, 0.25, 0.3, 1.0])));
                    // Draw some rectangles
                    ctx.draw_rect(
                        Rect {
                            x: 100.0,
                            y: 80.0,
                            w: 200.0,
                            h: 120.0,
                        },
                        Color([0.9, 0.2, 0.2, 1.0]),
                    );
                    ctx.draw_rect(
                        Rect {
                            x: 350.0,
                            y: 180.0,
                            w: 120.0,
                            h: 220.0,
                        },
                        Color([0.2, 0.9, 0.2, 1.0]),
                    );
                    ctx.draw_rect(
                        Rect {
                            x: 220.0,
                            y: 360.0,
                            w: 360.0,
                            h: 100.0,
                        },
                        Color([0.2, 0.4, 0.9, 1.0]),
                    );

                    ctx.draw_line(
                        50.0,
                        50.0,
                        350.0,
                        200.0,
                        4.0,
                        libforge::Color([1.0, 1.0, 0.0, 1.0]),
                    );
                    ctx.draw_line(
                        300.0,
                        300.0,
                        600.0,
                        300.0,
                        10.0,
                        libforge::Color([0.0, 0.0, 0.0, 1.0]),
                    );

                    ctx.draw_circle(
                        200.0,
                        150.0,
                        30.0,
                        24,
                        libforge::Color([1.0, 0.5, 0.2, 1.0]),
                    );
                    ctx.draw_circle(
                        400.0,
                        150.0,
                        50.0,
                        64,
                        libforge::Color([0.2, 0.6, 1.0, 1.0]),
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
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
