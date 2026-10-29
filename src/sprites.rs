use crate::raycast::WallHit;
use crate::settings::*;
use std::collections::VecDeque;
use std::f64::consts::PI;

/// Какую картинку врага показать.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteFrame {
    /// `enemy/enemy.png`
    Idle,
    /// `enemy/enemy{i}.png`
    Action(u8),
    /// `enemy_dead/death{i}.png`
    Death(u8),
}

/// (x, y) в тайлах и агрессивность — список из `sprites.py`.
pub const ENEMIES: [(f64, f64, bool); 28] = [
    (8.5, 75.5, false),
    (18.5, 75.5, false),
    (13.5, 60.5, true),
    (9.5, 50.5, true),
    (4.5, 46.5, true),
    (24.5, 48.5, false),
    (24.5, 50.5, false),
    (37.5, 45.5, false),
    (37.5, 51.5, true),
    (43.5, 23.5, false),
    (37.5, 16.5, true),
    (27.5, 7.5, true),
    (59.5, 13.5, true),
    (57.5, 22.5, false),
    (62.5, 13.5, false),
    (63.5, 10.5, true),
    (45.5, 11.5, true),
    (46.5, 38.5, false),
    (38.5, 33.5, true),
    (37.5, 18.5, false),
    (49.5, 18.5, false),
    (28.5, 4.5, true),
    (28.5, 3.5, true),
    (30.5, 4.0, false),
    (23.5, 22.5, false),
    (43.5, 46.5, false),
    (52.5, 7.5, true),
    (58.5, 29.5, true),
];

/// `x ** 2` в Python — это вызов `pow()` из libm, а он не всегда совпадает с `x * x`.
/// `black_box` не даёт компилятору заменить вызов умножением.
fn py_square(x: f64) -> f64 {
    x.powf(std::hint::black_box(2.0))
}

/// Расстояние до спрайта (с поправкой на рыбий глаз) и номер луча — начало `sprite_locate`.
pub fn project(sx: f64, sy: f64, px: f64, py: f64, angle: f64) -> (f64, i32) {
    let (dx, dy) = (sx - px, sy - py);
    let mut d_sprite = (py_square(dx) + py_square(dy)).sqrt();
    let theta = dy.atan2(dx);
    let mut gamma = theta - angle;
    let degrees = angle.to_degrees();
    if dx > 0.0 && (180.0..=360.0).contains(&degrees) || dx < 0.0 && dy < 0.0 {
        gamma += PI * 2.0;
    }
    let delta_rays = (gamma / DELTA_ANGLE) as i32;
    let ray = CENTER_RAY + delta_rays;
    d_sprite *= (HALF_FOV - ray as f64 * DELTA_ANGLE).cos();
    (d_sprite, ray)
}

/// Высота проекции, если спрайт в поле зрения и ближе стены на своём луче.
pub fn visible_projection(d_sprite: f64, ray: i32, walls: &[WallHit]) -> Option<i32> {
    if (0..RAYS_NUMBER).contains(&ray) && d_sprite < walls[ray as usize].depth {
        Some(((*SCALE_COEFFICIENT / d_sprite) as i32).min(DOUBLE_HEIGHT))
    } else {
        None
    }
}

/// Прямоугольник на экране (x, y, w, h), куда рисуется спрайт.
pub fn sprite_rect(projection_height: i32, ray: i32, dead: bool) -> (i32, i32, i32, i32) {
    let sprite_width = projection_height;
    let mut sprite_height = projection_height;
    let half_sprite_width = sprite_width / 2;
    let half_sprite_height = sprite_height / 2;
    let mut shift = 0.0;
    if dead {
        shift = half_sprite_height as f64 * 0.5;
        sprite_height = (sprite_height as f64 / 1.3) as i32;
    }
    let x = ray * SCALE - half_sprite_width;
    let y = (HALF_HEIGHT as f64 - half_sprite_height as f64 + shift) as i32;
    (x, y, sprite_width, sprite_height)
}

pub struct Sprite {
    pub x: f64,
    pub y: f64,
    pub side: i32,
    pub aggressive: bool,
    pub obj_action: VecDeque<u8>,
    pub death_animation: VecDeque<u8>,
    pub dead_sprite: Option<u8>,
    pub dead_animation_count: i32,
    pub dead_animation_speed: i32,
    pub animation_speed: i32,
    pub animation_count: i32,
    pub is_dead: bool,
    pub blocked: bool,
    /// В оригинале изначально `(False,)` — непустой кортеж, то есть истина.
    pub npc_action_trigger: bool,
    pub projection_height: i32,
    pub d_sprite: f64,
    pub ray: i32,
    /// Снимок для отрисовки: где спрайт стоял и что показывал в момент `sprite_locate`.
    pub snap: SpriteSnap,
    pub prev_snap: (f64, f64),
}

#[derive(Clone, Copy, Debug)]
pub struct SpriteSnap {
    pub x: f64,
    pub y: f64,
    pub dead: bool,
    pub frame: Option<SpriteFrame>,
}

impl Sprite {
    pub fn new(tx: f64, ty: f64, aggressive: bool) -> Sprite {
        let (x, y) = (tx * TILE, ty * TILE);
        Sprite {
            x,
            y,
            side: 50,
            aggressive,
            obj_action: (0..4).collect(),
            death_animation: (0..5).collect(),
            dead_sprite: None,
            dead_animation_count: 0,
            dead_animation_speed: 16,
            animation_speed: 40,
            animation_count: 0,
            is_dead: false,
            blocked: true,
            npc_action_trigger: true,
            projection_height: 0,
            d_sprite: 0.0,
            ray: 0,
            snap: SpriteSnap {
                x,
                y,
                dead: false,
                frame: None,
            },
            prev_snap: (x, y),
        }
    }

    pub fn all() -> Vec<Sprite> {
        ENEMIES
            .iter()
            .map(|&(x, y, a)| Sprite::new(x, y, a))
            .collect()
    }

    /// `is_on_fire[1]`: высота проекции, если спрайт под прицелом.
    pub fn on_fire_projection(&self) -> Option<i32> {
        let half = self.side / 2;
        if CENTER_RAY - half < self.ray && self.ray < CENTER_RAY + half && self.blocked {
            Some(self.projection_height)
        } else {
            None
        }
    }

    /// `sprite_locate` без рисования: обновляет состояние и анимацию.
    pub fn locate(&mut self, px: f64, py: f64, angle: f64, walls: &[WallHit]) {
        let (d_sprite, ray) = project(self.x, self.y, px, py, angle);
        self.d_sprite = d_sprite;
        self.ray = ray;
        self.snap = SpriteSnap {
            x: self.x,
            y: self.y,
            dead: self.is_dead,
            frame: None,
        };
        if let Some(ph) = visible_projection(d_sprite, ray, walls) {
            self.projection_height = ph;
            let frame = if self.is_dead {
                self.dead_animation()
            } else if self.npc_action_trigger {
                self.npc_in_action()
            } else {
                // анимации простоя у врага нет — всегда enemy.png
                SpriteFrame::Idle
            };
            self.snap.frame = Some(frame);
        }
    }

    fn dead_animation(&mut self) -> SpriteFrame {
        if !self.death_animation.is_empty() {
            if self.dead_animation_count < self.animation_speed {
                self.dead_sprite = self.death_animation.front().copied();
                self.dead_animation_count += self.dead_animation_speed;
            } else {
                self.dead_sprite = self.death_animation.pop_front();
                self.dead_animation_count = 0;
            }
        }
        SpriteFrame::Death(self.dead_sprite.expect("death animation started"))
    }

    fn npc_in_action(&mut self) -> SpriteFrame {
        let frame = self.obj_action[0];
        if self.animation_count < self.animation_speed {
            self.animation_count += 4;
        } else {
            self.obj_action.rotate_right(1);
        }
        SpriteFrame::Action(frame)
    }

    /// Кадр для отрисовки между логическими тиками.
    pub fn render_frame(&self) -> SpriteFrame {
        if let Some(frame) = self.snap.frame {
            frame
        } else if self.snap.dead {
            SpriteFrame::Death(
                self.dead_sprite
                    .or(self.death_animation.front().copied())
                    .unwrap_or(4),
            )
        } else if self.npc_action_trigger {
            SpriteFrame::Action(self.obj_action[0])
        } else {
            SpriteFrame::Idle
        }
    }

    /// Позиция для отрисовки с интерполяцией между двумя тиками.
    pub fn render_pos(&self, alpha: f64) -> (f64, f64) {
        (
            lerp(self.prev_snap.0, self.snap.x, alpha),
            lerp(self.prev_snap.1, self.snap.y, alpha),
        )
    }
}

/// Интерполяция, точно возвращающая `b` при `t == 1`.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    b - (b - a) * (1.0 - t)
}
