use escape_from_podval::assets::{Assets, TextCache, resource_root};
use escape_from_podval::audio::Audio;
use escape_from_podval::game::{Game, Outcome};
use escape_from_podval::gfx::{Canvas, Image};
use escape_from_podval::map::WorldMap;
use escape_from_podval::player::Input;
use escape_from_podval::render::{Hover, build_minimap, draw_end, draw_game, draw_menu};
use escape_from_podval::settings::*;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, WindowCanvas};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_nanos(1_000_000_000 / FPS as u64);
/// Если кадр завис надолго, не пытаемся догнать больше этого.
const MAX_CATCH_UP: Duration = Duration::from_millis(250);

struct Screen<'a> {
    canvas: WindowCanvas,
    texture: Texture<'a>,
    frame: Canvas,
}

impl Screen<'_> {
    fn present(&mut self) -> Result<(), String> {
        self.texture
            .update(None, self.frame.as_bytes(), WIDTH as usize * 4)
            .map_err(|e| e.to_string())?;
        self.canvas.copy(&self.texture, None, None)?;
        self.canvas.present();
        Ok(())
    }
}

/// Счётчик как у `pygame.time.Clock.get_fps()`: пересчитывается каждые 10 кадров.
struct FpsCounter {
    count: u32,
    since: Instant,
    fps: f64,
}

impl FpsCounter {
    fn new() -> Self {
        FpsCounter {
            count: 0,
            since: Instant::now(),
            fps: 0.0,
        }
    }

    fn tick(&mut self) {
        self.count += 1;
        if self.count >= 10 {
            let elapsed = self.since.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                self.fps = self.count as f64 / elapsed;
            }
            self.count = 0;
            self.since = Instant::now();
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // `--bench N`: без меню, звука и захвата мыши N секунд гоняет игру и печатает средний FPS
    let args: Vec<String> = std::env::args().collect();
    let bench = match args.iter().position(|a| a == "--bench") {
        Some(i) => {
            let secs: f64 = args
                .get(i + 1)
                .and_then(|s| s.parse().ok())
                .ok_or("использование: --bench <секунд>")?;
            Some(Duration::from_secs_f64(secs))
        }
        None => None,
    };
    let root = resource_root().ok_or(
        "не найдены resources/ и maps/ (запускайте из папки игры или задайте PODVAL_ROOT)",
    )?;
    // кадры не ждут вертикальной синхронизации — FPS не ограничен
    sdl2::hint::set("SDL_RENDER_VSYNC", "0");
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let ttf: &'static _ = Box::leak(Box::new(sdl2::ttf::init().map_err(|e| e.to_string())?));
    let window = video
        .window(TITLE, WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let canvas = window
        .into_canvas()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB888, WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;
    let mut screen = Screen {
        canvas,
        texture,
        frame: Canvas::new(WIDTH, HEIGHT),
    };

    let assets = Assets::load(&root)?;
    let mut text = TextCache::new(ttf, &root)?;
    let map = WorldMap::load(&root)?;
    let minimap = build_minimap(&map);
    let mut audio = if bench.is_some() {
        Audio::silent()
    } else {
        Audio::new(&root)
    };
    let mut events = sdl.event_pump()?;
    let mouse = sdl.mouse();

    if let Some(duration) = bench {
        let start = Instant::now();
        let mut frames = 0u64;
        play(
            &mut screen,
            &mut events,
            &assets,
            &mut text,
            &minimap,
            &audio,
            Game::new(map),
            Some((duration, &mut frames)),
        )?;
        let secs = start.elapsed().as_secs_f64();
        println!(
            "{frames} кадров за {secs:.1} с: {:.0} FPS",
            frames as f64 / secs
        );
        return Ok(());
    }

    // main_menu
    audio.play_music("mainmenu.mp3", 10);
    let menu_start = Instant::now();
    loop {
        for event in events.poll_iter() {
            if let Event::Quit { .. } = event {
                return Ok(());
            }
        }
        // фон сдвигается на пиксель за кадр меню (20 FPS в оригинале)
        let x = 12 + (menu_start.elapsed().as_secs_f64() * 20.0) as i64;
        let state = events.mouse_state();
        let hover = draw_menu(
            &mut screen.frame,
            &assets,
            &mut text,
            x,
            (state.x(), state.y()),
        );
        screen.present()?;
        match hover {
            Hover::Start if state.left() => break,
            Hover::Exit if state.left() => return Ok(()),
            _ => {}
        }
    }

    mouse.set_relative_mouse_mode(true);
    audio.play_music("theme.mp3", 10);
    let outcome = play(
        &mut screen,
        &mut events,
        &assets,
        &mut text,
        &minimap,
        &audio,
        Game::new(map),
        None,
    )?;
    let won = match outcome {
        Outcome::Won => true,
        Outcome::Lost => false,
        Outcome::Quit | Outcome::Continue => return Ok(()),
    };

    mouse.set_relative_mouse_mode(false);
    mouse.show_cursor(true);
    audio.play_music(if won { "win.mp3" } else { "loss.mp3" }, 0);
    loop {
        for event in events.poll_iter() {
            if let Event::Quit { .. } = event {
                return Ok(());
            }
        }
        let state = events.mouse_state();
        let hover = draw_end(&mut screen.frame, &mut text, won, (state.x(), state.y()));
        screen.present()?;
        if hover == Hover::Exit && state.left() {
            return Ok(());
        }
    }
}

/// Игровой цикл: логика фиксированным шагом 60 Гц, отрисовка — так часто, как получится.
/// `bench` — ограничение по времени и счётчик отрисованных кадров.
#[allow(clippy::too_many_arguments)]
fn play(
    screen: &mut Screen,
    events: &mut sdl2::EventPump,
    assets: &Assets,
    text: &mut TextCache,
    minimap: &Image,
    audio: &Audio,
    mut game: Game,
    mut bench: Option<(Duration, &mut u64)>,
) -> Result<Outcome, String> {
    let started = Instant::now();
    let mut fps = FpsCounter::new();
    let mut pending_dx = 0i32;
    let mut pending_click = false;
    let mut lag = Duration::ZERO;
    let mut last = Instant::now();
    loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(Outcome::Quit),
                Event::MouseMotion { xrel, .. } => pending_dx += xrel,
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    ..
                } => pending_click = true,
                _ => {}
            }
        }

        let now = Instant::now();
        lag = (lag + (now - last)).min(MAX_CATCH_UP);
        last = now;
        while lag >= TICK {
            lag -= TICK;
            let keys = events.keyboard_state();
            let input = Input {
                forward: keys.is_scancode_pressed(Scancode::W),
                back: keys.is_scancode_pressed(Scancode::S),
                left: keys.is_scancode_pressed(Scancode::A),
                right: keys.is_scancode_pressed(Scancode::D),
                escape: keys.is_scancode_pressed(Scancode::Escape),
                mouse_dx: std::mem::take(&mut pending_dx),
                focused: true,
                click: std::mem::take(&mut pending_click),
            };
            let (outcome, tick_events) = game.tick(&input);
            if tick_events.shot_sound {
                audio.play_shot();
            }
            if outcome != Outcome::Continue {
                return Ok(outcome);
            }
        }

        let alpha = lag.as_secs_f64() / TICK.as_secs_f64();
        let view = game.view(alpha, pending_dx);
        draw_game(
            &mut screen.frame,
            assets,
            text,
            minimap,
            &game,
            &view,
            fps.fps as i32,
        );
        screen.present()?;
        fps.tick();
        if let Some((limit, frames)) = &mut bench {
            **frames += 1;
            if started.elapsed() >= *limit {
                return Ok(Outcome::Quit);
            }
        }
    }
}
