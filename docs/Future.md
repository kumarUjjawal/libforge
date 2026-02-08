1. Performance notes & improvements for the circle:

Triangle-fan generates segments * 3 vertices. For 32 segments, that’s 96 vertices per circle. For many circles or very large segments, we may:

use an indexed approach (reuse ring vertices with index buffer),

cache the CPU-side ring (if radius and segments repeat),

precompute a unit circle (NDC or normalized) and scale/translate in shader using uniforms/instancing (fastest).

For now, triangle-fan is simple and good for moderate usage (UI badges, markers, icons).

Later we can add an instanced circle rendering path or a shared unit-circle vertex buffer to reduce CPU work and upload size.
