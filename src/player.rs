use crate::rect::Rect;
use crate::settings::*;
use std::f64::consts::PI;

/// Ввод за один логический кадр.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub escape: bool,
    /// Смещение мыши по X в пикселях с прошлого кадра.
    pub mouse_dx: i32,
    pub focused: bool,
    /// Было нажатие левой кнопки мыши.
    pub click: bool,
}

pub struct Player {
    pub x: f64,
    pub y: f64,
    pub hp: f64,
    pub angle: f64,
    pub sensitivity: f64,
    pub rect: Rect,
    pub shot: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Player {
        let (x, y) = PLAYER_POS;
        let side = 32;
        Player {
            x,
            y,
            hp: 200.0,
            angle: PLAYER_ANGLE,
            sensitivity: 0.0025,
            rect: Rect::new(x as i32, y as i32, side, side),
            shot: false,
        }
    }

    pub fn hurt(&mut self, points: f64) {
        self.hp -= points;
    }

    fn detect_collision(&mut self, mut dx: f64, mut dy: f64, collision: &[Rect]) {
        let next = self.rect.moved(dx, dy);
        let mut hit = false;
        let (mut delta_x, mut delta_y) = (0i64, 0i64);
        for wall in collision.iter().filter(|r| next.colliderect(r)) {
            hit = true;
            delta_x += if dx > 0.0 {
                next.right() - wall.x
            } else {
                wall.right() - next.x
            } as i64;
            delta_y += if dy > 0.0 {
                next.bottom() - wall.y
            } else {
                wall.bottom() - next.y
            } as i64;
        }
        if hit {
            if (delta_x - delta_y).abs() < 20 {
                dx = 0.0;
                dy = 0.0;
            } else if delta_x > delta_y {
                dy = 0.0;
            } else if delta_y > delta_x {
                dx = 0.0;
            }
        }
        self.x += dx;
        self.y += dy;
    }

    /// `Player.movement()`. Возвращает `false`, если нажат Escape (выход из игры).
    pub fn movement(&mut self, input: &Input, collision: &[Rect]) -> bool {
        if !self.keys_control(input, collision) {
            return false;
        }
        if input.focused {
            self.angle += input.mouse_dx as f64 * self.sensitivity;
        }
        self.rect.set_center(self.x, self.y);
        self.angle = py_mod(self.angle, PI * 2.0);
        true
    }

    fn keys_control(&mut self, input: &Input, collision: &[Rect]) -> bool {
        let sin_a = self.angle.sin();
        let cos_a = self.angle.cos();
        if input.escape {
            return false;
        }
        if input.forward {
            self.detect_collision(PLAYER_SPEED * cos_a, PLAYER_SPEED * sin_a, collision);
        }
        if input.back {
            self.detect_collision(-PLAYER_SPEED * cos_a, -PLAYER_SPEED * sin_a, collision);
        }
        if input.left {
            self.detect_collision(PLAYER_SPEED * sin_a, -PLAYER_SPEED * cos_a, collision);
        }
        if input.right {
            self.detect_collision(-PLAYER_SPEED * sin_a, PLAYER_SPEED * cos_a, collision);
        }
        if input.click && !self.shot {
            self.shot = true;
        }
        self.angle = py_mod(self.angle, PI * 2.0);
        true
    }
}

/// Остаток от деления как `%` в Python для положительного делителя.
pub fn py_mod(a: f64, b: f64) -> f64 {
    let r = a % b;
    if r == 0.0 {
        0.0f64.copysign(b)
    } else if (r < 0.0) != (b < 0.0) {
        r + b
    } else {
        r
    }
}
