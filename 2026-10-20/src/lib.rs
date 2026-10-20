#[derive(Clone, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w &&
        self.x + self.w > other.x &&
        self.y < other.y + other.h &&
        self.y + self.h > other.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intersect() {
        let a = Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 };
        let b = Rect { x: 5.0, y: 5.0, w: 10.0, h: 10.0 };
        assert!(a.intersects(&b));
    }
    
    #[test]
    fn test_no_intersect() {
        let a = Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 };
        let b = Rect { x: 20.0, y: 20.0, w: 10.0, h: 10.0 };
        assert!(!a.intersects(&b));
    }
}
