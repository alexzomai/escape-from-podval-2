use crate::embedded;
use crate::gfx::Image;
use crate::settings::Rgb;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rwops::RWops;
use sdl2::ttf::{Font, Sdl2TtfContext};
use std::collections::HashMap;
use std::path::Path;

/// Картинка из файла на диске.
pub fn load_image(path: &Path, alpha: bool) -> Result<Image, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_image(&bytes, alpha).map_err(|e| format!("{}: {e}", path.display()))
}

/// Декодирование картинки. `alpha = false` — как `.convert()`: альфа отбрасывается, RGB остаётся.
pub fn decode_image(bytes: &[u8], alpha: bool) -> Result<Image, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let px = img
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0;
            let a = if alpha { a } else { 255 };
            (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32
        })
        .collect();
    Ok(Image::from_pixels(w as i32, h as i32, px))
}

pub struct Assets {
    /// Текстуры стен по символу карты.
    pub walls: HashMap<u8, Image>,
    pub weapon_base: Image,
    /// gun1..gun5
    pub weapon_animation: Vec<Image>,
    pub enemy: Image,
    /// enemy0..enemy3
    pub enemy_action: Vec<Image>,
    /// death0..death4
    pub enemy_death: Vec<Image>,
    pub menu_picture: Image,
    pub label: Image,
}

impl Assets {
    pub fn load() -> Result<Assets, String> {
        let img = |p: &str, alpha: bool| {
            let path = format!("resources/{p}");
            decode_image(embedded::file(&path), alpha).map_err(|e| format!("{path}: {e}"))
        };
        let mut walls = HashMap::new();
        for (key, file) in [
            (b'B', "brick"),
            (b'C', "cage"),
            (b'S', "computer"),
            (b'D', "darkbrick"),
            (b'c', "darkbrick_caged"),
            (b'd', "darkstone"),
            (b'O', "door"),
            (b'*', "freedom"),
            (b'I', "iron"),
            (b'G', "greybrick"),
            (b'P', "plate"),
        ] {
            walls.insert(key, img(&format!("walls/{file}.png"), false)?);
        }
        Ok(Assets {
            walls,
            weapon_base: img("gun/gun0.png", true)?,
            weapon_animation: (1..6)
                .map(|i| img(&format!("gun/gun{i}.png"), true))
                .collect::<Result<_, _>>()?,
            enemy: img("enemy/enemy.png", true)?,
            enemy_action: (0..4)
                .map(|i| img(&format!("enemy/enemy{i}.png"), true))
                .collect::<Result<_, _>>()?,
            enemy_death: (0..5)
                .map(|i| img(&format!("enemy_dead/death{i}.png"), true))
                .collect::<Result<_, _>>()?,
            menu_picture: img("bg.jpg", false)?,
            label: img("label.png", true)?,
        })
    }
}

type TextKey = (u16, String, Rgb, bool);

/// Рендер текста через SDL_ttf — ту же библиотеку, что использует `pygame.font`.
pub struct TextCache {
    fonts: HashMap<u16, Font<'static, 'static>>,
    cache: HashMap<TextKey, Image>,
}

impl TextCache {
    pub fn new(ttf: &'static Sdl2TtfContext) -> Result<TextCache, String> {
        let mut fonts = HashMap::new();
        for size in [72, 40, 25] {
            let data = RWops::from_bytes(embedded::file("resources/font.ttf"))?;
            fonts.insert(size, ttf.load_font_from_rwops(data, size)?);
        }
        Ok(TextCache {
            fonts,
            cache: HashMap::new(),
        })
    }

    /// `font.render(text, antialias, color)`.
    pub fn render(&mut self, size: u16, text: &str, color: Rgb, antialias: bool) -> &Image {
        // счётчик FPS каждый кадр новый — не даём кэшу расти бесконечно
        if self.cache.len() > 512 {
            self.cache.clear();
        }
        let key = (size, text.to_string(), color, antialias);
        if !self.cache.contains_key(&key) {
            let font = &self.fonts[&size];
            let image =
                render_text(font, text, color, antialias).unwrap_or_else(|_| Image::new(0, 0, 0));
            self.cache.insert(key.clone(), image);
        }
        &self.cache[&key]
    }
}

fn render_text(font: &Font, text: &str, (r, g, b): Rgb, antialias: bool) -> Result<Image, String> {
    let color = Color::RGB(r, g, b);
    let partial = font.render(text);
    let surface = if antialias {
        partial.blended(color)
    } else {
        partial.solid(color)
    }
    .map_err(|e| e.to_string())?;
    let (w, h) = (surface.width() as i32, surface.height() as i32);
    let argb = surface.convert_format(PixelFormatEnum::ARGB8888)?;
    let pitch = argb.pitch() as usize;
    let mut px: Vec<u32> = argb.with_lock(|bytes| {
        (0..h as usize)
            .flat_map(|y| {
                (0..w as usize).map(move |x| {
                    let o = y * pitch + x * 4;
                    u32::from_ne_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
                })
            })
            .collect()
    });
    if !antialias {
        // Solid-рендер — 8-битная поверхность с colorkey: прозрачен только ключевой индекс.
        let mut key = 0u32;
        // SAFETY: surface живая, указатель на key валиден на время вызова.
        let has_key = unsafe { sdl2::sys::SDL_GetColorKey(surface.raw(), &mut key) } == 0;
        let pitch = surface.pitch() as usize;
        surface.with_lock(|bytes| {
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let p = &mut px[y * w as usize + x];
                    let transparent = has_key && bytes[y * pitch + x] as u32 == key;
                    *p = if transparent {
                        *p & 0x00FF_FFFF
                    } else {
                        *p | 0xFF00_0000
                    };
                }
            }
        });
    }
    Ok(Image::from_pixels(w, h, px))
}
