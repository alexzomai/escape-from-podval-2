//! Замер скорости отрисовки игрового кадра: `cargo run --release --example bench`.

use escape_from_podval::assets::{Assets, TextCache};
use escape_from_podval::game::Game;
use escape_from_podval::gfx::Canvas;
use escape_from_podval::map::WorldMap;
use escape_from_podval::player::Input;
use escape_from_podval::render::{build_minimap, draw_game};
use escape_from_podval::settings::*;
use std::time::Instant;

fn main() {
    let ttf = Box::leak(Box::new(sdl2::ttf::init().unwrap()));
    let assets = Assets::load().unwrap();
    let mut text = TextCache::new(ttf).unwrap();
    let map = WorldMap::load().unwrap();
    let minimap = build_minimap(&map);
    let mut game = Game::new(map);
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    // немного повернуться, чтобы в кадре были стены разной дальности
    game.tick(&Input {
        mouse_dx: 300,
        focused: true,
        ..Input::default()
    });
    let frames = 2000;
    let start = Instant::now();
    for i in 0..frames {
        let view = game.view((i % 10) as f64 / 10.0, i % 7);
        draw_game(&mut canvas, &assets, &mut text, &minimap, &game, &view, i);
    }
    let per_frame = start.elapsed().as_secs_f64() / frames as f64;
    println!(
        "{:.3} мс на кадр, ~{:.0} FPS (только отрисовка)",
        per_frame * 1000.0,
        1.0 / per_frame
    );
}
