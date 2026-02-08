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
    rotation: f32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - mixed_rendering")
            .with_inner_size(PhysicalSize::new(1000, 700));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        let mut ctx = LibContext::new_from_window(window).unwrap();

        // Load texture
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
                        ctx.begin_frame(Some(Color([0.12, 0.12, 0.18, 1.0])));

                        // Draw textured background
                        ctx.draw_texture(
                            tex,
                            Rect {
                                x: 50.0,
                                y: 50.0,
                                w: 400.0,
                                h: 300.0,
                            },
                            Color([0.8, 0.8, 0.8, 1.0]),
                        );

                        // Draw semi-transparent rect overlay
                        ctx.draw_rect(
                            Rect {
                                x: 150.0,
                                y: 100.0,
                                w: 200.0,
                                h: 200.0,
                            },
                            Color([0.2, 0.6, 1.0, 0.5]),
                        );

                        // Draw animated circles
                        self.rotation += 0.02;
                        let center_x = 700.0;
                        let center_y = 200.0;
                        let orbit_radius = 100.0;

                        for i in 0..6 {
                            let angle = self.rotation + (i as f32) * std::f32::consts::PI / 3.0;
                            let x = center_x + angle.cos() * orbit_radius;
                            let y = center_y + angle.sin() * orbit_radius;
                            let hue = (i as f32) / 6.0;
                            let color = Color([
                                (hue * 2.0 * std::f32::consts::PI).sin() * 0.5 + 0.5,
                                (hue * 2.0 * std::f32::consts::PI + 2.0).sin() * 0.5 + 0.5,
                                (hue * 2.0 * std::f32::consts::PI + 4.0).sin() * 0.5 + 0.5,
                                1.0,
                            ]);
                            ctx.draw_circle(x, y, 20.0, 24, color);
                        }

                        // Draw connecting lines
                        ctx.draw_line(
                            250.0,
                            400.0,
                            750.0,
                            400.0,
                            5.0,
                            Color([1.0, 0.8, 0.2, 1.0]),
                        );
                        ctx.draw_line(
                            250.0,
                            400.0,
                            250.0,
                            600.0,
                            3.0,
                            Color([1.0, 0.2, 0.8, 1.0]),
                        );
                        ctx.draw_line(
                            750.0,
                            400.0,
                            750.0,
                            600.0,
                            3.0,
                            Color([0.2, 1.0, 0.8, 1.0]),
                        );

                        // Draw small texture in corner
                        ctx.draw_texture(
                            tex,
                            Rect {
                                x: 550.0,
                                y: 450.0,
                                w: 150.0,
                                h: 150.0,
                            },
                            Color([1.0, 0.6, 0.3, 0.8]),
                        );

                        // Draw decorative rects
                        for i in 0..5 {
                            let x = 50.0 + (i as f32) * 180.0;
                            let height = 50.0 + (self.rotation + i as f32).sin() * 30.0;
                            ctx.draw_rect(
                                Rect {
                                    x,
                                    y: 650.0 - height,
                                    w: 150.0,
                                    h: height,
                                },
                                Color([
                                    0.3 + (i as f32) * 0.15,
                                    0.5,
                                    0.8 - (i as f32) * 0.1,
                                    0.8,
                                ]),
                            );
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
        rotation: 0.0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
