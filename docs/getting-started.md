# Getting Started

## Installation

Add `libforge` to your `Cargo.toml`:

```toml
[dependencies]
libforge = "0.1"
winit = "0.30"
pollster = "0.3"
```

## Basic Example

```rust
use libforge::{LibContext, Color, Rect, Key};
use std::sync::Arc;
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;
use winit::event::{Event, WindowEvent};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = Arc::new(WindowBuilder::new()
        .with_title("My Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap());

    let mut ctx = LibContext::new_from_window(window.clone()).unwrap();

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                ctx.handle_window_event(&event);

                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => {
                        ctx.resize(size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        ctx.begin_drawing();
                        ctx.clear_background(Color::BLACK);

                        // Draw a rectangle
                        ctx.draw_rect(
                            Rect { x: 100.0, y: 100.0, w: 200.0, h: 150.0 },
                            Color([1.0, 0.0, 0.0, 1.0]) // red
                        );

                        ctx.end_drawing().unwrap();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }).unwrap();
}
```

## Frame Loop Pattern

Every frame follows this pattern:

1. `ctx.begin_drawing()` - starts the frame, advances input state
2. `ctx.clear_background(color)` - (optional) clear the screen
3. Draw calls - `draw_rect`, `draw_circle`, `draw_texture`, etc.
4. `ctx.end_drawing()` - submits to GPU and presents

## Coordinate System

By default, drawing uses screen-space coordinates:
- Origin `(0, 0)` is the top-left corner
- X increases to the right
- Y increases downward
- Units are logical pixels

## Input Handling

Check input state anywhere in your frame:

```rust
ctx.begin_drawing();

// Continuous movement
if ctx.is_key_down(Key::Right) {
    player_x += 200.0 * ctx.frame_time();
}

// One-time actions
if ctx.is_key_pressed(Key::Space) {
    jump();
}

// Mouse
let (mx, my) = ctx.mouse_position();
if ctx.is_mouse_button_pressed(MouseButton::Left) {
    click_at(mx, my);
}

ctx.end_drawing().unwrap();
```

## Loading Textures

```rust
let texture_bytes = include_bytes!("my_sprite.png");
let tex_id = ctx.load_texture_from_bytes("my_sprite", texture_bytes)?;

// Draw it
ctx.draw_texture(
    tex_id,
    Rect { x: 50.0, y: 50.0, w: 64.0, h: 64.0 },
    Color::WHITE
);
```

## Next Steps

- [Architecture](architecture.md) - how the rendering pipeline works
- [Input](input.md) - keyboard, mouse, and timing details
- Check `examples/` for more complete demos
