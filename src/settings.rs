use std::f64::consts::PI;
use std::sync::LazyLock;

// окно
pub const WIDTH: i32 = 1152;
pub const HEIGHT: i32 = 864;
pub const HALF_WIDTH: i32 = WIDTH / 2;
pub const HALF_HEIGHT: i32 = HEIGHT / 2;
pub const DOUBLE_HEIGHT: i32 = 2 * HEIGHT;
/// Частота игровой логики. Отрисовка не ограничена, но вся механика оригинала
/// (скорость, анимации, урон) завязана на 60 кадров в секунду, поэтому логика
/// работает фиксированным шагом 60 Гц.
pub const FPS: i32 = 60;
pub const TILE_SIZE: i32 = 64;
pub const TILE: f64 = TILE_SIZE as f64;
pub const MUSIC_VOLUME: f32 = 0.25;
pub const TITLE: &str = "ESCAPE FROM PODVAL";

// игрок
pub const PLAYER_POS: (f64, f64) = (8.5 * TILE, 70.5 * TILE);
pub const PLAYER_ANGLE: f64 = 4.714;
pub const PLAYER_SPEED: f64 = 6.0;

// миникарта
pub const MINIMAP_SCALE: i32 = 10;
pub const MAP_SCALE: i32 = 2 * MINIMAP_SCALE;
pub const MAP_TILE: i32 = TILE_SIZE / MAP_SCALE;
pub const MAP_POS: (i32, i32) = (WIDTH - 220, 20);

// камера
pub const FOV: f64 = PI / 3.0;
pub const HALF_FOV: f64 = FOV / 2.0;
pub const RAYS_NUMBER: i32 = WIDTH / 6;
pub const DELTA_ANGLE: f64 = FOV / RAYS_NUMBER as f64;
pub const COEFFICIENT: f64 = 6.0;
pub static SCALE_COEFFICIENT: LazyLock<f64> = LazyLock::new(|| {
    let distance = RAYS_NUMBER as f64 / (2.0 * HALF_FOV.tan());
    COEFFICIENT * distance * TILE
});
pub const SCALE: i32 = WIDTH / RAYS_NUMBER;
pub const FPS_POS: (i32, i32) = (5, 5);
pub const HP_POS: (i32, i32) = (15, HEIGHT - 55);
pub const ENEMIES_POS: (i32, i32) = (WIDTH - 219, 275);

// спрайты
pub const CENTER_RAY: i32 = RAYS_NUMBER / 2 - 1;

// текстуры (64x64)
pub const TEXTURE_WIDTH: i32 = 64;
pub const TEXTURE_HEIGHT: i32 = 64;
pub const HALF_TEXTURE_HEIGHT: i32 = TEXTURE_HEIGHT / 2;
pub const TEXTURE_SCALE: i32 = TEXTURE_WIDTH / TILE_SIZE;

// цвета
pub type Rgb = (u8, u8, u8);
pub const WHITE: Rgb = (255, 255, 255);
pub const BLACK: Rgb = (0, 0, 0);
pub const RED: Rgb = (220, 0, 0);
pub const GREEN: Rgb = (0, 80, 0);
pub const LIGHT_GREEN: Rgb = (0, 255, 0);
pub const GRAY: Rgb = (40, 40, 40);
pub const SKY: Rgb = (23, 0, 3);
pub const FLOOR: Rgb = (48, 24, 28);
pub const FLOOR2: Rgb = (107, 111, 110);
pub const SKY2: Rgb = (59, 59, 59);
pub const YELLOW: Rgb = (220, 220, 0);
pub const BUTTON: Rgb = (68, 164, 104);
pub const BUTTON_HOVER: Rgb = (200, 102, 24);
