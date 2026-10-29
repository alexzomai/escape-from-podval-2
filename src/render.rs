//! Отрисовка кадров (`render.py`).

use crate::assets::{Assets, TextCache};
use crate::game::{Game, View};
use crate::gfx::{Canvas, Image, WallColumn};
use crate::map::WorldMap;
use crate::raycast::{WallHit, ray_casting};
use crate::rect::Rect;
use crate::settings::*;
use crate::sprites::{SpriteFrame, project, sprite_rect, visible_projection};
use std::cmp::Ordering;

pub fn start_button() -> Rect {
    Rect::centered(HALF_WIDTH, HALF_HEIGHT, 400, 150)
}

pub fn exit_button() -> Rect {
    Rect::centered(HALF_WIDTH, HALF_HEIGHT + 200, 400, 150)
}

/// Миникарта рисуется один раз: в оригинале она каждый кадр одинаковая.
pub fn build_minimap(map: &WorldMap) -> Image {
    let mut surface = Canvas::new(MAP_TILE * map.width, MAP_TILE * map.height);
    surface.fill_rect(Rect::new(0, 0, surface.w, surface.h), GRAY);
    for (tx, ty) in map.blocks() {
        surface.fill_rect(
            Rect::new(tx * MAP_TILE, ty * MAP_TILE, MAP_TILE, MAP_TILE),
            GREEN,
        );
    }
    Image::from_pixels(surface.w, surface.h, surface.px)
}

fn wall_column<'a>(assets: &'a Assets, wall: &WallHit) -> WallColumn<'a> {
    let texture = wall
        .texture
        .filter(|t| assets.walls.contains_key(t))
        .unwrap_or(b'*');
    let tex = &assets.walls[&texture];
    let ph = wall.projection_height;
    let tx = wall.offset * TEXTURE_SCALE;
    if ph > HEIGHT {
        let co = ph as f64 / HEIGHT as f64;
        let texture_height = TEXTURE_HEIGHT as f64 / co;
        let sy = (HALF_TEXTURE_HEIGHT as f64 - (texture_height / 2.0).floor()) as i32;
        WallColumn {
            tex,
            tx,
            sy,
            sh: texture_height as i32,
            top: 0,
            h: HEIGHT,
        }
    } else {
        WallColumn {
            tex,
            tx,
            sy: 0,
            sh: TEXTURE_HEIGHT,
            top: HALF_HEIGHT - ph / 2,
            h: ph,
        }
    }
}

fn sprite_image(assets: &Assets, frame: SpriteFrame) -> &Image {
    match frame {
        SpriteFrame::Idle => &assets.enemy,
        SpriteFrame::Action(i) => &assets.enemy_action[i as usize],
        SpriteFrame::Death(i) => &assets.enemy_death[i as usize],
    }
}

/// Игровой кадр: фон, стены и спрайты, оружие, HUD.
pub fn draw_game(
    c: &mut Canvas,
    assets: &Assets,
    text: &mut TextCache,
    minimap: &Image,
    game: &Game,
    view: &View,
    fps: i32,
) {
    // background + vision
    let (tx, ty) = (view.x / TILE, view.y / TILE);
    let (sky, floor) = if ty > 44.0 && tx < 42.0 {
        (SKY, FLOOR)
    } else {
        (SKY2, FLOOR2)
    };
    let walls = ray_casting(view.x, view.y, view.angle, &game.map);
    let columns: Vec<WallColumn> = walls.iter().map(|w| wall_column(assets, w)).collect();
    c.walls_and_background::<{ SCALE as usize }>(&columns, HALF_HEIGHT, sky, floor);

    // Оригинал рисует стены и спрайты вместе от дальних к ближним. Стены уже на экране,
    // поэтому спрайт идёт от дальнего к ближнему и не рисуется там, где его закрыл бы
    // столбец более близкой стены. При равной дальности стена в оригинале рисуется раньше.
    let mut sprites: Vec<(f64, usize, i32, i32)> = Vec::new();
    for (index, sprite) in game.sprites.iter().enumerate() {
        let (sx, sy) = sprite.render_pos(view.alpha);
        let (depth, ray) = project(sx, sy, view.x, view.y, view.angle);
        if let Some(projection_height) = visible_projection(depth, ray, &walls) {
            sprites.push((depth, index, ray, projection_height));
        }
    }
    sprites.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    for &(depth, index, ray, projection_height) in &sprites {
        if depth == 0.0 {
            continue;
        }
        let sprite = &game.sprites[index];
        let (x, y, w, h) = sprite_rect(projection_height, ray, sprite.snap.dead);
        let hidden = |px: i32, py: i32| {
            let column = (px / SCALE) as usize;
            walls[column].depth < depth && columns[column].covers(py)
        };
        c.blit_scaled_alpha(
            sprite_image(assets, sprite.render_frame()),
            x,
            y,
            w,
            h,
            |px, py| !hidden(px, py),
        );
    }

    // weapon
    let weapon = match game.weapon.display {
        Some(i) => &assets.weapon_animation[i as usize],
        None => &assets.weapon_base,
    };
    c.blit_alpha(
        weapon,
        HALF_WIDTH - assets.weapon_base.w / 2,
        HEIGHT - assets.weapon_base.h,
    );

    for _ in 0..game.damage_count {
        c.overlay((255, 0, 0), 15);
    }

    c.blit_opaque(minimap, MAP_POS.0, MAP_POS.1);
    c.blit_alpha(
        text.render(40, &fps.to_string(), YELLOW, false),
        FPS_POS.0,
        FPS_POS.1,
    );
    let hp = format!("> {} <", game.player.hp as i64);
    c.blit_alpha(text.render(40, &hp, RED, false), HP_POS.0, HP_POS.1);
    c.crosshair(HALF_WIDTH, HALF_HEIGHT, LIGHT_GREEN);
    let enemies = format!("ENEMIES: {}", game.enemies);
    c.blit_alpha(
        text.render(25, &enemies, WHITE, false),
        ENEMIES_POS.0,
        ENEMIES_POS.1,
    );
}

fn draw_button(
    c: &mut Canvas,
    text: &mut TextCache,
    rect: Rect,
    label: &str,
    dx: i32,
    hover: bool,
) {
    let pos = (rect.centerx() - dx, rect.centery() - 35);
    if hover {
        c.fill_rect(rect, BUTTON_HOVER);
    } else {
        c.rect_border(rect, 10, BUTTON);
    }
    c.blit_alpha(text.render(72, label, BUTTON, true), pos.0, pos.1);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    None,
    Start,
    Exit,
}

/// Главное меню; `x` — сдвиг прокрутки фона.
pub fn draw_menu(
    c: &mut Canvas,
    assets: &Assets,
    text: &mut TextCache,
    x: i64,
    mouse: (i32, i32),
) -> Hover {
    let area = Rect::new(
        x.rem_euclid(WIDTH as i64) as i32,
        HALF_HEIGHT,
        WIDTH,
        HEIGHT,
    );
    c.blit_opaque_area(&assets.menu_picture, 0, 0, area);
    let (start, exit) = (start_button(), exit_button());
    draw_button(c, text, start, "START", 130, false);
    draw_button(c, text, exit, "EXIT", 85, false);
    c.blit_alpha(
        &assets.label,
        (HALF_WIDTH as f64 - assets.label.w as f64 / 2.0) as i32,
        100,
    );
    if start.collidepoint(mouse) {
        // в оригинале поверх рамки рисуется заливка, а затем снова текст
        draw_button(c, text, start, "START", 130, true);
        Hover::Start
    } else if exit.collidepoint(mouse) {
        draw_button(c, text, exit, "EXIT", 85, true);
        Hover::Exit
    } else {
        Hover::None
    }
}

/// Экран победы или поражения.
pub fn draw_end(c: &mut Canvas, text: &mut TextCache, won: bool, mouse: (i32, i32)) -> Hover {
    c.fill_rect(Rect::new(0, 0, WIDTH, HEIGHT), BLACK);
    let exit = exit_button();
    draw_button(c, text, exit, "EXIT", 85, false);
    let title = if won {
        "You have won !!!"
    } else {
        "You have lost ..."
    };
    c.blit_alpha(
        text.render(72, title, BUTTON, true),
        HALF_WIDTH - 300,
        HALF_HEIGHT - 150,
    );
    if exit.collidepoint(mouse) {
        draw_button(c, text, exit, "EXIT", 85, true);
        Hover::Exit
    } else {
        Hover::None
    }
}
