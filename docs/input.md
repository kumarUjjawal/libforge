# Input and Timing

## Overview

libforge provides simple polling-based input. Check the current state of keys, mouse buttons, cursor position, and scroll wheel at any time during your frame.

## Feeding Events

You must feed winit window events into the context:

```rust
Event::WindowEvent { event, .. } => {
    ctx.handle_window_event(&event);
    // ... rest of your event handling
}
```

This updates the internal input state.

## Keyboard

### Continuous Input (Holding)

```rust
if ctx.is_key_down(Key::W) {
    player_y -= speed * ctx.frame_time();
}
```

Returns `true` every frame while the key is held.

### Edge Detection (Press)

```rust
if ctx.is_key_pressed(Key::Space) {
    jump();
}
```

Returns `true` only on the frame the key transitions from up to down.

### Supported Keys

See `Key` enum in `src/input.rs`:
- Arrow keys: `Left`, `Right`, `Up`, `Down`
- WASD: `W`, `A`, `S`, `D`
- Other: `Q`, `E`, `Space`, `Enter`, `Escape`, `Minus`, `Equal`

Keys use physical codes (layout-independent).

## Mouse

### Buttons

```rust
if ctx.is_mouse_button_down(MouseButton::Left) {
    // Held
}

if ctx.is_mouse_button_pressed(MouseButton::Left) {
    // Just clicked this frame
}
```

Supported buttons: `Left`, `Right`, `Middle`.

### Cursor Position

```rust
let (x, y) = ctx.mouse_position();
```

Returns screen-space pixel coordinates. `(0, 0)` is top-left.

### Scroll Wheel

```rust
let (dx, dy) = ctx.mouse_wheel();
```

Returns the scroll delta for this frame. Positive `dy` = scroll up.

The delta resets to `(0, 0)` at the start of each frame (in `begin_drawing()`).

## Timing

### Frame Time

```rust
let dt = ctx.frame_time();
player_x += velocity * dt;
```

Returns the time elapsed since the last frame in seconds. Use this for smooth, frame-rate-independent movement.

### FPS

```rust
let fps = ctx.fps();
```

Returns the current frames per second, computed as `1.0 / frame_time()`.

## Input State Lifecycle

1. `begin_drawing()` is called
2. Previous frame's input state is saved (for edge detection)
3. Scroll wheel resets to zero
4. Your game logic reads input via `is_key_down`, `mouse_position`, etc.
5. `handle_window_event` updates state as events arrive
6. Repeat next frame

Edge detection (`is_key_pressed`, `is_mouse_button_pressed`) compares current state to the previous frame, so it only triggers once per press.

## Example: Smooth Player Movement

```rust
ctx.begin_drawing();
ctx.clear_background(Color::BLACK);

let speed = 200.0; // pixels per second
let dt = ctx.frame_time();

if ctx.is_key_down(Key::Right) { player_x += speed * dt; }
if ctx.is_key_down(Key::Left)  { player_x -= speed * dt; }
if ctx.is_key_down(Key::Down)  { player_y += speed * dt; }
if ctx.is_key_down(Key::Up)    { player_y -= speed * dt; }

if ctx.is_key_pressed(Key::Space) {
    fire_bullet(player_x, player_y);
}

ctx.draw_circle(player_x, player_y, 16.0, 32, Color::WHITE);

ctx.end_drawing().unwrap();
```
