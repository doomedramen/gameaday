//! TTF text via ab_glyph with a glyph coverage cache. Fonts are the vendored
//! Kenney Future pair (CC0).

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use std::collections::HashMap;

static FUTURE: &[u8] = include_bytes!("../assets/fonts/kenney_future.ttf");
static NARROW: &[u8] = include_bytes!("../assets/fonts/kenney_future_narrow.ttf");

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Face {
    /// Kenney Future: headings, numbers.
    Bold,
    /// Kenney Future Narrow: body text.
    Body,
}

pub struct Glyph {
    pub w: usize,
    pub h: usize,
    pub ox: i32,
    pub oy: i32,
    pub cov: Vec<u8>,
}

pub struct Fonts {
    future: FontRef<'static>,
    narrow: FontRef<'static>,
    cache: HashMap<(Face, u32, char), std::rc::Rc<Glyph>>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self::new()
    }
}

impl Fonts {
    pub fn new() -> Fonts {
        Fonts {
            future: FontRef::try_from_slice(FUTURE).expect("font"),
            narrow: FontRef::try_from_slice(NARROW).expect("font"),
            cache: HashMap::new(),
        }
    }

    fn font(&self, f: Face) -> &FontRef<'static> {
        match f {
            Face::Bold => &self.future,
            Face::Body => &self.narrow,
        }
    }

    pub fn measure(&self, s: &str, size: f32, face: Face) -> f32 {
        let font = self.font(face).as_scaled(PxScale::from(size));
        let mut w = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            let id = font.glyph_id(ch);
            if let Some(p) = prev {
                w += font.kern(p, id);
            }
            w += font.h_advance(id);
            prev = Some(id);
        }
        w
    }

    /// Glyph positions (x offset from start, y offset to baseline) for `s`.
    pub fn layout(
        &mut self,
        s: &str,
        size: f32,
        face: Face,
    ) -> Vec<(f32, f32, std::rc::Rc<Glyph>)> {
        let key_size = (size * 4.0).round() as u32;
        let ascent = self.font(face).as_scaled(PxScale::from(size)).ascent();
        let mut out = Vec::with_capacity(s.len());
        let mut x = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            let (adv, kern, id) = {
                let font = self.font(face).as_scaled(PxScale::from(size));
                let id = font.glyph_id(ch);
                let kern = prev.map(|p| font.kern(p, id)).unwrap_or(0.0);
                (font.h_advance(id), kern, id)
            };
            x += kern;
            let key = (face, key_size, ch);
            if !self.cache.contains_key(&key) {
                let g = self.raster(face, size, id);
                self.cache.insert(key, std::rc::Rc::new(g));
            }
            out.push((x, ascent, self.cache[&key].clone()));
            x += adv;
            prev = Some(id);
        }
        out
    }

    fn raster(&self, face: Face, size: f32, id: ab_glyph::GlyphId) -> Glyph {
        let font = self.font(face);
        let glyph = id.with_scale_and_position(PxScale::from(size), ab_glyph::point(0.0, 0.0));
        if let Some(og) = font.outline_glyph(glyph) {
            let b = og.px_bounds();
            let w = b.width().ceil() as usize;
            let h = b.height().ceil() as usize;
            let mut cov = vec![0u8; w * h];
            og.draw(|x, y, c| {
                let (x, y) = (x as usize, y as usize);
                if x < w && y < h {
                    cov[y * w + x] = (c.clamp(0.0, 1.0) * 255.0) as u8;
                }
            });
            Glyph {
                w,
                h,
                ox: b.min.x.floor() as i32,
                oy: b.min.y.floor() as i32,
                cov,
            }
        } else {
            Glyph {
                w: 0,
                h: 0,
                ox: 0,
                oy: 0,
                cov: vec![],
            }
        }
    }
}
