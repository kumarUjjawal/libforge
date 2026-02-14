use glam::Mat4;

/// A 2D camera for world-space rendering.
///
/// Use with `begin_mode_2d(camera)` to enter world-space drawing mode.
/// - `x, y`: camera position (world units)
/// - `rotation`: rotation in radians (positive = counter-clockwise)
/// - `zoom`: scale factor (values > 1.0 zoom out, < 1.0 zoom in)
#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            // `zoom` is a scale factor. `1.0` means "no zoom".
            zoom: 1.0,
        }
    }

    /// Returns the view matrix (maps world -> camera space).
    /// We produce the matrix that should be multiplied on the left of model:
    /// final = projection * view * model
    pub fn view_matrix(&self) -> glam::Mat4 {
        // Prevent division-by-zero and keep the view matrix well-defined.
        let zoom = if self.zoom <= 0.0 { 1.0 } else { self.zoom };

        // Note: this convention means zoom > 1.0 zooms *out* (world appears smaller).
        // If you want zoom > 1.0 to zoom *in*, change this to `Mat4::from_scale(vec3(zoom, zoom, 1.0))`.
        let scale = Mat4::from_scale(glam::vec3(1.0 / zoom, 1.0 / zoom, 1.0));
        let rotation = Mat4::from_rotation_z(-self.rotation);
        let translation = Mat4::from_translation(glam::vec3(-self.x, -self.y, 0.0));

        scale * rotation * translation
    }
}
