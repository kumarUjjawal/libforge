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
    time: f32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - texture_tint")
            .with_inner_size(PhysicalSize::new(900, 900));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        let mut ctx = LibContext::new_from_window(window).unwrap();
        // Initialize the transform pipeline to pixel-space orthographic projection.
        ctx.reset_transform();

        // Load texture once at startup
        let bytes = include_bytes!("tennis-clay-court.png");
        let tex = ctx
            .load_texture_from_bytes("tennis_court", bytes)
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
                        ctx.begin_frame(Some(Color([0.05, 0.05, 0.1, 1.0])));

                        // Draw the same texture 9 times in a 3x3 grid with different tints
                        let size = 280.0;
                        let spacing = 10.0;

                        // Animate the tints over time
                        self.time += 0.016; // ~60 FPS
                        let pulse = (self.time * 1.5).sin() * 0.5 + 0.5;

                        let tints = [
                            // Row 1: Red variations
                            Color([1.0, 0.3 * pulse, 0.3 * pulse, 1.0]),
                            Color([1.0, 0.5, 0.5, 1.0]),
                            Color([1.0 * pulse, 0.2, 0.2, 1.0]),
                            // Row 2: Green variations
                            Color([0.3 * pulse, 1.0, 0.3 * pulse, 1.0]),
                            Color([1.0, 1.0, 1.0, 1.0]), // original (center)
                            Color([0.2, 1.0 * pulse, 0.2, 1.0]),
                            // Row 3: Blue variations
                            Color([0.3 * pulse, 0.3 * pulse, 1.0, 1.0]),
                            Color([0.5, 0.5, 1.0, 1.0]),
                            Color([0.2, 0.2, 1.0 * pulse, 1.0]),
                        ];

                        for row in 0..3 {
                            for col in 0..3 {
                                let x = spacing + col as f32 * (size + spacing);
                                let y = spacing + row as f32 * (size + spacing);
                                let idx = row * 3 + col;

                                ctx.draw_texture(
                                    tex,
                                    Rect {
                                        x,
                                        y,
                                        w: size,
                                        h: size,
                                    },
                                    tints[idx],
                                );
                            }
                        }

                        ctx.end_frame().expect("end_frame failed");

                        // Request continuous redraw for animation
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
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
        time: 0.0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
