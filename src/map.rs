use crate::rect::Rect;
use crate::settings::*;

/// Карта уровня (`mapdata.py`).
pub struct WorldMap {
    /// `map_width` из оригинала: длина самой длинной строки вместе с `\n` минус 1.
    pub width: i32,
    pub height: i32,
    grid: Vec<Vec<Option<u8>>>,
    /// Блоки в порядке обхода файла — как `collision_map`.
    pub collision: Vec<Rect>,
}

impl WorldMap {
    pub fn parse(text: &str) -> WorldMap {
        // Python читает файл в текстовом режиме: \r\n и \r превращаются в \n.
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as i32 - 1;
        let height = lines.len() as i32;
        let mut grid = Vec::with_capacity(lines.len());
        let mut collision = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let mut row = Vec::new();
            for (j, block) in line.chars().enumerate() {
                if block != ' ' && block != '\n' {
                    row.push(Some(block as u8));
                    collision.push(Rect::new(
                        j as i32 * TILE_SIZE,
                        i as i32 * TILE_SIZE,
                        TILE_SIZE,
                        TILE_SIZE,
                    ));
                } else {
                    row.push(None);
                }
            }
            grid.push(row);
        }
        WorldMap {
            width,
            height,
            grid,
            collision,
        }
    }

    pub fn load() -> Result<WorldMap, String> {
        let text = std::str::from_utf8(crate::embedded::file("maps/map.txt"))
            .map_err(|e| format!("maps/map.txt: {e}"))?;
        Ok(WorldMap::parse(text))
    }

    pub fn tile(&self, tx: i64, ty: i64) -> Option<u8> {
        if tx < 0 || ty < 0 {
            return None;
        }
        *self.grid.get(ty as usize)?.get(tx as usize)?
    }

    /// `surface_hit(x, y) in world_map`.
    pub fn at(&self, x: f64, y: f64) -> Option<u8> {
        self.tile((x / TILE).floor() as i64, (y / TILE).floor() as i64)
    }

    /// Координаты всех блоков в тайлах (для миникарты).
    pub fn blocks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.grid.iter().enumerate().flat_map(|(i, row)| {
            row.iter()
                .enumerate()
                .filter_map(move |(j, b)| b.map(|_| (j as i32, i as i32)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_python() {
        let m = WorldMap::parse("ab \r\n c\r\nxyzw");
        // самая длинная строка "xyzw" без \n: 4 - 1 = 3 (так считает оригинал)
        assert_eq!((m.width, m.height), (3, 3));
        assert_eq!(m.tile(0, 0), Some(b'a'));
        assert_eq!(m.tile(2, 0), None);
        assert_eq!(m.tile(1, 1), Some(b'c'));
        assert_eq!(m.collision.len(), 7);
        assert_eq!(m.at(70.0, 1.0), Some(b'b'));
        assert_eq!(m.at(-0.5, 1.0), None);
    }
}
