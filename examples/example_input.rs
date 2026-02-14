use libforge::{Color, Key, LibContext, MouseButton, Rect};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    ctx: Option<LibContext<Arc<Window>>>,

    // simple player state (world/screen space in pixels)
    player_pos: (f32, f32),
    player_speed: f32,

    // toggles
    show_help: bool,
}

impl App {
    fn ensure_init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("libforge - input")
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 600.0));

        let window = Arc::new(event_loop.create_window(attrs).expect("create_window"));
        let ctx = LibContext::new_from_window(window.clone()).expect("LibContext::new_from_window");

        self.window = Some(window);
        self.ctx = Some(ctx);
        self.player_pos = (450.0, 300.0);
        self.player_speed = 280.0;
        self.show_help = true;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.ensure_init(event_loop);
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.ensure_init(event_loop);

        let Some(ctx) = &mut self.ctx else {
            return;
        };

        // Feed all window events to libforge input state.
        ctx.handle_window_event(&event);

        match &event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => ctx.resize(size.width, size.height),
            WindowEvent::KeyboardInput { .. } => {
                //  react to edge events in your update loop, not here
            }
            WindowEvent::RedrawRequested => {
                // --- Begin frame ---
                ctx.begin_drawing();

                // Toggle UI with a key press edge.
                if ctx.is_key_pressed(Key::Space) {
                    self.show_help = !self.show_help;
                }

                // Quit with escape.
                if ctx.is_key_pressed(Key::Escape) {
                    event_loop.exit();
                    return;
                }

                // Mouse click: teleport player.
                if ctx.is_mouse_button_pressed(MouseButton::Left) {
                    self.player_pos = ctx.mouse_position();
                }

                // Movement (smooth, dt-based).
                let dt = ctx.frame_time();
                let mut dx: f32 = 0.0;
                let mut dy: f32 = 0.0;

                if ctx.is_key_down(Key::Left) || ctx.is_key_down(Key::A) {
                    dx -= 1.0;
                }
                if ctx.is_key_down(Key::Right) || ctx.is_key_down(Key::D) {
                    dx += 1.0;
                }
                if ctx.is_key_down(Key::Up) || ctx.is_key_down(Key::W) {
                    dy -= 1.0;
                }
                if ctx.is_key_down(Key::Down) || ctx.is_key_down(Key::S) {
                    dy += 1.0;
                }

                // Normalize to keep diagonal speed consistent.
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    dx /= len;
                    dy /= len;
                }

                self.player_pos.0 += dx * self.player_speed * dt;
                self.player_pos.1 += dy * self.player_speed * dt;

                // Scroll wheel: adjust speed.
                let (_wx, wy) = ctx.mouse_wheel();
                if wy != 0.0 {
                    self.player_speed = (self.player_speed + wy * 30.0).clamp(60.0, 900.0);
                }

                // --- Draw ---
                ctx.clear_background(Color::BLACK);

                // Crosshair at mouse
                let (mx, my) = ctx.mouse_position();
                ctx.draw_line(
                    mx - 10.0,
                    my,
                    mx + 10.0,
                    my,
                    2.0,
                    Color([1.0, 1.0, 1.0, 0.7]),
                );
                ctx.draw_line(
                    mx,
                    my - 10.0,
                    mx,
                    my + 10.0,
                    2.0,
                    Color([1.0, 1.0, 1.0, 0.7]),
                );

                // Player
                ctx.draw_rect(
                    Rect {
                        x: self.player_pos.0 - 20.0,
                        y: self.player_pos.1 - 20.0,
                        w: 40.0,
                        h: 40.0,
                    },
                    Color([0.2, 0.8, 0.4, 1.0]),
                );

                // HUD panels
                if self.show_help {
                    ctx.draw_rect(
                        Rect {
                            x: 20.0,
                            y: 20.0,
                            w: 380.0,
                            h: 120.0,
                        },
                        Color([0.1, 0.1, 0.1, 0.75]),
                    );

                    // Note: until text rendering exists, we just draw visual hints.
                    ctx.draw_rect(
                        Rect {
                            x: 30.0,
                            y: 40.0,
                            w: 10.0,
                            h: 10.0,
                        },
                        Color([0.9, 0.9, 0.2, 1.0]),
                    );

                    // Speed bar
                    let speed_norm = (self.player_speed / 900.0).clamp(0.0, 1.0);
                    ctx.draw_rect(
                        Rect {
                            x: 30.0,
                            y: 80.0,
                            w: 360.0,
                            h: 14.0,
                        },
                        Color([0.2, 0.2, 0.2, 1.0]),
                    );
                    ctx.draw_rect(
                        Rect {
                            x: 30.0,
                            y: 80.0,
                            w: 360.0 * speed_norm,
                            h: 14.0,
                        },
                        Color([0.2, 0.6, 0.9, 1.0]),
                    );
                }

                ctx.end_frame().expect("end_frame");
                ctx.end_drawing().expect("end_drawing");

                // Continue rendering; this is the "game loop".
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
