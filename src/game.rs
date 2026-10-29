use crate::map::WorldMap;
use crate::player::{Input, Player, py_mod};
use crate::raycast::ray_casting;
use crate::settings::*;
use crate::sprites::{Sprite, lerp};
use std::collections::VecDeque;
use std::f64::consts::PI;

/// Видит ли враг игрока (`ray_casting_npc_player` из `interactions.py`).
pub fn ray_casting_npc_player(npc_x: f64, npc_y: f64, map: &WorldMap, x0: f64, y0: f64) -> bool {
    let x_square = (x0 / TILE).floor() * TILE;
    let y_square = (y0 / TILE).floor() * TILE;
    let (delta_x, delta_y) = (x0 - npc_x, y0 - npc_y);
    let ray_angle = delta_y.atan2(delta_x) + PI;

    let mut sin_a = ray_angle.sin();
    let mut cos_a = ray_angle.cos();
    if sin_a == 0.0 {
        sin_a = 0.000001;
    }
    if cos_a == 0.0 {
        cos_a = 0.000001;
    }

    // алгоритм брезенхема
    let (mut x, dx) = if cos_a >= 0.0 {
        (x_square + TILE, 1.0)
    } else {
        (x_square, -1.0)
    };
    for _ in 0..(delta_x.abs() as i64) / TILE_SIZE as i64 {
        let depth_v = (x - x0) / cos_a;
        let y_vertical = y0 + depth_v * sin_a;
        if map.at(x + dx, y_vertical).is_some() {
            return false;
        }
        x += dx * TILE;
    }

    let (mut y, dy) = if sin_a >= 0.0 {
        (y_square + TILE, 1.0)
    } else {
        (y_square, -1.0)
    };
    for _ in 0..(delta_y.abs() as i64) / TILE_SIZE as i64 {
        let depth_h = (y - y0) / sin_a;
        let x_horizontal = x0 + depth_h * cos_a;
        if map.at(x_horizontal, y + dy).is_some() {
            return false;
        }
        y += dy * TILE;
    }
    true
}

/// Состояние анимации дробовика (`Rendering.weapon`).
pub struct Weapon {
    /// Индексы кадров gun1..gun5 (0..5).
    pub animation: VecDeque<u8>,
    pub shot_length: i32,
    pub shot_length_count: i32,
    pub shot_sound_length_count: i32,
    pub shot_animation_speed: i32,
    pub shot_animation_count: i32,
    pub shot_animation_trigger: bool,
    /// Что рисовать: `None` — gun0, `Some(i)` — кадр выстрела gun{i+1}.
    pub display: Option<u8>,
}

impl Default for Weapon {
    fn default() -> Self {
        Self::new()
    }
}

impl Weapon {
    pub fn new() -> Weapon {
        Weapon {
            animation: (0..5).collect(),
            shot_length: 5,
            shot_length_count: 0,
            shot_sound_length_count: 0,
            shot_animation_speed: 7,
            shot_animation_count: 0,
            shot_animation_trigger: true,
            display: None,
        }
    }

    /// Шаг анимации. Возвращает `true`, если нужно проиграть звук выстрела.
    fn update(&mut self, shot: &mut bool) -> bool {
        if !*shot {
            self.display = None;
            return false;
        }
        let play_sound = self.shot_sound_length_count == 0;
        self.display = Some(self.animation[0]);
        self.shot_animation_count += 1;
        if self.shot_animation_count == self.shot_animation_speed {
            self.animation.rotate_left(1);
            self.shot_animation_count = 0;
            self.shot_length_count += 1;
            self.shot_sound_length_count += 1;
            self.shot_animation_trigger = false;
        }
        if self.shot_animation_count == 1 {
            self.shot_sound_length_count += 1;
        }
        if self.shot_length_count == self.shot_length {
            *shot = false;
            self.shot_length_count = 0;
            self.shot_sound_length_count = 0;
            self.shot_animation_trigger = true;
        }
        play_sound
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TickEvents {
    pub shot_sound: bool,
}

/// Положение камеры для отрисовки.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    /// Доля времени между прошлым и текущим тиком (1.0 — ровно текущий).
    pub alpha: f64,
}

pub struct Game {
    pub map: WorldMap,
    pub player: Player,
    pub sprites: Vec<Sprite>,
    pub weapon: Weapon,
    pub hurt_count: i32,
    /// Сколько раз за последний тик вызывался `render.damage()`.
    pub damage_count: u32,
    pub enemies: usize,
    pub prev_pos: (f64, f64),
}

impl Game {
    pub fn new(map: WorldMap) -> Game {
        let player = Player::new();
        let prev_pos = (player.x, player.y);
        let sprites = Sprite::all();
        let enemies = sprites.len();
        Game {
            map,
            player,
            sprites,
            weapon: Weapon::new(),
            hurt_count: 0,
            damage_count: 0,
            enemies,
            prev_pos,
        }
    }

    /// Один кадр игровой логики `main.py` (без рисования).
    pub fn tick(&mut self, input: &Input) -> (Outcome, TickEvents) {
        let mut events = TickEvents::default();
        if self.player.hp < 1.0 {
            return (Outcome::Lost, events);
        }
        self.prev_pos = (self.player.x, self.player.y);
        for s in &mut self.sprites {
            s.prev_snap = (s.snap.x, s.snap.y);
        }

        if !self.player.movement(input, &self.map.collision) {
            return (Outcome::Quit, events);
        }
        let (px, py, angle) = (self.player.x, self.player.y, self.player.angle);
        let walls = ray_casting(px, py, angle, &self.map);
        for s in &mut self.sprites {
            s.locate(px, py, angle, &walls);
        }
        events.shot_sound = self.weapon.update(&mut self.player.shot);
        self.interaction_objects();
        self.npc_action();

        self.enemies = self.sprites.iter().filter(|s| !s.is_dead).count();
        if self.enemies == 0 {
            return (Outcome::Won, events);
        }
        (Outcome::Continue, events)
    }

    fn interaction_objects(&mut self) {
        if !(self.player.shot && self.weapon.shot_animation_trigger) {
            return;
        }
        let mut order: Vec<usize> = (0..self.sprites.len()).collect();
        order.sort_by(|&a, &b| {
            let (da, db) = (self.sprites[a].d_sprite, self.sprites[b].d_sprite);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        for i in order {
            let obj = &self.sprites[i];
            if obj.on_fire_projection().is_some_and(|ph| ph != 0) {
                if !obj.is_dead
                    && ray_casting_npc_player(obj.x, obj.y, &self.map, self.player.x, self.player.y)
                {
                    let obj = &mut self.sprites[i];
                    obj.is_dead = true;
                    obj.blocked = false;
                    self.weapon.shot_animation_trigger = false;
                }
                break;
            }
        }
    }

    fn npc_action(&mut self) {
        self.damage_count = 0;
        let (px, py) = (self.player.x, self.player.y);
        for obj in &mut self.sprites {
            if obj.is_dead {
                continue;
            }
            if ray_casting_npc_player(obj.x, obj.y, &self.map, px, py) {
                if self.hurt_count < FPS {
                    self.hurt_count += 1;
                } else {
                    self.player.hurt(0.1);
                    self.damage_count += 1;
                }
                obj.npc_action_trigger = true;
                // npc_move
                if obj.d_sprite.abs() > TILE && obj.aggressive {
                    let dx = obj.x - px;
                    let dy = obj.y - py;
                    obj.x = if dx < 0.0 { obj.x + 1.0 } else { obj.x - 1.0 };
                    obj.y = if dy < 0.0 { obj.y + 1.0 } else { obj.y - 1.0 };
                }
            } else {
                obj.npc_action_trigger = false;
            }
        }
    }

    /// Камера для кадра между тиками: позиция интерполируется, а ещё не
    /// применённое движение мыши добавляется к углу сразу.
    pub fn view(&self, alpha: f64, pending_mouse_dx: i32) -> View {
        let angle = self.player.angle + pending_mouse_dx as f64 * self.player.sensitivity;
        View {
            x: lerp(self.prev_pos.0, self.player.x, alpha),
            y: lerp(self.prev_pos.1, self.player.y, alpha),
            angle: py_mod(angle, PI * 2.0),
            alpha,
        }
    }
}
