use std::collections::HashSet;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Keyboard keys supported by the input system.
///
/// Uses physical key codes (layout-independent).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    W,
    A,
    S,
    D,
    Q,
    E,
    Space,
    Enter,
    Minus,
    Equal,
    Escape,
}

impl Key {
    fn from_keycode(code: KeyCode) -> Option<Self> {
        Some(match code {
            KeyCode::ArrowLeft => Key::Left,
            KeyCode::ArrowRight => Key::Right,
            KeyCode::ArrowUp => Key::Up,
            KeyCode::ArrowDown => Key::Down,
            KeyCode::KeyW => Key::W,
            KeyCode::KeyA => Key::A,
            KeyCode::KeyS => Key::S,
            KeyCode::KeyD => Key::D,
            KeyCode::KeyQ => Key::Q,
            KeyCode::KeyE => Key::E,
            KeyCode::Space => Key::Space,
            KeyCode::Enter => Key::Enter,
            KeyCode::Minus => Key::Minus,
            KeyCode::Equal => Key::Equal,
            KeyCode::Escape => Key::Escape,
            _ => return None,
        })
    }
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    fn from_winit(mouse_button: WinitMouseButton) -> Option<Self> {
        Some(match mouse_button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => return None,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct InputState {
    keys_down: HashSet<Key>,
    prev_keys_down: HashSet<Key>,

    mouse_down: HashSet<MouseButton>,
    prev_mouse_down: HashSet<MouseButton>,

    mouse_position: (f32, f32),
    mouse_wheel: (f32, f32),
}

impl InputState {
    pub fn begin_frame(&mut self) {
        self.prev_keys_down = self.keys_down.clone();
        self.prev_mouse_down = self.mouse_down.clone();
        self.mouse_wheel = (0.0, 0.0);
    }

    pub fn handle_keyboard_input(&mut self, physical_key: PhysicalKey, state: ElementState) {
        let PhysicalKey::Code(code) = physical_key else {
            return;
        };
        let Some(key) = Key::from_keycode(code) else {
            return;
        };

        match state {
            ElementState::Pressed => {
                self.keys_down.insert(key);
            }
            ElementState::Released => {
                self.keys_down.remove(&key);
            }
        }
    }

    pub fn handle_mouse_button(&mut self, button: WinitMouseButton, state: ElementState) {
        let Some(button) = MouseButton::from_winit(button) else {
            return;
        };

        match state {
            ElementState::Pressed => {
                self.mouse_down.insert(button);
            }

            ElementState::Released => {
                self.mouse_down.remove(&button);
            }
        }
    }

    pub fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_position = (position.x as f32, position.y as f32);
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let (dx, dy) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x, y),
            MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
        };

        self.mouse_wheel.0 += dx;
        self.mouse_wheel.1 += dy;
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_down.contains(&key) && !self.prev_keys_down.contains(&key)
    }

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button) && !self.prev_mouse_down.contains(&button)
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn mouse_wheel(&self) -> (f32, f32) {
        self.mouse_wheel
    }
}
