1. Performance notes & improvements for the circle:

Triangle-fan generates segments * 3 vertices. For 32 segments, that’s 96 vertices per circle. For many circles or very large segments, we may:

use an indexed approach (reuse ring vertices with index buffer),

cache the CPU-side ring (if radius and segments repeat),

precompute a unit circle (NDC or normalized) and scale/translate in shader using uniforms/instancing (fastest).

For now, triangle-fan is simple and good for moderate usage (UI badges, markers, icons).

Later we can add an instanced circle rendering path or a shared unit-circle vertex buffer to reduce CPU work and upload size.

2. Texture:

Pipeline ordering: In this simple approach we issue draw calls per command. For performance later, we’ll want to batch commands by pipeline and by texture (group all DrawCommand::Texture with same texture into one draw call), avoiding pipeline and bind group rebinding. That is a next optimization.

Texture format: I used Rgba8UnormSrgb. Change if we need linear colors.

UV coordinate convention: I used (0,0) top-left and (1,1) bottom-right. WGSL textureSample uses normalized coords with (0,0) at top-left for typical 2D textures; this is consistent with our NDC mapping that flips Y so the texture appears upright. If we see it vertically flipped, swap v coordinate accordingly.

Sampler filtering: I used linear filtering. For pixel-art use Nearest.

Bind group lawet must match shader @group(0) @binding(0) and binding(1) as in WGSL.

Error conversion: return RendererError::Internal(...) for image decode errors or convert appropriate wgpu errors.
