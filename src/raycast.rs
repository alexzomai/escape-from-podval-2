use crate::map::WorldMap;
use crate::settings::*;

/// Результат одного луча (`ray_casting.py`).
#[derive(Clone, Copy, Debug)]
pub struct WallHit {
    pub depth: f64,
    /// Символ блока карты; `None` — луч ничего не нашёл (в оригинале ключ `1`).
    pub texture: Option<u8>,
    pub offset: i32,
    pub projection_height: i32,
}

fn surface_hit(v: f64) -> f64 {
    (v / TILE).floor() * TILE
}

pub fn ray_casting(x0: f64, y0: f64, angle: f64, map: &WorldMap) -> Vec<WallHit> {
    let mut walls = Vec::with_capacity(RAYS_NUMBER as usize);
    // как и в оригинале, текстура переживает лучи, которые ничего не нашли
    let (mut texture_v, mut texture_h): (Option<u8>, Option<u8>) = (None, None);
    let (x_square, y_square) = (surface_hit(x0), surface_hit(y0));
    let mut ray_angle = angle - HALF_FOV;
    for _ in 0..RAYS_NUMBER {
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
        let (mut depth_v, mut y_vertical) = (0.0, 0.0);
        for _ in 0..map.width {
            depth_v = (x - x0) / cos_a;
            y_vertical = y0 + depth_v * sin_a;
            if let Some(t) = map.at(x + dx, y_vertical) {
                texture_v = Some(t);
                break;
            }
            x += dx * TILE;
        }

        let (mut y, dy) = if sin_a >= 0.0 {
            (y_square + TILE, 1.0)
        } else {
            (y_square, -1.0)
        };
        let (mut depth_h, mut x_horizontal) = (0.0, 0.0);
        for _ in 0..map.height {
            depth_h = (y - y0) / sin_a;
            x_horizontal = x0 + depth_h * cos_a;
            if let Some(t) = map.at(x_horizontal, y + dy) {
                texture_h = Some(t);
                break;
            }
            y += dy * TILE;
        }

        let (depth, offset, texture) = if depth_v < depth_h {
            (depth_v, y_vertical, texture_v)
        } else {
            (depth_h, x_horizontal, texture_h)
        };
        let offset = (offset as i64).rem_euclid(TILE_SIZE as i64) as i32;
        let depth = (depth * (angle - ray_angle).cos()).max(0.00001);
        let projection_height = (*SCALE_COEFFICIENT / depth) as i32;
        walls.push(WallHit {
            depth,
            texture,
            offset,
            projection_height,
        });
        ray_angle += DELTA_ANGLE;
    }
    walls
}
