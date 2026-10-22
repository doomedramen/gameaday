//! CPU framebuffer and drawing primitives. Pixels are 0x00RRGGBB.
// Drawing calls naturally take position, size, colour and alpha.
#![allow(clippy::too_many_arguments)]

use font8x8::legacy::BASIC_LEGACY;

pub const W: usize = 1280;
pub const H: usize = 720;

#[derive(Clone)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
    /// Translation applied to every draw call (screen shake).
    pub ox: f32,
    pub oy: f32,
}

#[inline]
pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

#[inline]
pub fn split(c: u32) -> (f32, f32, f32) {
    (
        ((c >> 16) & 0xff) as f32,
        ((c >> 8) & 0xff) as f32,
        (c & 0xff) as f32,
    )
}

#[inline]
pub fn pack(r: f32, g: f32, b: f32) -> u32 {
    let c = |v: f32| v.clamp(0.0, 255.0) as u32;
    (c(r) << 16) | (c(g) << 8) | c(b)
}

pub fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let (ar, ag, ab) = split(a);
    let (br, bg, bb) = split(b);
    let t = t.clamp(0.0, 1.0);
    pack(ar + (br - ar) * t, ag + (bg - ag) * t, ab + (bb - ab) * t)
}

pub fn scale(c: u32, k: f32) -> u32 {
    let (r, g, b) = split(c);
    pack(r * k, g * k, b * k)
}

/// Multiply colour by light, adding a warm tint for bright light.
pub fn lit(c: u32, ambient: f32, glow: f32, warm: u32) -> u32 {
    let (r, g, b) = split(c);
    let (wr, wg, wb) = split(warm);
    let k = ambient + (1.0 - ambient) * glow;
    let w = glow * glow * 0.32;
    pack(r * k + wr * w, g * k + wg * w, b * k + wb * w)
}

#[inline]
pub fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Canvas {
        Canvas {
            w,
            h,
            px: vec![0; w * h],
            ox: 0.0,
            oy: 0.0,
        }
    }

    pub fn clear(&mut self, c: u32) {
        self.px.fill(c);
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize]
        } else {
            0
        }
    }

    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: u32, a: f32) {
        if a <= 0.0 || x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = y as usize * self.w + x as usize;
        if a >= 1.0 {
            self.px[i] = c;
            return;
        }
        let (r0, g0, b0) = split(self.px[i]);
        let (r1, g1, b1) = split(c);
        self.px[i] = pack(r0 + (r1 - r0) * a, g0 + (g1 - g0) * a, b0 + (b1 - b0) * a);
    }

    #[inline]
    pub fn add(&mut self, x: i32, y: i32, c: u32, a: f32) {
        if a <= 0.0 || x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = y as usize * self.w + x as usize;
        let (r0, g0, b0) = split(self.px[i]);
        let (r1, g1, b1) = split(c);
        self.px[i] = pack(r0 + r1 * a, g0 + g1 * a, b0 + b1 * a);
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, a: f32) {
        let x0 = (x + self.ox).round() as i32;
        let y0 = (y + self.oy).round() as i32;
        let x1 = (x + w + self.ox).round() as i32;
        let y1 = (y + h + self.oy).round() as i32;
        for yy in y0.max(0)..y1.min(self.h as i32) {
            for xx in x0.max(0)..x1.min(self.w as i32) {
                self.blend(xx, yy, c, a);
            }
        }
    }

    pub fn rect_outline(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, a: f32) {
        self.fill_rect(x, y, w, 1.0, c, a);
        self.fill_rect(x, y + h - 1.0, w, 1.0, c, a);
        self.fill_rect(x, y + 1.0, 1.0, h - 2.0, c, a);
        self.fill_rect(x + w - 1.0, y + 1.0, 1.0, h - 2.0, c, a);
    }

    pub fn vgradient(&mut self, x: f32, y: f32, w: f32, h: f32, top: u32, bottom: u32, a: f32) {
        let y0 = (y + self.oy).round() as i32;
        let y1 = (y + h + self.oy).round() as i32;
        let x0 = (x + self.ox).round() as i32;
        let x1 = (x + w + self.ox).round() as i32;
        for yy in y0.max(0)..y1.min(self.h as i32) {
            let t = (yy - y0) as f32 / (y1 - y0).max(1) as f32;
            let c = lerp_color(top, bottom, t);
            for xx in x0.max(0)..x1.min(self.w as i32) {
                self.blend(xx, yy, c, a);
            }
        }
    }

    /// Rounded rectangle with soft edges.
    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: u32, a: f32) {
        let x0 = (x + self.ox).floor() as i32;
        let y0 = (y + self.oy).floor() as i32;
        let x1 = (x + w + self.ox).ceil() as i32;
        let y1 = (y + h + self.oy).ceil() as i32;
        let (fx, fy) = (x + self.ox, y + self.oy);
        for yy in y0.max(0)..y1.min(self.h as i32) {
            for xx in x0.max(0)..x1.min(self.w as i32) {
                let px = xx as f32 + 0.5;
                let py = yy as f32 + 0.5;
                let qx = (px - fx - w / 2.0).abs() - (w / 2.0 - r);
                let qy = (py - fy - h / 2.0).abs() - (h / 2.0 - r);
                let d =
                    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(xx, yy, c, a * cov);
                }
            }
        }
    }

    pub fn round_rect_outline(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: u32, a: f32) {
        let x0 = (x + self.ox).floor() as i32 - 1;
        let y0 = (y + self.oy).floor() as i32 - 1;
        let x1 = (x + w + self.ox).ceil() as i32 + 1;
        let y1 = (y + h + self.oy).ceil() as i32 + 1;
        let (fx, fy) = (x + self.ox, y + self.oy);
        for yy in y0.max(0)..y1.min(self.h as i32) {
            for xx in x0.max(0)..x1.min(self.w as i32) {
                let px = xx as f32 + 0.5;
                let py = yy as f32 + 0.5;
                let qx = (px - fx - w / 2.0).abs() - (w / 2.0 - r);
                let qy = (py - fy - h / 2.0).abs() - (h / 2.0 - r);
                let d =
                    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
                let cov = (1.0 - d.abs()).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(xx, yy, c, a * cov);
                }
            }
        }
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
        self.ellipse(cx, cy, r, r, c, a);
    }

    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: u32, a: f32) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let cx = cx + self.ox;
        let cy = cy + self.oy;
        let x0 = (cx - rx - 1.0).floor() as i32;
        let x1 = (cx + rx + 1.0).ceil() as i32;
        let y0 = (cy - ry - 1.0).floor() as i32;
        let y1 = (cy + ry + 1.0).ceil() as i32;
        let m = rx.min(ry);
        for yy in y0.max(0)..y1.min(self.h as i32) {
            for xx in x0.max(0)..x1.min(self.w as i32) {
                let dx = (xx as f32 + 0.5 - cx) / rx;
                let dy = (yy as f32 + 0.5 - cy) / ry;
                let d = (dx * dx + dy * dy).sqrt();
                let cov = ((1.0 - d) * m + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(xx, yy, c, a * cov);
                }
            }
        }
    }

    pub fn ring(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, thick: f32, c: u32, a: f32) {
        let cx = cx + self.ox;
        let cy = cy + self.oy;
        let x0 = (cx - rx - thick - 1.0).floor() as i32;
        let x1 = (cx + rx + thick + 1.0).ceil() as i32;
        let y0 = (cy - ry - thick - 1.0).floor() as i32;
        let y1 = (cy + ry + thick + 1.0).ceil() as i32;
        for yy in y0.max(0)..y1.min(self.h as i32) {
            for xx in x0.max(0)..x1.min(self.w as i32) {
                let dx = (xx as f32 + 0.5 - cx) / rx;
                let dy = (yy as f32 + 0.5 - cy) / ry;
                let d = ((dx * dx + dy * dy).sqrt() - 1.0).abs() * rx.min(ry);
                let cov = (thick / 2.0 + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(xx, yy, c, a * cov);
                }
            }
        }
    }

    /// Additive radial glow.
    pub fn glow(&mut self, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
        self.glow_ellipse(cx, cy, r, r, c, a);
    }

    pub fn glow_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: u32, a: f32) {
        if rx <= 0.0 || ry <= 0.0 || a <= 0.0 {
            return;
        }
        let cx = cx + self.ox;
        let cy = cy + self.oy;
        let x0 = (cx - rx).floor() as i32;
        let x1 = (cx + rx).ceil() as i32;
        let y0 = (cy - ry).floor() as i32;
        let y1 = (cy + ry).ceil() as i32;
        for yy in y0.max(0)..y1.min(self.h as i32) {
            for xx in x0.max(0)..x1.min(self.w as i32) {
                let dx = (xx as f32 + 0.5 - cx) / rx;
                let dy = (yy as f32 + 0.5 - cy) / ry;
                let d2 = dx * dx + dy * dy;
                if d2 < 1.0 {
                    let f = (1.0 - d2.sqrt()).powi(2);
                    self.add(xx, yy, c, a * f);
                }
            }
        }
    }

    /// Filled polygon (even-odd), one sample per pixel row with AA columns.
    pub fn poly(&mut self, pts: &[(f32, f32)], c: u32, a: f32) {
        if pts.len() < 3 {
            return;
        }
        let pts: Vec<(f32, f32)> = pts
            .iter()
            .map(|(x, y)| (x + self.ox, y + self.oy))
            .collect();
        let miny = pts
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min)
            .floor() as i32;
        let maxy = pts
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as i32;
        let mut xs: Vec<f32> = Vec::with_capacity(8);
        for yy in miny.max(0)..maxy.min(self.h as i32) {
            let sy = yy as f32 + 0.5;
            xs.clear();
            for i in 0..pts.len() {
                let (x0, y0) = pts[i];
                let (x1, y1) = pts[(i + 1) % pts.len()];
                if (y0 <= sy && y1 > sy) || (y1 <= sy && y0 > sy) {
                    xs.push(x0 + (sy - y0) / (y1 - y0) * (x1 - x0));
                }
            }
            xs.sort_by(|a, b| a.total_cmp(b));
            for pair in xs.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let (l, r) = (pair[0], pair[1]);
                let xl = l.floor() as i32;
                let xr = r.ceil() as i32;
                for xx in xl.max(0)..xr.min(self.w as i32) {
                    let px = xx as f32;
                    let cov = ((px + 1.0).min(r) - px.max(l)).clamp(0.0, 1.0);
                    if cov > 0.0 {
                        self.blend(xx, yy, c, a * cov);
                    }
                }
            }
        }
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thick: f32, c: u32, a: f32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let nx = -dy / len * thick / 2.0;
        let ny = dx / len * thick / 2.0;
        self.poly(
            &[
                (x0 + nx, y0 + ny),
                (x1 + nx, y1 + ny),
                (x1 - nx, y1 - ny),
                (x0 - nx, y0 - ny),
            ],
            c,
            a,
        );
    }

    // ------------------------------------------------------------------ text

    pub fn text_width(s: &str, scale: f32) -> f32 {
        s.chars().count() as f32 * 7.0 * scale
    }

    /// Draw text; the 8x8 font is drawn with 7px advance for tighter spacing.
    pub fn text(&mut self, x: f32, y: f32, s: &str, scale: f32, c: u32, a: f32) {
        let mut cx = x;
        for ch in s.chars() {
            let code = ch as usize;
            if code < 128 {
                let glyph = BASIC_LEGACY[code];
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            if scale <= 1.0 {
                                let px = (cx + col as f32 + self.ox).round() as i32;
                                let py = (y + row as f32 + self.oy).round() as i32;
                                self.blend(px, py, c, a);
                            } else {
                                self.fill_rect(
                                    cx + col as f32 * scale,
                                    y + row as f32 * scale,
                                    scale,
                                    scale,
                                    c,
                                    a,
                                );
                            }
                        }
                    }
                }
            }
            cx += 7.0 * scale;
        }
    }

    pub fn text_shadow(&mut self, x: f32, y: f32, s: &str, scale: f32, c: u32, a: f32) {
        let o = scale.max(1.0);
        self.text(x + o, y + o, s, scale, 0x000000, a * 0.75);
        self.text(x, y, s, scale, c, a);
    }

    pub fn text_center(&mut self, cx: f32, y: f32, s: &str, scale: f32, c: u32, a: f32) {
        let w = Canvas::text_width(s, scale);
        self.text_shadow(cx - w / 2.0, y, s, scale, c, a);
    }

    /// Word-wrapped text; returns the height used.
    pub fn text_wrap(
        &mut self,
        x: f32,
        y: f32,
        max_w: f32,
        s: &str,
        scale: f32,
        c: u32,
        a: f32,
    ) -> f32 {
        let mut line = String::new();
        let mut yy = y;
        let lh = 10.0 * scale;
        for word in s.split_whitespace() {
            let cand = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if Canvas::text_width(&cand, scale) > max_w && !line.is_empty() {
                self.text_shadow(x, yy, &line, scale, c, a);
                yy += lh;
                line = word.to_string();
            } else {
                line = cand;
            }
        }
        if !line.is_empty() {
            self.text_shadow(x, yy, &line, scale, c, a);
            yy += lh;
        }
        yy - y
    }

    // ------------------------------------------------------------------- png

    pub fn save_png(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut enc =
            png::Encoder::new(std::io::BufWriter::new(file), self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(std::io::Error::other)?;
        let mut data = Vec::with_capacity(self.w * self.h * 3);
        for p in &self.px {
            data.push((p >> 16) as u8);
            data.push((p >> 8) as u8);
            data.push(*p as u8);
        }
        writer
            .write_image_data(&data)
            .map_err(std::io::Error::other)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_stay_in_bounds() {
        let mut c = Canvas::new(64, 48);
        c.circle(-10.0, -10.0, 30.0, 0xffffff, 1.0);
        c.poly(&[(-5.0, 0.0), (100.0, 10.0), (20.0, 90.0)], 0xff0000, 0.5);
        c.line(0.0, 0.0, 200.0, 200.0, 3.0, 0x00ff00, 1.0);
        c.glow(32.0, 24.0, 100.0, 0x404040, 1.0);
        c.text(-3.0, 40.0, "Hello", 2.0, 0xffffff, 1.0);
        c.round_rect(-4.0, -4.0, 80.0, 60.0, 6.0, 0x123456, 0.5);
        assert!(c.px.iter().any(|p| *p != 0));
    }

    #[test]
    fn color_math() {
        assert_eq!(pack(300.0, -4.0, 128.0), rgb(255, 0, 128));
        assert_eq!(lerp_color(0x000000, 0xffffff, 0.5) >> 16, 127);
        assert_eq!(scale(rgb(100, 100, 100), 0.5), rgb(50, 50, 50));
    }

    #[test]
    fn text_draws_glyph_pixels() {
        let mut c = Canvas::new(32, 16);
        c.text(0.0, 0.0, "A", 1.0, 0xffffff, 1.0);
        assert!(c.px.iter().filter(|p| **p == 0xffffff).count() > 8);
    }
}
