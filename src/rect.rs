/// Целочисленный прямоугольник с семантикой `pygame.Rect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    /// `rect.move(dx, dy)`: pygame отбрасывает дробную часть.
    pub fn moved(&self, dx: f64, dy: f64) -> Rect {
        Rect::new(self.x + dx as i32, self.y + dy as i32, self.w, self.h)
    }

    /// `rect.center = (x, y)`: pygame округляет float (половины — от нуля).
    pub fn set_center(&mut self, cx: f64, cy: f64) {
        self.x = cx.round() as i32 - self.w / 2;
        self.y = cy.round() as i32 - self.h / 2;
    }

    pub fn colliderect(&self, other: &Rect) -> bool {
        if self.w == 0 || self.h == 0 || other.w == 0 || other.h == 0 {
            return false;
        }
        self.x < other.right()
            && self.y < other.bottom()
            && self.right() > other.x
            && self.bottom() > other.y
    }

    pub fn collidepoint(&self, (px, py): (i32, i32)) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    pub fn centered(cx: i32, cy: i32, w: i32, h: i32) -> Rect {
        Rect::new(cx - w / 2, cy - h / 2, w, h)
    }

    pub fn centerx(&self) -> i32 {
        self.x + self.w / 2
    }

    pub fn centery(&self) -> i32 {
        self.y + self.h / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_rounds_half_away_from_zero() {
        let mut r = Rect::new(0, 0, 32, 32);
        for (v, x) in [
            (100.5, 85),
            (101.5, 86),
            (100.49, 84),
            (-0.5, -17),
            (-2.5, -19),
            (2.5, -13),
        ] {
            r.set_center(v, v);
            assert_eq!(r.x, x, "center {v}");
        }
    }

    #[test]
    fn move_truncates() {
        let r = Rect::new(85, -22, 32, 32).moved(5.9, -5.9);
        assert_eq!((r.x, r.y), (90, -27));
    }

    #[test]
    fn touching_rects_do_not_collide() {
        let a = Rect::new(0, 0, 64, 64);
        assert!(!a.colliderect(&Rect::new(64, 0, 64, 64)));
        assert!(a.colliderect(&Rect::new(63, 63, 64, 64)));
    }
}
