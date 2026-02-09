use crate::Rect;

#[derive(Clone)]
pub struct SpriteAnimation {
    pub frames: Vec<Rect>,
    pub fps: f32,
}

impl SpriteAnimation {
    pub fn frame_at_time(&self, time: f32) -> Rect {
        if self.frames.is_empty() {
            return Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            };
        }

        let frame_count = self.frames.len();
        let frame = ((time * self.fps) as usize) % frame_count;
        self.frames[frame]
    }
}
