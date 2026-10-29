//! Программная отрисовка с той же арифметикой, что у pygame/SDL.

use crate::rect::Rect;
use crate::settings::Rgb;

/// Картинка в формате ARGB (`a << 24 | r << 16 | g << 8 | b`).
#[derive(Clone, Debug)]
pub struct Image {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
    /// Для каждой строки — диапазон столбцов с ненулевой альфой (остальное прозрачно).
    rows: Vec<(i32, i32)>,
}

impl Image {
    pub fn new(w: i32, h: i32, fill: u32) -> Image {
        Image::from_pixels(w, h, vec![fill; (w.max(0) * h.max(0)) as usize])
    }

    pub fn from_pixels(w: i32, h: i32, px: Vec<u32>) -> Image {
        let rows = (0..h)
            .map(|y| {
                let row = &px[(y * w) as usize..((y + 1) * w) as usize];
                match row.iter().position(|p| p >> 24 != 0) {
                    Some(first) => (
                        first as i32,
                        w - row.iter().rev().position(|p| p >> 24 != 0).unwrap() as i32,
                    ),
                    None => (0, 0),
                }
            })
            .collect();
        Image { w, h, px, rows }
    }

    pub fn at(&self, x: i32, y: i32) -> u32 {
        self.px[(y * self.w + x) as usize]
    }
}

pub const fn rgb(c: Rgb) -> u32 {
    0xFF00_0000 | (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32
}

/// `ALPHA_BLEND_COMP` из pygame — так смешиваются поверхности с попиксельной альфой.
#[inline]
fn pygame_blend_channel(s: u32, d: u32, a: u32) -> u32 {
    ((((s as i32 - d as i32) * a as i32 + s as i32) >> 8) + d as i32) as u32
}

#[inline]
fn blend_with(dst: u32, src: u32, a: u32, channel: fn(u32, u32, u32) -> u32) -> u32 {
    let r = channel(src >> 16 & 0xFF, dst >> 16 & 0xFF, a);
    let g = channel(src >> 8 & 0xFF, dst >> 8 & 0xFF, a);
    let b = channel(src & 0xFF, dst & 0xFF, a);
    0xFF00_0000 | r << 16 | g << 8 | b
}

#[inline]
fn blend_pixel(dst: &mut u32, src: u32) {
    match src >> 24 {
        0 => {}
        255 => *dst = src,
        a => *dst = blend_with(*dst, src, a, pygame_blend_channel),
    }
}

/// Индексы исходных пикселей при растяжении `src` -> `dst` (алгоритм `stretch` из pygame.transform.scale).
pub fn stretch_indices(src: i32, dst: i32) -> Vec<i32> {
    let mut out = Vec::with_capacity(dst.max(0) as usize);
    let (src2, dst2) = (src * 2, dst * 2);
    let mut err = src2 - dst2;
    let mut i = 0;
    for _ in 0..dst {
        out.push(i);
        while err >= 0 {
            i += 1;
            err -= dst2;
        }
        err += src2;
    }
    out
}

/// Столбец стены на экране: строки `top..top + h` растягивают строки текстуры `sy..sy + sh`
/// из её колонки `tx`.
pub struct WallColumn<'a> {
    pub tex: &'a Image,
    pub tx: i32,
    pub sy: i32,
    pub sh: i32,
    pub top: i32,
    pub h: i32,
}

impl WallColumn<'_> {
    pub fn covers(&self, y: i32) -> bool {
        self.sh > 0 && y >= self.top && y < self.top + self.h
    }
}

/// Экран: тот же формат, что и `Image`, альфа всегда 255.
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas {
            w,
            h,
            px: vec![0xFF00_0000; (w * h) as usize],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: u32 не имеет недопустимых битовых представлений, выравнивание u8 = 1.
        unsafe { std::slice::from_raw_parts(self.px.as_ptr().cast::<u8>(), self.px.len() * 4) }
    }

    pub fn at(&self, x: i32, y: i32) -> u32 {
        self.px[(y * self.w + x) as usize]
    }

    /// Пересечение прямоугольника с экраном.
    fn clip(&self, x: i32, y: i32, w: i32, h: i32) -> Option<(i32, i32, i32, i32)> {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.w), (y + h).min(self.h));
        (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
    }

    pub fn fill_rect(&mut self, r: Rect, color: Rgb) {
        let Some((x0, y0, x1, y1)) = self.clip(r.x, r.y, r.w, r.h) else {
            return;
        };
        let c = rgb(color);
        for y in y0..y1 {
            let row = (y * self.w) as usize;
            self.px[row + x0 as usize..row + x1 as usize].fill(c);
        }
    }

    /// `pygame.draw.rect(..., width=n)`: рамка внутрь прямоугольника.
    pub fn rect_border(&mut self, r: Rect, width: i32, color: Rgb) {
        self.fill_rect(Rect::new(r.x, r.y, r.w, width), color);
        self.fill_rect(Rect::new(r.x, r.bottom() - width, r.w, width), color);
        self.fill_rect(Rect::new(r.x, r.y + width, width, r.h - 2 * width), color);
        self.fill_rect(
            Rect::new(r.right() - width, r.y + width, width, r.h - 2 * width),
            color,
        );
    }

    /// Полупрозрачная заливка всего экрана (`Surface.set_alpha` + `blit`).
    pub fn overlay(&mut self, color: Rgb, alpha: u32) {
        // ALPHA_BLEND_CHANNEL из SDL: (s - d) * a + d * 255 == s * a + d * (255 - a) >= 0,
        // записанная без ветвлений, чтобы цикл векторизовался
        let base = |s: u8| s as u32 * alpha + 1;
        let (r0, g0, b0) = (base(color.0), base(color.1), base(color.2));
        let k = 255 - alpha;
        let channel = |x0: u32, d: u32| {
            let x = x0 + d * k;
            (x + (x >> 8)) >> 8
        };
        for p in &mut self.px {
            let d = *p;
            let r = channel(r0, d >> 16 & 0xFF);
            let g = channel(g0, d >> 8 & 0xFF);
            let b = channel(b0, d & 0xFF);
            *p = 0xFF00_0000 | r << 16 | g << 8 | b;
        }
    }

    /// Непрозрачный блит области `src` картинки (`convert()`-поверхности).
    pub fn blit_opaque_area(&mut self, img: &Image, x: i32, y: i32, src: Rect) {
        let Some((x0, y0, x1, y1)) = self.clip(x, y, src.w, src.h) else {
            return;
        };
        for dy in y0..y1 {
            let s0 = ((src.y + dy - y) * img.w + src.x + x0 - x) as usize;
            let d0 = (dy * self.w + x0) as usize;
            let n = (x1 - x0) as usize;
            self.px[d0..d0 + n].copy_from_slice(&img.px[s0..s0 + n]);
        }
    }

    pub fn blit_opaque(&mut self, img: &Image, x: i32, y: i32) {
        self.blit_opaque_area(img, x, y, Rect::new(0, 0, img.w, img.h));
    }

    /// Блит с попиксельной альфой (`convert_alpha()`-поверхности, текст).
    pub fn blit_alpha(&mut self, img: &Image, x: i32, y: i32) {
        let Some((x0, y0, x1, y1)) = self.clip(x, y, img.w, img.h) else {
            return;
        };
        for dy in y0..y1 {
            let (first, last) = img.rows[(dy - y) as usize];
            let (from, to) = (x0.max(x + first), x1.min(x + last));
            if from >= to {
                continue;
            }
            let s0 = ((dy - y) * img.w + from - x) as usize;
            let d0 = (dy * self.w + from) as usize;
            let n = (to - from) as usize;
            let dst = &mut self.px[d0..d0 + n];
            let src = &img.px[s0..s0 + n];
            for (d, &s) in dst.iter_mut().zip(src) {
                blend_pixel(d, s);
            }
        }
    }

    /// `blit(pygame.transform.scale(img, (w, h)), (x, y))` для картинки с альфой.
    /// Пиксели, для которых `visible(x, y)` ложно, не рисуются.
    pub fn blit_scaled_alpha(
        &mut self,
        img: &Image,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        visible: impl Fn(i32, i32) -> bool,
    ) {
        let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) else {
            return;
        };
        let xs = stretch_indices(img.w, w);
        let ys = stretch_indices(img.h, h);
        for dy in y0..y1 {
            let srow = (ys[(dy - y) as usize] * img.w) as usize;
            let drow = (dy * self.w) as usize;
            for dx in x0..x1 {
                let s = img.px[srow + xs[(dx - x) as usize] as usize];
                if s >> 24 != 0 && visible(dx, dy) {
                    blend_pixel(&mut self.px[drow + dx as usize], s);
                }
            }
        }
    }

    /// Фон (небо/пол) и столбцы стен за один построчный проход.
    /// Столбцы стен не пересекаются, поэтому порядок их отрисовки не важен.
    pub fn walls_and_background<const COL_W: usize>(
        &mut self,
        cols: &[WallColumn],
        horizon: i32,
        sky: Rgb,
        floor: Rgb,
    ) {
        debug_assert_eq!(cols.len() * COL_W, self.w as usize);
        // состояние алгоритма растяжения (как в pygame.transform.scale) для каждого столбца
        let mut state: Vec<(i32, i32)> = cols.iter().map(|c| (0, 2 * c.sh - 2 * c.h)).collect();
        let w = self.w as usize;
        for y in 0..self.h {
            let bg = rgb(if y < horizon { sky } else { floor });
            let row = &mut self.px[y as usize * w..(y as usize + 1) * w];
            let (chunks, _) = row.as_chunks_mut::<COL_W>();
            for (chunk, (col, (src, err))) in chunks.iter_mut().zip(cols.iter().zip(&mut state)) {
                if !col.covers(y) {
                    *chunk = [bg; COL_W];
                    continue;
                }
                *chunk = [col.tex.at(col.tx, col.sy + *src) | 0xFF00_0000; COL_W];
                while *err >= 0 {
                    *src += 1;
                    *err -= 2 * col.h;
                }
                *err += 2 * col.sh;
            }
        }
    }

    /// `pygame.draw.circle(screen, color, (x0, y0), 2)`.
    pub fn crosshair(&mut self, x0: i32, y0: i32, color: Rgb) {
        for (dy, from, to) in [(-2, -1, 0), (-1, -2, 1), (0, -2, 1), (1, -1, 0)] {
            self.fill_rect(Rect::new(x0 + from, y0 + dy, to - from + 1, 1), color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretch_matches_pygame() {
        assert_eq!(stretch_indices(64, 4), vec![0, 16, 32, 48]);
        assert_eq!(stretch_indices(2, 5), vec![0, 0, 0, 1, 1]);
        assert_eq!(stretch_indices(1, 3), vec![0, 0, 0]);
        let up = stretch_indices(13, 864);
        assert_eq!((up[0], up[863]), (0, 12));
    }

    /// `ALPHA_BLEND_CHANNEL` из SDL в исходном виде.
    fn sdl_blend_channel(s: u32, d: u32, a: u32) -> u32 {
        let mut x = (s as i32 - d as i32) * a as i32 + ((d as i32) << 8) - d as i32;
        x += 1;
        x += x >> 8;
        (x >> 8) as u32
    }

    #[test]
    fn overlay_matches_sdl() {
        let mut c = Canvas::new(256, 1);
        for (d, p) in c.px.iter_mut().enumerate() {
            *p = rgb((d as u8, 255 - d as u8, d as u8));
        }
        c.overlay((255, 0, 0), 15);
        for d in 0..256u32 {
            let p = c.at(d as i32, 0);
            assert_eq!(p >> 16 & 0xFF, sdl_blend_channel(255, d, 15));
            assert_eq!(p >> 8 & 0xFF, sdl_blend_channel(0, 255 - d, 15));
        }
        // значения, снятые с pygame
        c.px[0] = rgb((10, 20, 30));
        c.px[1] = rgb((200, 220, 230));
        c.overlay((255, 0, 0), 15);
        assert_eq!(c.px[0] & 0xFFFFFF, 0x18_12_1C);
        assert_eq!(c.px[1] & 0xFFFFFF, 0xCB_CF_D8);
    }

    #[test]
    fn per_pixel_alpha_matches_pygame() {
        let mut p = rgb((48, 48, 48));
        blend_pixel(&mut p, 0x8000_0000 | 0xC8C8C8);
        assert_eq!(p & 0xFF, ((((200 - 48) * 128 + 200) >> 8) + 48) as u32);
        for (s, a) in [(0, 0), (255, 255), (77, 255), (77, 0)] {
            let mut p = rgb((90, 90, 90));
            blend_pixel(&mut p, a << 24 | s << 16 | s << 8 | s);
            assert_eq!(p & 0xFF, pygame_blend_channel(s, 90, a));
        }
    }
}
