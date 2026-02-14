use libforge::{Camera2D, Color, LibContext, Rect};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},

    window::{Window, WindowId},
};

/// Demonstrates the 2D camera (pan/zoom/rotation) using the transform pipeline.
///
/// Controls:
/// - Arrow keys: pan camera
/// - Q / E: rotate camera
/// - +/-: zoom in/out
struct App {
    window: Option<Arc<Window>>,
    ctx: Option<LibContext<Arc<Window>>>,
    camera: Camera2D,

    // input state
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    rot_left: bool,
    rot_right: bool,
    zoom_in: bool,
    zoom_out: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - example_camera")
            .with_inner_size(PhysicalSize::new(900, 600));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        let ctx = LibContext::new_from_window(window).unwrap();
        self.ctx = Some(ctx);

        // Kick the first frame.
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ctx) = &mut self.ctx {
                    ctx.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let is_down = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::ArrowLeft) => self.left = is_down,
                    PhysicalKey::Code(KeyCode::ArrowRight) => self.right = is_down,
                    PhysicalKey::Code(KeyCode::ArrowUp) => self.up = is_down,
                    PhysicalKey::Code(KeyCode::ArrowDown) => self.down = is_down,
                    PhysicalKey::Code(KeyCode::KeyQ) => self.rot_left = is_down,
                    PhysicalKey::Code(KeyCode::KeyE) => self.rot_right = is_down,
                    PhysicalKey::Code(KeyCode::Equal) => self.zoom_in = is_down, // '+' without shift on many layouts
                    PhysicalKey::Code(KeyCode::NumpadAdd) => self.zoom_in = is_down,
                    PhysicalKey::Code(KeyCode::Minus) => self.zoom_out = is_down,
                    PhysicalKey::Code(KeyCode::NumpadSubtract) => self.zoom_out = is_down,
                    PhysicalKey::Code(KeyCode::Escape) if is_down => event_loop.exit(),
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(ctx) = &mut self.ctx else { return };

                // Update camera (simple fixed-step).
                let pan_speed = 6.0;
                if self.left {
                    self.camera.x -= pan_speed;
                }
                if self.right {
                    self.camera.x += pan_speed;
                }
                if self.up {
                    self.camera.y -= pan_speed;
                }
                if self.down {
                    self.camera.y += pan_speed;
                }

                let rot_speed = 0.02;
                if self.rot_left {
                    self.camera.rotation -= rot_speed;
                }
                if self.rot_right {
                    self.camera.rotation += rot_speed;
                }

                let zoom_speed = 0.02;
                if self.zoom_in {
                    self.camera.zoom = (self.camera.zoom - zoom_speed).max(0.05);
                }
                if self.zoom_out {
                    self.camera.zoom += zoom_speed;
                }

                // Draw
                ctx.clear_background(Color([0.06, 0.06, 0.08, 1.0]));

                // World-space drawing under a scoped 2D camera mode.
                ctx.begin_mode_2d(self.camera);

                // A simple grid + some reference shapes in world space.
                let grid_color = Color([0.25, 0.25, 0.3, 1.0]);
                for i in (-50..=50).step_by(5) {
                    let x = i as f32 * 50.0;
                    ctx.draw_line(x, -2500.0, x, 2500.0, 1.0, grid_color);
                    let y = i as f32 * 50.0;
                    ctx.draw_line(-2500.0, y, 2500.0, y, 1.0, grid_color);
                }

                // axis lines
                ctx.draw_line(-2500.0, 0.0, 2500.0, 0.0, 3.0, Color([0.9, 0.2, 0.2, 1.0]));
                ctx.draw_line(0.0, -2500.0, 0.0, 2500.0, 3.0, Color([0.2, 0.9, 0.2, 1.0]));

                // some rectangles/circles at different world coordinates
                ctx.draw_rect(
                    Rect {
                        x: -200.0,
                        y: -120.0,
                        w: 400.0,
                        h: 240.0,
                    },
                    Color([0.2, 0.6, 1.0, 0.35]),
                );
                ctx.draw_circle(300.0, 200.0, 40.0, 32, Color([1.0, 0.85, 0.2, 1.0]));
                ctx.draw_circle(-450.0, -300.0, 60.0, 32, Color([0.9, 0.3, 0.9, 1.0]));

                ctx.end_mode_2d();

                ctx.end_frame().unwrap();

                // Keep animating while a key is held (otherwise some platforms won't
                // generate repeat key events, so the camera would "not move").
                let any_input = self.left
                    || self.right
                    || self.up
                    || self.down
                    || self.rot_left
                    || self.rot_right
                    || self.zoom_in
                    || self.zoom_out;

                if any_input
                    && let Some(window) = &self.window
                {
                    window.request_redraw();
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
        camera: Camera2D::new(),
        left: false,
        right: false,
        up: false,
        down: false,
        rot_left: false,
        rot_right: false,
        zoom_in: false,
        zoom_out: false,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}
