//! Parallax sky: a pre-rendered gradient with stars and distant islets,
//! plus two wrapping cloud layers that drift and shift with the mouse.
#![allow(clippy::too_many_arguments)]

use super::canvas::{Canvas, H, W, lerp_color, split};
use super::palette::Palette;
use crate::rng::Rng;

struct Layer {
    w: usize,
    h: usize,
    y0: usize,
    alpha: Vec<u8>,
    color: u32,
}

impl Layer {
    fn new(
        rng: &mut Rng,
        w: usize,
        h: usize,
        y0: usize,
        blobs: usize,
        rmin: f32,
        rmax: f32,
        color: u32,
    ) -> Layer {
        let mut acc = vec![0f32; w * h];
        for _ in 0..blobs {
            let cx = rng.f32() * w as f32;
            let cy = h as f32 * (0.35 + rng.f32() * 0.5);
            let r = rmin + rng.f32() * (rmax - rmin);
            // A cloud is a cluster of flattened puffs.
            for _ in 0..6 {
                let px = cx + (rng.f32() - 0.5) * r * 2.2;
                let py = cy + (rng.f32() - 0.5) * r * 0.5;
                let pr = r * (0.4 + rng.f32() * 0.6);
                let ry = pr * 0.45;
                let x0 = (px - pr) as i32;
                let x1 = (px + pr) as i32;
                let y0 = (py - ry).max(0.0) as i32;
                let y1 = ((py + ry) as i32).min(h as i32 - 1);
                for yy in y0..=y1 {
                    for xx in x0..=x1 {
                        let dx = (xx as f32 - px) / pr;
                        let dy = (yy as f32 - py) / ry;
                        let d = dx * dx + dy * dy;
                        if d < 1.0 {
                            let wx = xx.rem_euclid(w as i32) as usize;
                            let i = yy as usize * w + wx;
                            acc[i] += (1.0 - d) * 0.6;
                        }
                    }
                }
            }
        }
        let alpha = acc.iter().map(|a| (a.min(1.0) * 255.0) as u8).collect();
        Layer {
            w,
            h,
            y0,
            alpha,
            color,
        }
    }

    fn draw(&self, c: &mut Canvas, offset: f32, strength: f32) {
        let off = offset.rem_euclid(self.w as f32) as usize;
        let (cr, cg, cb) = split(self.color);
        for y in 0..self.h {
            let sy = self.y0 + y;
            if sy >= c.h {
                break;
            }
            let row = &self.alpha[y * self.w..(y + 1) * self.w];
            let dst = &mut c.px[sy * c.w..(sy + 1) * c.w];
            for (x, d) in dst.iter_mut().enumerate() {
                let a = row[(x + off) % self.w];
                if a == 0 {
                    continue;
                }
                let a = a as f32 / 255.0 * strength;
                let (r0, g0, b0) = split(*d);
                *d =
                    super::canvas::pack(r0 + (cr - r0) * a, g0 + (cg - g0) * a, b0 + (cb - b0) * a);
            }
        }
    }
}

pub struct Sky {
    base: Vec<u32>,
    far: Layer,
    near: Layer,
}

impl Sky {
    pub fn new(p: &Palette, seed: u64) -> Sky {
        let mut rng = Rng::new(seed ^ 0x5C1E5);
        let mut base = Canvas::new(W, H);
        // Three-stop gradient.
        for y in 0..H {
            let t = y as f32 / H as f32;
            let col = if t < 0.55 {
                lerp_color(p.sky_top, p.sky_mid, t / 0.55)
            } else {
                lerp_color(p.sky_mid, p.sky_low, (t - 0.55) / 0.45)
            };
            for x in 0..W {
                base.px[y * W + x] = col;
            }
        }
        // Stars in the upper sky.
        for _ in 0..260 {
            let x = rng.range(0, W as i32 - 1);
            let y = rng.range(0, (H as f32 * 0.5) as i32);
            let fade = 1.0 - y as f32 / (H as f32 * 0.5);
            let b = 0.25 + rng.f32() * 0.75;
            base.add(x, y, 0xffffff, b * fade * 0.8);
            if rng.chance(0.08) {
                base.glow(x as f32, y as f32, 3.0, 0xaaccff, 0.5 * fade);
            }
        }
        // Horizon glow (a low sun or moon behind the clouds).
        let sx = W as f32 * (0.2 + rng.f32() * 0.6);
        base.glow_ellipse(sx, H as f32 * 0.78, 520.0, 220.0, p.horizon_glow, 0.35);
        base.glow(sx, H as f32 * 0.66, 60.0, p.horizon_glow, 0.5);
        // Distant floating islets.
        for _ in 0..7 {
            let cx = rng.f32() * W as f32;
            let cy = H as f32 * (0.25 + rng.f32() * 0.45);
            let w = 30.0 + rng.f32() * 90.0;
            let depth = w * (0.5 + rng.f32() * 0.5);
            let col = lerp_color(p.islet, p.sky_mid, 0.3 + rng.f32() * 0.4);
            let mut pts = vec![(cx - w / 2.0, cy), (cx + w / 2.0, cy)];
            let steps = 5;
            for i in 0..=steps {
                let t = 1.0 - i as f32 / steps as f32;
                let x = cx - w / 2.0 + w * t;
                let jag = (rng.f32() - 0.5) * w * 0.15;
                let y = cy + depth * (1.0 - (2.0 * t - 1.0).abs()).powf(0.7) + jag;
                pts.push((x + jag * 0.5, y));
            }
            base.poly(&pts, col, 1.0);
            base.fill_rect(
                cx - w / 2.0,
                cy - 2.0,
                w,
                3.0,
                lerp_color(col, p.horizon_glow, 0.25),
                0.8,
            );
        }
        let far = Layer::new(&mut rng, W * 2, 300, 120, 22, 60.0, 140.0, p.cloud_far);
        let near = Layer::new(&mut rng, W * 2, 260, H - 260, 18, 80.0, 180.0, p.cloud);
        Sky {
            base: base.px,
            far,
            near,
        }
    }

    pub fn draw(&self, c: &mut Canvas, t: f32, parallax: f32) {
        c.px.copy_from_slice(&self.base);
        self.far.draw(c, t * 4.0 + parallax * 12.0, 0.55);
    }

    /// Low cloud sea drawn after the island so it wraps the cliff bases.
    pub fn draw_near(&self, c: &mut Canvas, t: f32, parallax: f32) {
        self.near.draw(c, t * 9.0 + parallax * 30.0, 0.5);
    }
}
