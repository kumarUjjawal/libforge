// Geometry generation helpers (CPU-side)
//
// This module is intentionally `wgpu`-free.

use crate::vertex::Vertex;
use std::f32::consts::PI;

// helper: convert a line (x1,y1)-(x2,y2) and thickness into a quad (4 points)
// Returns points in CCW order: [top-left, top-right, bottom-right, bottom-left]
pub(crate) fn line_to_quad(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32) -> [[f32; 2]; 4] {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6); // avoid div by zero
    let ux = dx / len;
    let uy = dy / len;

    // perpendicular (pointing "up" relative to line direction)
    let px = -uy;
    let py = ux;

    let half = thickness * 0.5;
    let ox = px * half;
    let oy = py * half;

    // top-left  = p1 + perp
    // top-right = p2 + perp
    // bottom-right = p2 - perp
    // bottom-left  = p1 - perp
    [
        [x1 + ox, y1 + oy], // tl
        [x2 + ox, y2 + oy], // tr
        [x2 - ox, y2 - oy], // br
        [x1 - ox, y1 - oy], // bl
    ]
}

// helper: convert quad corners into 6 vertices (two triangles).
// uv is unused for colored geometry so set to 0.0
pub(crate) fn quad_to_vertices(quad: [[f32; 2]; 4], color: [f32; 4]) -> Vec<Vertex> {
    let p0 = quad[0];
    let p1 = quad[1];
    let p2 = quad[2];
    let p3 = quad[3];

    vec![
        Vertex {
            pos: [p0[0], p0[1]],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [p1[0], p1[1]],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [p2[0], p2[1]],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [p0[0], p0[1]],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [p2[0], p2[1]],
            uv: [0.0, 0.0],
            color,
        },
        Vertex {
            pos: [p3[0], p3[1]],
            uv: [0.0, 0.0],
            color,
        },
    ]
}

// helper: build a triangle-fan circle in pixel-space
// returns Vec<Vertex> with triangles (center, p_i, p_i+1)
pub(crate) fn circle_to_vertices(
    cx: f32,
    cy: f32,
    radius: f32,
    segments: usize,
    color: [f32; 4],
) -> Vec<Vertex> {
    let mut verts = Vec::with_capacity(segments * 3);
    let step = 2.0 * PI / (segments as f32);

    for i in 0..segments {
        let a0 = (i as f32) * step;
        let a1 = ((i + 1) as f32) * step;
        let x0 = cx + a0.cos() * radius;
        let y0 = cy + a0.sin() * radius;
        let x1 = cx + a1.cos() * radius;
        let y1 = cy + a1.sin() * radius;

        // triangle (center, p0, p1)
        verts.push(Vertex {
            pos: [cx, cy],
            uv: [0.0, 0.0],
            color,
        });
        verts.push(Vertex {
            pos: [x0, y0],
            uv: [0.0, 0.0],
            color,
        });
        verts.push(Vertex {
            pos: [x1, y1],
            uv: [0.0, 0.0],
            color,
        });
    }

    verts
}
