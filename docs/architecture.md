# Architecture

## Rendering Model

libforge uses an immediate-mode API with deferred rendering.

### Immediate Mode

You call draw functions directly each frame:

```rust
ctx.draw_rect(rect, color);
ctx.draw_circle(x, y, radius, 32, color);
```

This feels natural and keeps game code simple.

### Deferred Execution

Draw calls do not render immediately. Instead:

1. Each draw call generates vertices (CPU-side)
2. Vertices are batched by draw type (colored shapes vs textured)
3. At `end_drawing()`, all batches are uploaded to GPU and rendered in one pass

This gives you immediate-mode ergonomics with efficient GPU usage.

## Transform Pipeline

The framework uses a GPU uniform matrix for view/projection and CPU-side transforms for per-draw operations.

### Screen Space (Default)

By default, coordinates are screen pixels:

```rust
// Draw at pixel (100, 50)
ctx.draw_rect(Rect { x: 100.0, y: 50.0, w: 64.0, h: 64.0 }, color);
```

Internally, the GPU uniform matrix is set to an orthographic projection that maps pixel coordinates to clip space.

### World Space (Camera Mode)

Enter world-space mode with a camera:

```rust
let camera = Camera2D {
    x: player_x,
    y: player_y,
    rotation: 0.0,
    zoom: 1.0,
};

ctx.begin_mode_2d(camera);

// Now drawing happens in world coordinates
ctx.draw_rect(Rect { x: world_x, y: world_y, w: 32.0, h: 32.0 }, color);

ctx.end_mode_2d();

// Back to screen-space
```

The GPU uniform is updated to `projection * camera.view_matrix()` while in camera mode.

### Per-Draw Transforms (Matrix Stack)

You can apply transforms to individual draw calls using the matrix stack:

```rust
ctx.push_matrix();
ctx.translate(100.0, 50.0);
ctx.rotate_z(angle);
ctx.scale(2.0, 2.0);

// This rectangle is transformed
ctx.draw_rect(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, color);

ctx.pop_matrix();

// This one is not
ctx.draw_rect(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, color);
```

These transforms are applied on the CPU before vertices are buffered. The current top-of-stack matrix multiplies each vertex position.

### Complete Transform Chain

Final vertex position in clip space:

```
clip_pos = projection * view * model * vertex_pos
```

Where:
- `projection` = orthographic matrix (screen size)
- `view` = camera matrix (if in mode_2d) or identity
- `model` = current matrix stack top
- `vertex_pos` = position from draw call

The GPU uniform holds `projection * view`. The `model` transform is applied on CPU.

## Batching

Draw calls are batched by:

1. Pipeline (colored vs textured)
2. Texture (for textured draws)

This minimizes GPU state changes. The renderer automatically handles batching.

## Resize Behavior

When the window resizes:

```rust
ctx.resize(new_width, new_height);
```

The orthographic projection matrix is recalculated. If you are in camera mode, the camera view is preserved and re-applied with the new projection.

## Module Structure

```
src/
  lib.rs           - public API (LibContext)
  renderer/
    mod.rs         - draw recording, batching, frame submission
    gpu.rs         - wgpu setup, pipelines, render pass
    geometry.rs    - CPU-side shape tessellation
  camera.rs        - Camera2D + view matrix
  input.rs         - keyboard/mouse state tracking
  vertex.rs        - Vertex layout
  shaders/
    basic.wgsl     - vertex + fragment shaders
```

The public API is in `LibContext`. All rendering details are internal.
