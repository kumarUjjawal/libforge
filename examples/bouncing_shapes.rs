use libforge::{Color, LibContext, Rect};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    color: Color,
}

struct App {
    window: Option<Arc<Window>>,
    ctx: Option<LibContext<Arc<Window>>>,
    balls: Vec<Ball>,
    width: f32,
    height: f32,
}

impl App {
    fn update(&mut self, dt: f32) {
        for ball in &mut self.balls {
            // Update position
            ball.x += ball.vx * dt;
            ball.y += ball.vy * dt;

            // Bounce off walls
            if ball.x - ball.radius < 0.0 {
                ball.x = ball.radius;
                ball.vx = -ball.vx;
            }
            if ball.x + ball.radius > self.width {
                ball.x = self.width - ball.radius;
                ball.vx = -ball.vx;
            }
            if ball.y - ball.radius < 0.0 {
                ball.y = ball.radius;
                ball.vy = -ball.vy;
            }
            if ball.y + ball.radius > self.height {
                ball.y = self.height - ball.radius;
                ball.vy = -ball.vy;
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - bouncing_shapes")
            .with_inner_size(PhysicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        let mut ctx = LibContext::new_from_window(window).unwrap();
        // Initialize the transform pipeline to pixel-space orthographic projection.
        ctx.reset_transform();
        self.ctx = Some(ctx);

        // Initialize balls
        self.balls = vec![
            Ball {
                x: 200.0,
                y: 150.0,
                vx: 150.0,
                vy: 100.0,
                radius: 30.0,
                color: Color([1.0, 0.3, 0.3, 1.0]),
            },
            Ball {
                x: 400.0,
                y: 300.0,
                vx: -120.0,
                vy: 130.0,
                radius: 40.0,
                color: Color([0.3, 1.0, 0.3, 1.0]),
            },
            Ball {
                x: 600.0,
                y: 450.0,
                vx: 80.0,
                vy: -150.0,
                radius: 25.0,
                color: Color([0.3, 0.3, 1.0, 1.0]),
            },
            Ball {
                x: 300.0,
                y: 400.0,
                vx: -100.0,
                vy: -80.0,
                radius: 35.0,
                color: Color([1.0, 1.0, 0.3, 1.0]),
            },
            Ball {
                x: 500.0,
                y: 200.0,
                vx: 110.0,
                vy: 120.0,
                radius: 28.0,
                color: Color([1.0, 0.5, 0.8, 1.0]),
            },
        ];
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
                    self.width = size.width as f32;
                    self.height = size.height as f32;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Update physics first (before borrowing ctx)
                self.update(1.0 / 60.0); // ~60 FPS

                if let Some(ctx) = &mut self.ctx {
                    ctx.begin_frame(Some(Color([0.15, 0.15, 0.2, 1.0])));

                    // Draw border rectangle
                    ctx.draw_rect(
                        Rect {
                            x: 10.0,
                            y: 10.0,
                            w: self.width - 20.0,
                            h: 10.0,
                        },
                        Color([0.5, 0.5, 0.5, 1.0]),
                    );
                    ctx.draw_rect(
                        Rect {
                            x: 10.0,
                            y: self.height - 20.0,
                            w: self.width - 20.0,
                            h: 10.0,
                        },
                        Color([0.5, 0.5, 0.5, 1.0]),
                    );
                    ctx.draw_rect(
                        Rect {
                            x: 10.0,
                            y: 10.0,
                            w: 10.0,
                            h: self.height - 20.0,
                        },
                        Color([0.5, 0.5, 0.5, 1.0]),
                    );
                    ctx.draw_rect(
                        Rect {
                            x: self.width - 20.0,
                            y: 10.0,
                            w: 10.0,
                            h: self.height - 20.0,
                        },
                        Color([0.5, 0.5, 0.5, 1.0]),
                    );

                    // Draw all balls
                    for ball in &self.balls {
                        ctx.draw_circle(ball.x, ball.y, ball.radius, 32, ball.color);
                    }

                    // Draw connecting lines between balls
                    for i in 0..self.balls.len() {
                        for j in (i + 1)..self.balls.len() {
                            let b1 = &self.balls[i];
                            let b2 = &self.balls[j];
                            let dist = ((b2.x - b1.x).powi(2) + (b2.y - b1.y).powi(2)).sqrt();
                            if dist < 250.0 {
                                let alpha = 1.0 - (dist / 250.0);
                                ctx.draw_line(
                                    b1.x,
                                    b1.y,
                                    b2.x,
                                    b2.y,
                                    2.0,
                                    Color([0.8, 0.8, 0.8, alpha * 0.5]),
                                );
                            }
                        }
                    }

                    ctx.end_frame().expect("end_frame failed");

                    // Request continuous redraw for animation
                    if let Some(window) = &self.window {
                        window.request_redraw();
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
        balls: Vec::new(),
        width: 800.0,
        height: 600.0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
