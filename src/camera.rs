use glam::Mat4;

#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub zoom: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            zoom: 0.0,
        }
    }

    /// Returns the view matrix (maps world -> camera space).
    /// We produce the matrix that should be multiplied on the left of model:
    /// final = projection * view * model
    pub fn view_matrix(&self) -> glam::Mat4 {
        let scale = Mat4::from_scale(glam::vec3(1.0 / self.zoom, 1.0 / self.zoom, 1.0));
        let rotation = Mat4::from_rotation_z(-self.rotation);
        let translation = Mat4::from_translation(glam::vec3(-self.x, -self.y, 0.0));

        scale * rotation * translation
    }
}
