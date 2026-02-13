# libforge

<table>
<tr>
<td width="50%">

<p align="center">
  <img src="tennis-game.gif" alt="Tennis Game Demo" width="100%" />
</p>

</td>
<td width="50%">

**libforge** is an easy-to-use 2D graphics library for Rust, built on top of [wgpu](https://wgpu.rs/) for cross-platform GPU-accelerated rendering.

### Core Features
-  **Immediate-mode rendering** - Simple draw calls, no complex setup
-  **Game-ready** - Physics, sprites, animations
-  **Texture support** - Load from bytes, sprite sheets, sub-textures
-  **GPU-accelerated** - Powered by wgpu (Metal/Vulkan/DX12/WebGL)
-  **Cross-platform** - macOS, iOS, Windows, Linux, Web
-  **Minimal dependencies** - Just wgpu, winit, and a few utilities

</td>
</tr>
</table>


## Features

-  **Primitives**: `draw_rect()`, `draw_circle()`, `draw_line()`
-  **Textures**: `load_texture_from_bytes()`, `draw_texture()`, `draw_subtexture()`
-  **Sprite Animation**: `SpriteAnimation`, `draw_sprite_animation()`
-  **Alpha Blending**: Full transparency support
-  **Color Tinting**: Modify texture colors on the fly
-  **Immediate Mode**: No complex state management


## Basic Example

```rust
use libforge::{Color, LibContext, Rect};
use std::sync::Arc;
use winit::{event_loop::EventLoop, window::Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let window = Arc::new(event_loop.create_window(Window::default_attributes())?);
    
    let mut ctx = LibContext::new_from_window(window.clone())?;
    
    // Load a texture
    let texture = ctx.load_texture_from_bytes(
        "my_image",
        include_bytes!("my_image.png")
    )?;
    
    // Render loop
    ctx.begin_frame(Some(Color([0.1, 0.1, 0.15, 1.0])));
    
    // Draw a rectangle
    ctx.draw_rect(
        Rect { x: 100.0, y: 100.0, w: 200.0, h: 150.0 },
        Color([1.0, 0.5, 0.2, 1.0])
    );
    
    // Draw a circle
    ctx.draw_circle(300.0, 200.0, 50.0, 32, Color([0.3, 0.7, 1.0, 1.0]));
    
    // Draw a texture
    ctx.draw_texture(
        texture,
        Rect { x: 400.0, y: 100.0, w: 256.0, h: 256.0 },
        Color([1.0, 1.0, 1.0, 1.0])
    );
    
    ctx.end_frame()?;
    
    Ok(())
}
```


## Build

```bash
# Build library
cargo build

# Build with optimizations
cargo build --release
```


## Run Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test texture_loading
```


## Run Examples

```bash
# Simple shapes demo
cargo run --example bouncing_shapes

# Tennis game 
cargo run --example tennis_game

# Texture loading
cargo run --example simple_texture
```


## More Examples

Explore all examples in the [`examples/`](examples/) directory:

###  **Rendering Basics**
- [`hello_rects.rs`](examples/example_rects.rs) - Rectangles, lines, and circles
- [`simple_texture.rs`](examples/simple_texture.rs) - Basic texture loading and display
- [`hello_texture.rs`](examples/example_texture.rs) - Texture rendering example

###  **Texture Features**
- [`texture_tint.rs`](examples/texture_tint.rs) - Color tinting with animation (3×3 grid)
- [`hello_subtexture.rs`](examples/example_subtexture.rs) - Sprite sheet rendering
- [`mixed_rendering.rs`](examples/mixed_rendering.rs) - Combining textures and shapes

###  **Animating**
- [`tennis_game.rs`](examples/tennis_game.rs) - **Fully playable tennis game!** (keyboard controls, AI, physics)
- [`bouncing_shapes.rs`](examples/bouncing_shapes.rs) - Physics simulation with collision detection


## Technology Stack

- **[wgpu](https://wgpu.rs/)** - Cross-platform GPU API (supports Metal, Vulkan, DX12, WebGL)
- **[winit](https://github.com/rust-windowing/winit)** - Cross-platform windowing
- **[image](https://github.com/image-rs/image)** - Image loading (PNG, JPEG, etc.)
- **[glam](https://github.com/bitshifter/glam-rs)** - Math library
- **WGSL** - WebGPU Shading Language (automatically compiled to platform shaders)


## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| macOS    | Metal   |  Fully Supported |
| iOS      | Metal   |  Supported (touch input via winit) |
| Windows  | DX12/Vulkan |  UnTested|
| Linux    | Vulkan  | UnTested |
| Android  | Vulkan  |  Untested |
| Web      | WebGPU/WebGL |  Untested|


## License

See [LICENSE](LICENSE) file for details.
