//! Software framebuffer: alpha blending, anti-aliased shapes, rotated and
//! scaled sprite blits and TTF text. Colours are 0xRRGGBB; sprites are
//! straight-alpha 0xAARRGGBB.

use crate::font::{Face, Fonts};

pub const W: usize = 1280;
pub const H: usize = 720;

#[derive(Clone, Debug)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

impl Sprite {
    pub fn new(w: usize, h: usize) -> Sprite {
        Sprite {
            w,
            h,
            px: vec![0; w * h],
        }
    }
    /// Half-size copy using a 2x2 box filter in premultiplied space.
    pub fn half(&self) -> Sprite {
        let (w, h) = (self.w.div_ceil(2), self.h.div_ceil(2));
        let mut out = Sprite::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let (mut a, mut r, mut g, mut b) = (0u32, 0u32, 0u32, 0u32);
                for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + ox).min(self.w - 1);
                    let sy = (y * 2 + oy).min(self.h - 1);
                    let p = self.px[sy * self.w + sx];
                    let pa = p >> 24;
                    a += pa;
                    r += ((p >> 16) & 255) * pa;
                    g += ((p >> 8) & 255) * pa;
                    b += (p & 255) * pa;
                }
                out.px[y * w + x] = if a == 0 {
                    0
                } else {
                    ((a / 4) << 24) | ((r / a) << 16) | ((g / a) << 8) | (b / a)
                };
            }
        }
        out
    }
    /// Average opaque colour (used for minimap and particles).
    pub fn average(&self) -> u32 {
        let (mut n, mut r, mut g, mut b) = (0u64, 0u64, 0u64, 0u64);
        for &p in &self.px {
            if p >> 24 > 200 {
                n += 1;
                r += ((p >> 16) & 255) as u64;
                g += ((p >> 8) & 255) as u64;
                b += (p & 255) as u64;
            }
        }
        if n == 0 {
            return 0x808080;
        }
        (((r / n) << 16) | ((g / n) << 8) | (b / n)) as u32
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Blit {
    pub rot: f32,
    pub scale: f32,
    pub alpha: f32,
    /// Draw the sprite's shape in this colour (for shadows and silhouettes).
    pub solid: Option<u32>,
    /// 0..1 blend toward white (hit flash).
    pub flash: f32,
    /// Multiply tint.
    pub tint: u32,
}

impl Default for Blit {
    fn default() -> Blit {
        Blit {
            rot: 0.0,
            scale: 1.0,
            alpha: 1.0,
            solid: None,
            flash: 0.0,
            tint: 0xFFFFFF,
        }
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let x = ((a >> s) & 255) as f32;
        let y = ((b >> s) & 255) as f32;
        ((x + (y - x) * t) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

pub fn scale_rgb(c: u32, k: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 255) as f32 * k).clamp(0.0, 255.0) as u32) << s;
    ch(16) | ch(8) | ch(0)
}

#[inline]
fn blend(dst: u32, src: u32, a: u32) -> u32 {
    // a in 0..=256
    let ia = 256 - a;
    let rb = ((((src & 0xFF00FF) * a) + ((dst & 0xFF00FF) * ia)) >> 8) & 0xFF00FF;
    let g = ((((src & 0x00FF00) * a) + ((dst & 0x00FF00) * ia)) >> 8) & 0x00FF00;
    rb | g
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
    pub fonts: Fonts,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Canvas {
        Canvas {
            w,
            h,
            px: vec![0; w * h],
            fonts: Fonts::new(),
        }
    }

    pub fn clear(&mut self, c: u32) {
        self.px.fill(c);
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: u32, a: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 || a <= 0.0 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        let ai = (a.min(1.0) * 256.0) as u32;
        self.px[i] = blend(self.px[i], c, ai);
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, a: f32) {
        let x0 = x.max(0.0) as i32;
        let y0 = y.max(0.0) as i32;
        let x1 = ((x + w).min(self.w as f32)) as i32;
        let y1 = ((y + h).min(self.h as f32)) as i32;
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let ai = (a.clamp(0.0, 1.0) * 256.0) as u32;
        for yy in y0..y1 {
            let row = yy as usize * self.w;
            for xx in x0..x1 {
                let i = row + xx as usize;
                self.px[i] = if ai >= 256 {
                    c
                } else {
                    blend(self.px[i], c, ai)
                };
            }
        }
    }

    /// Anti-aliased rounded rectangle.
    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: u32, a: f32) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        let x0 = x.floor().max(0.0) as i32;
        let y0 = y.floor().max(0.0) as i32;
        let x1 = (x + w).ceil().min(self.w as f32) as i32;
        let y1 = (y + h).ceil().min(self.h as f32) as i32;
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let (hx, hy) = (w / 2.0 - r, h / 2.0 - r);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let px = (xx as f32 + 0.5 - cx).abs() - hx;
                let py = (yy as f32 + 0.5 - cy).abs() - hy;
                let d =
                    (px.max(0.0).powi(2) + py.max(0.0).powi(2)).sqrt() + px.max(py).min(0.0) - r;
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Rounded rectangle outline of the given thickness.
    pub fn round_rect_line(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        t: f32,
        c: u32,
        a: f32,
    ) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        let x0 = (x - 1.0).floor().max(0.0) as i32;
        let y0 = (y - 1.0).floor().max(0.0) as i32;
        let x1 = (x + w + 1.0).ceil().min(self.w as f32) as i32;
        let y1 = (y + h + 1.0).ceil().min(self.h as f32) as i32;
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let (hx, hy) = (w / 2.0 - r, h / 2.0 - r);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let px = (xx as f32 + 0.5 - cx).abs() - hx;
                let py = (yy as f32 + 0.5 - cy).abs() - hy;
                let d =
                    (px.max(0.0).powi(2) + py.max(0.0).powi(2)).sqrt() + px.max(py).min(0.0) - r;
                let cov = (0.5 + t / 2.0 - (d + t / 2.0).abs()).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
        if r <= 0.0 {
            return;
        }
        let x0 = (cx - r - 1.0).floor().max(0.0) as i32;
        let y0 = (cy - r - 1.0).floor().max(0.0) as i32;
        let x1 = (cx + r + 1.0).ceil().min(self.w as f32) as i32;
        let y1 = (cy + r + 1.0).ceil().min(self.h as f32) as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                let dx = xx as f32 + 0.5 - cx;
                let dy = yy as f32 + 0.5 - cy;
                let d = (dx * dx + dy * dy).sqrt();
                let cov = (r - d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Soft radial glow: alpha falls off smoothly to zero at r.
    pub fn glow(&mut self, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
        let x0 = (cx - r).floor().max(0.0) as i32;
        let y0 = (cy - r).floor().max(0.0) as i32;
        let x1 = (cx + r).ceil().min(self.w as f32) as i32;
        let y1 = (cy + r).ceil().min(self.h as f32) as i32;
        let inv = 1.0 / (r * r);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let dx = xx as f32 + 0.5 - cx;
                let dy = yy as f32 + 0.5 - cy;
                let t = 1.0 - (dx * dx + dy * dy) * inv;
                if t > 0.0 {
                    self.add(xx, yy, c, a * t * t);
                }
            }
        }
    }

    /// Additive blend.
    #[inline]
    pub fn add(&mut self, x: i32, y: i32, c: u32, a: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        let d = self.px[i];
        let ch = |s: u32| {
            let v = ((d >> s) & 255) as f32 + ((c >> s) & 255) as f32 * a;
            (v.min(255.0) as u32) << s
        };
        self.px[i] = ch(16) | ch(8) | ch(0);
    }

    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, c: u32, a: f32) {
        let ro = r + t / 2.0;
        let x0 = (cx - ro - 1.0).floor().max(0.0) as i32;
        let y0 = (cy - ro - 1.0).floor().max(0.0) as i32;
        let x1 = (cx + ro + 1.0).ceil().min(self.w as f32) as i32;
        let y1 = (cy + ro + 1.0).ceil().min(self.h as f32) as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                let dx = xx as f32 + 0.5 - cx;
                let dy = yy as f32 + 0.5 - cy;
                let d = ((dx * dx + dy * dy).sqrt() - r).abs();
                let cov = (t / 2.0 - d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Partial ring from angle a0 sweeping by `sweep` radians (clockwise on screen).
    pub fn arc(&mut self, cx: f32, cy: f32, r: f32, t: f32, a0: f32, sweep: f32, c: u32, a: f32) {
        let ro = r + t / 2.0;
        let x0 = (cx - ro - 1.0).floor().max(0.0) as i32;
        let y0 = (cy - ro - 1.0).floor().max(0.0) as i32;
        let x1 = (cx + ro + 1.0).ceil().min(self.w as f32) as i32;
        let y1 = (cy + ro + 1.0).ceil().min(self.h as f32) as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                let dx = xx as f32 + 0.5 - cx;
                let dy = yy as f32 + 0.5 - cy;
                let d = ((dx * dx + dy * dy).sqrt() - r).abs();
                let cov = (t / 2.0 - d + 0.5).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    continue;
                }
                let ang = (dy.atan2(dx) - a0).rem_euclid(std::f32::consts::TAU);
                if ang <= sweep {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Anti-aliased thick line with round caps.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: u32, a: f32) {
        let r = t / 2.0;
        let bx0 = (x0.min(x1) - r - 1.0).floor().max(0.0) as i32;
        let by0 = (y0.min(y1) - r - 1.0).floor().max(0.0) as i32;
        let bx1 = (x0.max(x1) + r + 1.0).ceil().min(self.w as f32) as i32;
        let by1 = (y0.max(y1) + r + 1.0).ceil().min(self.h as f32) as i32;
        let (dx, dy) = (x1 - x0, y1 - y0);
        let l2 = (dx * dx + dy * dy).max(1e-6);
        for yy in by0..by1 {
            for xx in bx0..bx1 {
                let px = xx as f32 + 0.5 - x0;
                let py = yy as f32 + 0.5 - y0;
                let h = ((px * dx + py * dy) / l2).clamp(0.0, 1.0);
                let ex = px - dx * h;
                let ey = py - dy * h;
                let d = (ex * ex + ey * ey).sqrt();
                let cov = (r - d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Filled triangle (anti-aliased edges).
    pub fn tri(&mut self, p: [(f32, f32); 3], c: u32, a: f32) {
        let minx = p
            .iter()
            .map(|q| q.0)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as i32;
        let maxx = p
            .iter()
            .map(|q| q.0)
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(self.w as f32) as i32;
        let miny = p
            .iter()
            .map(|q| q.1)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as i32;
        let maxy = p
            .iter()
            .map(|q| q.1)
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(self.h as f32) as i32;
        let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
        if area.abs() < 1e-3 {
            return;
        }
        let s = area.signum();
        let edge = |a: (f32, f32), b: (f32, f32), x: f32, y: f32| {
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let l = (ex * ex + ey * ey).sqrt().max(1e-6);
            s * ((x - a.0) * ey - (y - a.1) * ex) / l
        };
        for yy in miny..maxy {
            for xx in minx..maxx {
                let (x, y) = (xx as f32 + 0.5, yy as f32 + 0.5);
                let d0 = -edge(p[0], p[1], x, y);
                let d1 = -edge(p[1], p[2], x, y);
                let d2 = -edge(p[2], p[0], x, y);
                let d = d0.min(d1).min(d2);
                let cov = (d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Draw a sprite centred at (cx, cy), rotated and scaled, with bilinear
    /// filtering in premultiplied space.
    pub fn blit(&mut self, s: &Sprite, cx: f32, cy: f32, o: Blit) {
        if o.alpha <= 0.0 || o.scale <= 0.0 {
            return;
        }
        let (sn, cs) = o.rot.sin_cos();
        let hw = s.w as f32 * 0.5 * o.scale;
        let hh = s.h as f32 * 0.5 * o.scale;
        let ex = (hw * cs).abs() + (hh * sn).abs();
        let ey = (hw * sn).abs() + (hh * cs).abs();
        let x0 = (cx - ex - 1.0).floor().max(0.0) as i32;
        let y0 = (cy - ey - 1.0).floor().max(0.0) as i32;
        let x1 = (cx + ex + 1.0).ceil().min(self.w as f32) as i32;
        let y1 = (cy + ey + 1.0).ceil().min(self.h as f32) as i32;
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let inv = 1.0 / o.scale;
        let (sw, sh) = (s.w as i32, s.h as i32);
        let tint = o.tint;
        let (tr, tg, tb) = (
            ((tint >> 16) & 255) as f32 / 255.0,
            ((tint >> 8) & 255) as f32 / 255.0,
            (tint & 255) as f32 / 255.0,
        );
        for yy in y0..y1 {
            let dy = yy as f32 + 0.5 - cy;
            for xx in x0..x1 {
                let dx = xx as f32 + 0.5 - cx;
                // inverse rotate
                let u = (dx * cs + dy * sn) * inv + s.w as f32 * 0.5 - 0.5;
                let v = (-dx * sn + dy * cs) * inv + s.h as f32 * 0.5 - 0.5;
                if u < -1.0 || v < -1.0 || u > sw as f32 || v > sh as f32 {
                    continue;
                }
                let ux = u.floor();
                let vy = v.floor();
                let fx = u - ux;
                let fy = v - vy;
                let (ix, iy) = (ux as i32, vy as i32);
                let mut acc = [0.0f32; 4];
                for (ox, oy, wgt) in [
                    (0, 0, (1.0 - fx) * (1.0 - fy)),
                    (1, 0, fx * (1.0 - fy)),
                    (0, 1, (1.0 - fx) * fy),
                    (1, 1, fx * fy),
                ] {
                    let (px, py) = (ix + ox, iy + oy);
                    if px < 0 || py < 0 || px >= sw || py >= sh || wgt <= 0.0 {
                        continue;
                    }
                    let p = s.px[(py * sw + px) as usize];
                    let pa = (p >> 24) as f32 * wgt;
                    acc[0] += pa;
                    acc[1] += ((p >> 16) & 255) as f32 * pa;
                    acc[2] += ((p >> 8) & 255) as f32 * pa;
                    acc[3] += (p & 255) as f32 * pa;
                }
                if acc[0] < 1.0 {
                    continue;
                }
                let a = acc[0] / 255.0 * o.alpha;
                let c = if let Some(sc) = o.solid {
                    sc
                } else {
                    let ia = 1.0 / acc[0];
                    let mut r = acc[1] * ia * tr;
                    let mut g = acc[2] * ia * tg;
                    let mut b = acc[3] * ia * tb;
                    if o.flash > 0.0 {
                        r += (255.0 - r) * o.flash;
                        g += (255.0 - g) * o.flash;
                        b += (255.0 - b) * o.flash;
                    }
                    ((r as u32).min(255) << 16) | ((g as u32).min(255) << 8) | (b as u32).min(255)
                };
                self.put(xx, yy, c, a);
            }
        }
    }

    /// Multiply every pixel by a colour (darkening/tinting the frame).
    pub fn multiply_all(&mut self, c: u32) {
        let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
        for p in self.px.iter_mut() {
            let pr = ((*p >> 16) & 255) * r / 255;
            let pg = ((*p >> 8) & 255) * g / 255;
            let pb = (*p & 255) * b / 255;
            *p = (pr << 16) | (pg << 8) | pb;
        }
    }

    pub fn darken(&mut self, a: f32) {
        let k = ((1.0 - a.clamp(0.0, 1.0)) * 256.0) as u32;
        for p in self.px.iter_mut() {
            let rb = ((*p & 0xFF00FF) * k >> 8) & 0xFF00FF;
            let g = ((*p & 0x00FF00) * k >> 8) & 0x00FF00;
            *p = rb | g;
        }
    }

    // ---- text ------------------------------------------------------------

    pub fn text_width(&mut self, s: &str, size: f32, face: Face) -> f32 {
        self.fonts.measure(s, size, face)
    }

    /// Draw text with its top-left (or aligned anchor) at x, baseline-free
    /// top at y. Returns the width.
    pub fn text(
        &mut self,
        s: &str,
        x: f32,
        y: f32,
        size: f32,
        face: Face,
        c: u32,
        a: f32,
        align: Align,
    ) -> f32 {
        let w = self.fonts.measure(s, size, face);
        let x0 = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        let glyphs = self.fonts.layout(s, size, face);
        for (gx, gy, g) in glyphs {
            let ox = (x0 + gx).round() as i32 + g.ox;
            let oy = (y + gy).round() as i32 + g.oy;
            for yy in 0..g.h {
                for xx in 0..g.w {
                    let cov = g.cov[yy * g.w + xx];
                    if cov > 0 {
                        self.put(ox + xx as i32, oy + yy as i32, c, a * cov as f32 / 255.0);
                    }
                }
            }
        }
        w
    }

    /// Text with a dark drop shadow for legibility over the world.
    pub fn text_sh(
        &mut self,
        s: &str,
        x: f32,
        y: f32,
        size: f32,
        face: Face,
        c: u32,
        a: f32,
        align: Align,
    ) -> f32 {
        let off = (size / 14.0).max(1.0).round();
        self.text(s, x + off, y + off, size, face, 0x1A1F2B, a * 0.55, align);
        self.text(s, x, y, size, face, c, a, align)
    }

    /// Text with a thick outline (titles, big callouts).
    pub fn text_outline(
        &mut self,
        s: &str,
        x: f32,
        y: f32,
        size: f32,
        face: Face,
        c: u32,
        oc: u32,
        a: f32,
        align: Align,
    ) -> f32 {
        let t = (size / 16.0).max(1.5);
        for (dx, dy) in [
            (-t, 0.0),
            (t, 0.0),
            (0.0, -t),
            (0.0, t),
            (-t * 0.7, -t * 0.7),
            (t * 0.7, -t * 0.7),
            (-t * 0.7, t * 0.7),
            (t * 0.7, t * 0.7),
            (t * 0.8, t * 1.6),
        ] {
            self.text(s, x + dx, y + dy, size, face, oc, a, align);
        }
        self.text(s, x, y, size, face, c, a, align)
    }
}
