use libforge::{Color, LibContext, Rect};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const COURT_WIDTH: f32 = 800.0;
const COURT_HEIGHT: f32 = 600.0;
const PLAYER_SIZE: f32 = 20.0;
const BALL_RADIUS: f32 = 8.0;

struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    width: f32,
    height: f32,
    color: Color,
}

impl Player {
    fn new(x: f32, y: f32, color: Color) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            width: PLAYER_SIZE,
            height: PLAYER_SIZE * 1.5,
            color,
        }
    }

    fn update(&mut self, dt: f32) {
        // Update position
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        // Apply friction
        self.vx *= 0.85;
        self.vy *= 0.85;

        // Clamp to court bounds
        self.x = self.x.clamp(0.0, COURT_WIDTH - self.width);
        self.y = self.y.clamp(0.0, COURT_HEIGHT - self.height);
    }

    fn move_input(&mut self, dx: f32, dy: f32) {
        let speed = 300.0;
        self.vx += dx * speed;
        self.vy += dy * speed;

        // Cap max speed
        let max_speed = 400.0;
        let speed_sq = self.vx * self.vx + self.vy * self.vy;
        if speed_sq > max_speed * max_speed {
            let speed = speed_sq.sqrt();
            self.vx = (self.vx / speed) * max_speed;
            self.vy = (self.vy / speed) * max_speed;
        }
    }

    fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.width,
            h: self.height,
        }
    }

    fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

struct Ball {
    x: f32,
    y: f32,
    z: f32, // height above ground
    vx: f32,
    vy: f32,
    vz: f32, // vertical velocity
    radius: f32,
    trail: Vec<(f32, f32, f32)>, // position history for trail effect
}

impl Ball {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            z: 0.0,
            vx: 200.0,
            vy: 150.0,
            vz: 0.0,
            radius: BALL_RADIUS,
            trail: Vec::new(),
        }
    }

    fn update(&mut self, dt: f32) {
        // Update position
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.z += self.vz * dt;

        // Gravity
        self.vz -= 800.0 * dt;

        // Bounce off ground
        if self.z <= 0.0 {
            self.z = 0.0;
            self.vz = -self.vz * 0.7; // bounce with energy loss
            
            // Apply friction when on ground
            self.vx *= 0.98;
            self.vy *= 0.98;

            // Stop if moving very slowly
            if self.vz.abs() < 50.0 {
                self.vz = 0.0;
            }
        }

        // Bounce off walls
        if self.x - self.radius < 0.0 || self.x + self.radius > COURT_WIDTH {
            self.vx = -self.vx;
            self.x = self.x.clamp(self.radius, COURT_WIDTH - self.radius);
        }
        if self.y - self.radius < 0.0 || self.y + self.radius > COURT_HEIGHT {
            self.vy = -self.vy;
            self.y = self.y.clamp(self.radius, COURT_HEIGHT - self.radius);
        }

        // Update trail
        self.trail.push((self.x, self.y, self.z));
        if self.trail.len() > 10 {
            self.trail.remove(0);
        }
    }

    fn hit(&mut self, px: f32, py: f32, power: f32) {
        // Calculate direction from player to ball
        let dx = self.x - px;
        let dy = self.y - py;
        let dist = (dx * dx + dy * dy).sqrt();
        
        if dist > 0.0 {
            self.vx = (dx / dist) * power;
            self.vy = (dy / dist) * power;
            self.vz = 300.0; // upward velocity
        }
    }

    fn check_collision(&self, player: &Player) -> bool {
        let (px, py) = player.center();
        let dx = self.x - px;
        let dy = self.y - py;
        let dist_sq = dx * dx + dy * dy;
        let collision_dist = self.radius + (player.width + player.height) / 4.0;
        
        dist_sq < collision_dist * collision_dist && self.z < 30.0
    }
}

struct TennisGame {
    player1: Player,
    player2: Player,
    ball: Ball,
    score: [u32; 2],
    hit_effect: Option<HitEffect>,
}

struct HitEffect {
    x: f32,
    y: f32,
    timer: f32,
}

impl TennisGame {
    fn new() -> Self {
        Self {
            player1: Player::new(100.0, COURT_HEIGHT / 2.0 - 15.0, Color([0.3, 0.7, 1.0, 1.0])),
            player2: Player::new(COURT_WIDTH - 120.0, COURT_HEIGHT / 2.0 - 15.0, Color([1.0, 0.4, 0.4, 1.0])),
            ball: Ball::new(COURT_WIDTH / 2.0, COURT_HEIGHT / 2.0),
            score: [0, 0],
            hit_effect: None,
        }
    }

    fn update(&mut self, dt: f32) {
        self.player1.update(dt);
        self.player2.update(dt);
        self.ball.update(dt);

        // Simple AI for player 2
        let (p2x, p2y) = self.player2.center();
        let dx = self.ball.x - p2x;
        let dy = self.ball.y - p2y;
        
        if dx.abs() > 5.0 {
            self.player2.move_input(dx.signum() * 0.3, 0.0);
        }
        if dy.abs() > 5.0 {
            self.player2.move_input(0.0, dy.signum() * 0.3);
        }

        // Check collisions
        if self.ball.check_collision(&self.player1) {
            let (px, py) = self.player1.center();
            self.ball.hit(px, py, 400.0);
            self.hit_effect = Some(HitEffect { x: self.ball.x, y: self.ball.y, timer: 0.3 });
        }
        if self.ball.check_collision(&self.player2) {
            let (px, py) = self.player2.center();
            self.ball.hit(px, py, 400.0);
            self.hit_effect = Some(HitEffect { x: self.ball.x, y: self.ball.y, timer: 0.3 });
        }

        // Update hit effect
        if let Some(effect) = &mut self.hit_effect {
            effect.timer -= dt;
            if effect.timer <= 0.0 {
                self.hit_effect = None;
            }
        }

        // Score when ball goes out on sides
        if self.ball.x < 0.0 && self.ball.z < 5.0 {
            self.score[1] += 1;
            self.reset_ball();
        } else if self.ball.x > COURT_WIDTH && self.ball.z < 5.0 {
            self.score[0] += 1;
            self.reset_ball();
        }
    }

    fn reset_ball(&mut self) {
        self.ball = Ball::new(COURT_WIDTH / 2.0, COURT_HEIGHT / 2.0);
    }

    fn render(&self, ctx: &mut LibContext<Arc<Window>>) {
        ctx.begin_frame(Some(Color([0.15, 0.5, 0.2, 1.0]))); // green court

        // Draw court markings
        self.draw_court(ctx);

        // Draw shadows
        self.draw_shadow(ctx, self.player1.x + self.player1.width / 2.0, self.player1.y + self.player1.height, 12.0);
        self.draw_shadow(ctx, self.player2.x + self.player2.width / 2.0, self.player2.y + self.player2.height, 12.0);
        
        // Ball shadow
        let shadow_offset = self.ball.z * 0.3;
        self.draw_shadow(ctx, self.ball.x + shadow_offset, self.ball.y + shadow_offset, 
                        self.ball.radius * (1.0 - self.ball.z / 200.0).max(0.3));

        // Draw ball trail
        for (i, (tx, ty, tz)) in self.ball.trail.iter().enumerate() {
            let alpha = (i as f32 / self.ball.trail.len() as f32) * 0.5;
            ctx.draw_circle(*tx, *ty - tz, 4.0, 8, Color([1.0, 1.0, 0.5, alpha]));
        }

        // Draw players
        ctx.draw_rect(self.player1.rect(), self.player1.color);
        ctx.draw_rect(self.player2.rect(), self.player2.color);

        // Draw ball
        ctx.draw_circle(
            self.ball.x, 
            self.ball.y - self.ball.z, 
            self.ball.radius, 
            16, 
            Color([1.0, 0.9, 0.2, 1.0])
        );

        // Draw hit effect
        if let Some(effect) = &self.hit_effect {
            let alpha = effect.timer / 0.3;
            let radius = (1.0 - alpha) * 30.0 + 10.0;
            ctx.draw_circle(effect.x, effect.y, radius, 24, Color([1.0, 1.0, 1.0, alpha * 0.6]));
        }

        // Draw score (visual bars)
        self.draw_score(ctx);

        ctx.end_frame().expect("end_frame failed");
    }

    fn draw_court(&self, ctx: &mut LibContext<Arc<Window>>) {
        // Center line
        ctx.draw_line(COURT_WIDTH / 2.0, 0.0, COURT_WIDTH / 2.0, COURT_HEIGHT, 3.0, Color([1.0, 1.0, 1.0, 0.5]));
        
        // Service lines
        ctx.draw_line(0.0, COURT_HEIGHT / 3.0, COURT_WIDTH, COURT_HEIGHT / 3.0, 2.0, Color([1.0, 1.0, 1.0, 0.4]));
        ctx.draw_line(0.0, 2.0 * COURT_HEIGHT / 3.0, COURT_WIDTH, 2.0 * COURT_HEIGHT / 3.0, 2.0, Color([1.0, 1.0, 1.0, 0.4]));
        
        // Border
        ctx.draw_rect(Rect { x: 0.0, y: 0.0, w: COURT_WIDTH, h: 5.0 }, Color([1.0, 1.0, 1.0, 0.8]));
        ctx.draw_rect(Rect { x: 0.0, y: COURT_HEIGHT - 5.0, w: COURT_WIDTH, h: 5.0 }, Color([1.0, 1.0, 1.0, 0.8]));
        ctx.draw_rect(Rect { x: 0.0, y: 0.0, w: 5.0, h: COURT_HEIGHT }, Color([1.0, 1.0, 1.0, 0.8]));
        ctx.draw_rect(Rect { x: COURT_WIDTH - 5.0, y: 0.0, w: 5.0, h: COURT_HEIGHT }, Color([1.0, 1.0, 1.0, 0.8]));

        // Net (center)
        for i in 0..20 {
            let y = i as f32 * (COURT_HEIGHT / 20.0);
            ctx.draw_rect(
                Rect { x: COURT_WIDTH / 2.0 - 2.0, y, w: 4.0, h: COURT_HEIGHT / 40.0 },
                Color([0.8, 0.8, 0.8, 0.6])
            );
        }
    }

    fn draw_shadow(&self, ctx: &mut LibContext<Arc<Window>>, x: f32, y: f32, radius: f32) {
        ctx.draw_circle(x, y, radius, 12, Color([0.0, 0.0, 0.0, 0.3]));
    }

    fn draw_score(&self, ctx: &mut LibContext<Arc<Window>>) {
        // Player 1 score (left side)
        for i in 0..self.score[0] {
            ctx.draw_circle(
                20.0 + i as f32 * 30.0,
                20.0,
                10.0,
                16,
                Color([0.3, 0.7, 1.0, 0.8])
            );
        }

        // Player 2 score (right side)
        for i in 0..self.score[1] {
            ctx.draw_circle(
                COURT_WIDTH - 20.0 - i as f32 * 30.0,
                20.0,
                10.0,
                16,
                Color([1.0, 0.4, 0.4, 0.8])
            );
        }
    }
}

struct App {
    window: Option<Arc<Window>>,
    ctx: Option<LibContext<Arc<Window>>>,
    game: TennisGame,
    keys: KeyState,
}

#[derive(Default)]
struct KeyState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("libforge - Tennis Game")
                        .with_inner_size(PhysicalSize::new(COURT_WIDTH as u32, COURT_HEIGHT as u32)),
                )
                .unwrap(),
        );
        self.window = Some(window.clone());
        let mut ctx = LibContext::new_from_window(window.clone()).unwrap();
        // Initialize the transform pipeline to pixel-space orthographic projection.
        ctx.reset_transform();
        self.ctx = Some(ctx);
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                match key {
                    KeyCode::ArrowUp | KeyCode::KeyW => self.keys.up = pressed,
                    KeyCode::ArrowDown | KeyCode::KeyS => self.keys.down = pressed,
                    KeyCode::ArrowLeft | KeyCode::KeyA => self.keys.left = pressed,
                    KeyCode::ArrowRight | KeyCode::KeyD => self.keys.right = pressed,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ctx) = &mut self.ctx {
                    // Update game
                    let dt = 1.0 / 60.0;

                    // Handle player 1 input
                    let mut dx = 0.0;
                    let mut dy = 0.0;
                    if self.keys.left { dx -= 1.0; }
                    if self.keys.right { dx += 1.0; }
                    if self.keys.up { dy -= 1.0; }
                    if self.keys.down { dy += 1.0; }
                    
                    if dx != 0.0 || dy != 0.0 {
                        self.game.player1.move_input(dx, dy);
                    }

                    self.game.update(dt);

                    // Render
                    self.game.render(ctx);

                    // Request next frame
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None,
        ctx: None,
        game: TennisGame::new(),
        keys: KeyState::default(),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
