use libforge::sprite_animation::SpriteAnimation;
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
    anim: Option<SpriteAnimation>,
    time: f32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("libforge - sprite animation")
            .with_inner_size(PhysicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        let mut ctx = LibContext::new_from_window(window.clone()).unwrap();
        // Load the spritesheet
        let bytes = include_bytes!("tennis-player-2.png");
        let tex = ctx.load_texture_from_bytes("sprite_sheet", bytes).unwrap();
        self.texture = Some(tex);

        // This example assumes a sheet of 6 frames, 64x64 each:
        let mut frames = Vec::new();
        for i in 0..6 {
            frames.push(Rect {
                x: (i as f32) * 64.0,
                y: 0.0,
                w: 64.0,
                h: 64.0,
            });
        }

        self.anim = Some(SpriteAnimation { frames, fps: 12.0 });
        self.ctx = Some(ctx);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let (Some(ctx), Some(tex), Some(anim)) =
                    (&mut self.ctx, self.texture, &self.anim)
                {
                    // Increase time
                    self.time += 1.0 / 60.0;

                    ctx.clear_background(Color([0.12, 0.12, 0.15, 1.0]));

                    // Draw animated sprite
                    ctx.draw_sprite_animation(
                        tex,
                        anim,
                        self.time,
                        Rect {
                            x: 350.0,
                            y: 250.0,
                            w: 128.0,
                            h: 128.0,
                        },
                        Color::WHITE,
                    );

                    ctx.end_frame().unwrap();

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
        texture: None,
        anim: None,
        time: 0.0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
