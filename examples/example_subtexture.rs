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
    sheet: Option<TextureId>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("libforge - subtextures")
                        .with_inner_size(PhysicalSize::new(800, 600)),
                )
                .unwrap(),
        );
        self.window = Some(window.clone());

        let mut ctx = LibContext::new_from_window(window.clone()).unwrap();
        // load sprite sheet
        let bytes = include_bytes!("tennis-player-1.png");
        let tex = ctx.load_texture_from_bytes("sheet", bytes).unwrap();
        self.sheet = Some(tex);

        window.request_redraw();
        self.ctx = Some(ctx);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(ctx) = &mut self.ctx {
                    let tex = self.sheet.unwrap();

                    ctx.begin_frame(Some(Color([0.15, 0.15, 0.2, 1.0])));

                    // Draw the full texture first (scaled down) to verify it loads
                    ctx.draw_texture(
                        tex,
                        Rect {
                            x: 500.0,
                            y: 50.0,
                            w: 263.0,
                            h: 339.0, // half size, maintains 526:678 ratio
                        },
                        Color([1.0, 1.0, 1.0, 0.3]), // semi-transparent for reference
                    );

                    // Draw subtexture - top-left corner of the sprite sheet
                    ctx.draw_subtexture(
                        tex,
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            w: 64.0,
                            h: 64.0,
                        }, // src in texture (top-left 64x64 pixels)
                        Rect {
                            x: 100.0,
                            y: 100.0,
                            w: 128.0,
                            h: 128.0,
                        }, // dest on screen (2x scale)
                        Color::WHITE,
                    );

                    // Draw another subtexture from a different part
                    ctx.draw_subtexture(
                        tex,
                        Rect {
                            x: 100.0,
                            y: 100.0,
                            w: 100.0,
                            h: 100.0,
                        },
                        Rect {
                            x: 250.0,
                            y: 100.0,
                            w: 150.0,
                            h: 150.0,
                        },
                        Color([1.0, 0.7, 0.7, 1.0]), // slight red tint
                    );

                    // Draw a third subtexture
                    ctx.draw_subtexture(
                        tex,
                        Rect {
                            x: 200.0,
                            y: 200.0,
                            w: 80.0,
                            h: 80.0,
                        },
                        Rect {
                            x: 100.0,
                            y: 300.0,
                            w: 120.0,
                            h: 120.0,
                        },
                        Color([0.7, 0.7, 1.0, 1.0]), // slight blue tint
                    );

                    ctx.end_frame().unwrap();
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
        sheet: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
