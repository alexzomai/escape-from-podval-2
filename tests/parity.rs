//! Сверка с оригиналом: тот же ввод, что получал бот в `tests/reference/harness.py`,
//! должен дать то же состояние игры и тот же экран в каждом кадре.
//! Другой набор эталонов можно подставить через `PARITY_DIR`.

use escape_from_podval::assets::{Assets, TextCache, load_image};
use escape_from_podval::game::{Game, Outcome};
use escape_from_podval::gfx::{Canvas, Image};
use escape_from_podval::map::WorldMap;
use escape_from_podval::player::Input;
use escape_from_podval::render::{build_minimap, draw_end, draw_game, draw_menu};
use escape_from_podval::settings::*;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn data_dir() -> PathBuf {
    std::env::var_os("PARITY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("tests/data/parity"))
}

fn ttf() -> &'static sdl2::ttf::Sdl2TtfContext {
    static TTF: OnceLock<usize> = OnceLock::new();
    let ptr =
        *TTF.get_or_init(|| Box::leak(Box::new(sdl2::ttf::init().unwrap())) as *const _ as usize);
    // SAFETY: контекст утёк и живёт до конца процесса.
    unsafe { &*(ptr as *const sdl2::ttf::Sdl2TtfContext) }
}

/// CRC32 экрана в формате `pygame.image.tostring(screen, 'RGB')`.
fn screen_crc(c: &Canvas) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    let mut row = Vec::with_capacity(c.w as usize * 3);
    for line in c.px.chunks(c.w as usize) {
        row.clear();
        row.extend(
            line.iter()
                .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8]),
        );
        hasher.update(&row);
    }
    hasher.finalize()
}

fn rgb_bytes(c: &Canvas) -> Vec<u8> {
    c.px.iter()
        .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8])
        .collect()
}

/// Число отличающихся пикселей и максимальная разница канала.
fn diff(c: &Canvas, reference: &Image) -> (usize, u32) {
    let mut count = 0;
    let mut max = 0;
    for (&a, &b) in c.px.iter().zip(&reference.px) {
        if a & 0xFFFFFF != b & 0xFFFFFF {
            count += 1;
            for shift in [0, 8, 16] {
                max = max.max(((a >> shift) & 0xFF).abs_diff((b >> shift) & 0xFF));
            }
        }
    }
    (count, max)
}

fn save(c: &Canvas, name: &str) -> PathBuf {
    let dir = root().join("target/parity");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    image::RgbImage::from_raw(c.w as u32, c.h as u32, rgb_bytes(c))
        .unwrap()
        .save(&path)
        .unwrap();
    path
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .map(str::to_string)
        .collect()
}

fn state_line(game: &Game, frame: usize, sounds: u32) -> Vec<String> {
    let p = &game.player;
    let mut parts = vec![
        frame.to_string(),
        p.x.to_string(),
        p.y.to_string(),
        p.angle.to_string(),
        p.hp.to_string(),
        (p.shot as u8).to_string(),
        sounds.to_string(),
    ];
    for s in &game.sprites {
        parts.extend([
            s.x.to_string(),
            s.y.to_string(),
            s.d_sprite.to_string(),
            s.ray.to_string(),
            (s.is_dead as u8).to_string(),
            (s.npc_action_trigger as u8).to_string(),
            s.projection_height.to_string(),
        ]);
    }
    parts
}

/// Поля сравниваются как числа: репрезентации float в Python и Rust пишутся по-разному.
fn same_value(a: &str, b: &str) -> bool {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(x), Ok(y)) => x.to_bits() == y.to_bits() || x == y,
        _ => a == b,
    }
}

#[test]
fn gameplay_matches_original() {
    let dir = data_dir();
    let inputs = read_lines(&dir.join("inputs.txt"));
    let states = read_lines(&dir.join("states.txt"));
    let crcs = read_lines(&dir.join("crc.txt"));
    let result = read_lines(&dir.join("result.txt"))[0].clone();

    let assets = Assets::load(&root()).unwrap();
    let mut text = TextCache::new(ttf(), &root()).unwrap();
    let map = WorldMap::load(&root()).unwrap();
    let minimap = build_minimap(&map);
    let mut game = Game::new(map);
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut sounds = 0;
    let mut next_state = 0;
    let mut frame_mismatches = Vec::new();

    for (frame, line) in inputs.iter().enumerate() {
        let v: Vec<i32> = line.split(' ').map(|t| t.parse().unwrap()).collect();
        let input = Input {
            forward: v[0] & 1 != 0,
            back: v[0] & 2 != 0,
            left: v[0] & 4 != 0,
            right: v[0] & 8 != 0,
            escape: false,
            mouse_dx: v[1],
            focused: true,
            click: v[2] != 0,
        };
        let (outcome, events) = game.tick(&input);
        sounds += events.shot_sound as u32;
        match outcome {
            Outcome::Continue => {}
            Outcome::Won | Outcome::Lost => {
                let expected = if outcome == Outcome::Won {
                    "WON"
                } else {
                    "LOST"
                };
                assert_eq!(
                    result,
                    format!("{expected} {frame}"),
                    "игра закончилась иначе"
                );
                assert_eq!(frame, crcs.len());
                break;
            }
            Outcome::Quit => unreachable!(),
        }

        // состояние в эталоне записано не для каждого кадра
        if let Some(line) = states
            .get(next_state)
            .filter(|l| l.split(' ').next() == Some(&frame.to_string()))
        {
            next_state += 1;
            let expected: Vec<&str> = line.split(' ').collect();
            let actual = state_line(&game, frame, sounds);
            assert_eq!(expected.len(), actual.len(), "кадр {frame}: число полей");
            for (i, (e, a)) in expected.iter().zip(&actual).enumerate() {
                assert!(
                    same_value(e, a),
                    "кадр {frame}, поле {i}: оригинал {e}, порт {a}\n{line}"
                );
            }
        }

        draw_game(
            &mut canvas,
            &assets,
            &mut text,
            &minimap,
            &game,
            &game.view(1.0, 0),
            60,
        );
        let crc = format!("{:08x}", screen_crc(&canvas));
        if crc != crcs[frame] {
            let png = dir.join(format!("frames/{frame:05}.png"));
            let detail = if png.exists() {
                let (n, max) = diff(&canvas, &load_image(&png, false).unwrap());
                format!("{n} пикселей, макс. разница {max}")
            } else {
                String::new()
            };
            if frame_mismatches.len() < 5 {
                save(&canvas, &format!("{frame:05}.png"));
            }
            frame_mismatches.push(format!("кадр {frame}: {detail}"));
        }
    }
    assert_eq!(
        next_state,
        states.len(),
        "проверены не все эталонные состояния"
    );
    assert!(
        frame_mismatches.is_empty(),
        "{} кадров отличаются от оригинала (первые сохранены в target/parity):\n{}",
        frame_mismatches.len(),
        frame_mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Меню и экраны победы/поражения. Фон меню — JPEG: декодер Pillow, которым сняты
/// эталоны, и декодер `image` расходятся на единицы яркости в ~0.5% пикселей,
/// поэтому для меню допускается разница до 3.
#[test]
fn screens_match_original() {
    let dir = data_dir();
    let assets = Assets::load(&root()).unwrap();
    let mut text = TextCache::new(ttf(), &root()).unwrap();
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let positions = [
        ("none", (0, 0)),
        ("start", (HALF_WIDTH, HALF_HEIGHT)),
        ("exit", (HALF_WIDTH, HALF_HEIGHT + 200)),
    ];
    let mut failures = Vec::new();
    let mut check = |canvas: &Canvas, name: &str, tolerance: u32| {
        let reference = load_image(&dir.join(name), false).unwrap();
        let (n, max) = diff(canvas, &reference);
        if max > tolerance {
            failures.push(format!(
                "{name}: {n} пикселей, макс. разница {max} (сохранено в {})",
                save(canvas, name).display()
            ));
        }
    };
    for (tag, mouse) in [positions[1], positions[2]] {
        draw_menu(&mut canvas, &assets, &mut text, 12, mouse);
        check(&canvas, &format!("menu_{tag}.png"), 3);
    }
    for (tag, mouse) in [positions[0], positions[2]] {
        for (kind, won) in [("win", true), ("lose", false)] {
            draw_end(&mut canvas, &mut text, won, mouse);
            check(&canvas, &format!("{kind}_{tag}.png"), 0);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
