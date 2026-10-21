//! Software rasteriser: everything is drawn into a `u32` ARGB buffer that
//! minifb blits to the window. The buffer is fully cleared every frame which
//! is what kills the "ghost frame" artifact the player complained about on
//! day 1.

use gameaday_2026_10_21::{
    BossBar, EnemyKind, Game, PLAYER_RADIUS, PowerKind, Vec2, WORLD_H, WORLD_W,
};

/// Integer-cast softness: blends `src` over `dst` by `t` (0..1).
fn blend(dst: u32, src: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((src >> 16) & 0xFF) as f32;
    let sg = ((src >> 8) & 0xFF) as f32;
    let sb = (src & 0xFF) as f32;
    let r = (dr + (sr - dr) * t) as u32;
    let g = (dg + (sg - dg) * t) as u32;
    let b = (db + (sb - db) * t) as u32;
    (r << 16) | (g << 8) | b
}

fn add_glow(dst: u32, color: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((color >> 16) & 0xFF) as f32 * t;
    let sg = ((color >> 8) & 0xFF) as f32 * t;
    let sb = (color & 0xFF) as f32 * t;
    let r = (dr + sr).min(255.0) as u32;
    let g = (dg + sg).min(255.0) as u32;
    let b = (db + sb).min(255.0) as u32;
    (r << 16) | (g << 8) | b
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub buf: Vec<u32>,
    /// Viewport transform: world -> screen.
    pub scale: f32,
    pub off_x: f32,
    pub off_y: f32,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Canvas {
            w,
            h,
            buf: vec![0; w * h],
            scale: 1.0,
            off_x: 0.0,
            off_y: 0.0,
        }
    }

    /// Erase the whole buffer. Called before every frame so nothing lingers.
    pub fn clear(&mut self, color: u32) {
        self.buf.iter_mut().for_each(|p| *p = color);
    }

    /// Fit the world into the window with letterboxing, then apply shake.
    pub fn set_viewport(&mut self, shake_x: f32, shake_y: f32) {
        let sx = self.w as f32 / WORLD_W;
        let sy = self.h as f32 / WORLD_H;
        self.scale = sx.min(sy);
        self.off_x = (self.w as f32 - WORLD_W * self.scale) * 0.5 - shake_x;
        self.off_y = (self.h as f32 - WORLD_H * self.scale) * 0.5 - shake_y;
    }

    #[inline]
    fn to_screen(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.scale + self.off_x, y * self.scale + self.off_y)
    }

    #[inline]
    pub fn px(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        self.buf[y as usize * self.w + x as usize] = color;
    }

    #[inline]
    pub fn px_blend(&mut self, x: i32, y: i32, color: u32, t: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        self.buf[i] = blend(self.buf[i], color, t);
    }

    #[inline]
    pub fn px_glow(&mut self, x: i32, y: i32, color: u32, t: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        self.buf[i] = add_glow(self.buf[i], color, t);
    }

    pub fn world_px_blend(&mut self, x: f32, y: f32, color: u32, t: f32) {
        let (sx, sy) = self.to_screen(x, y);
        self.px_blend(sx as i32, sy as i32, color, t);
    }

    /// Filled world-space circle with a bright core and a soft rim.
    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, color: u32) {
        let (sx, sy) = self.to_screen(cx, cy);
        let sr = r * self.scale;
        let x0 = (sx - sr).floor() as i32;
        let x1 = (sx + sr).ceil() as i32;
        let y0 = (sy - sr).floor() as i32;
        let y1 = (sy + sr).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 - sx;
                let dy = y as f32 - sy;
                let d = (dx * dx + dy * dy).sqrt();
                if d <= sr {
                    // Soft edge over the outer 22% of the radius.
                    let edge = ((sr - d) / (sr * 0.22 + 0.001)).clamp(0.0, 1.0);
                    if edge >= 1.0 {
                        self.px(x, y, color);
                    } else {
                        self.px_blend(x, y, color, edge);
                    }
                }
            }
        }
    }

    /// Hollow ring (used for the energy pulse shockwave).
    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, thick: f32, color: u32, alpha: f32) {
        let (sx, sy) = self.to_screen(cx, cy);
        let sr = r * self.scale;
        let st = (thick * self.scale).max(1.0);
        let x0 = (sx - sr - st).floor() as i32;
        let x1 = (sx + sr + st).ceil() as i32;
        let y0 = (sy - sr - st).floor() as i32;
        let y1 = (sy + sr + st).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 - sx;
                let dy = y as f32 - sy;
                let d = (dx * dx + dy * dy).sqrt();
                let delta = (d - sr).abs();
                if delta <= st {
                    let a = (1.0 - delta / st) * alpha;
                    self.px_glow(x, y, color, a);
                }
            }
        }
    }

    /// Rotated triangle used for the player hull.
    fn tri(&mut self, p0: Vec2, p1: Vec2, p2: Vec2, color: u32, alpha: f32) {
        let pts = [p0, p1, p2];
        let mut minx = f32::MAX;
        let mut maxx = f32::MIN;
        let mut miny = f32::MAX;
        let mut maxy = f32::MIN;
        let mut spts = [(0.0f32, 0.0f32); 3];
        for (i, p) in pts.iter().enumerate() {
            let (sx, sy) = self.to_screen(p.x, p.y);
            spts[i] = (sx, sy);
            minx = minx.min(sx);
            maxx = maxx.max(sx);
            miny = miny.min(sy);
            maxy = maxy.max(sy);
        }
        let x0 = minx.floor() as i32;
        let x1 = maxx.ceil() as i32;
        let y0 = miny.floor() as i32;
        let y1 = maxy.ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if point_in_tri(x as f32 + 0.5, y as f32 + 0.5, spts) {
                    if alpha >= 1.0 {
                        self.px(x, y, color);
                    } else {
                        self.px_blend(x, y, color, alpha);
                    }
                }
            }
        }
    }

    /// Filled rotated rectangle (stroked rect outline helper).
    fn rect_fill(&mut self, cx: f32, cy: f32, hw: f32, hh: f32, color: u32, alpha: f32) {
        let x0 = (cx - hw).floor() as i32;
        let x1 = (cx + hw).ceil() as i32;
        let y0 = (cy - hh).floor() as i32;
        let y1 = (cy + hh).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (x as f32 + 0.5) >= cx - hw
                    && (x as f32 + 0.5) <= cx + hw
                    && (y as f32 + 0.5) >= cy - hh
                    && (y as f32 + 0.5) <= cy + hh
                {
                    if alpha >= 1.0 {
                        self.px(x, y, color);
                    } else {
                        self.px_blend(x, y, color, alpha);
                    }
                }
            }
        }
    }

    /// Arena-aligned rect in world coords (used for HUD framing).
    pub fn world_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32, alpha: f32) {
        let (sx0, sy0) = self.to_screen(x, y);
        let (sx1, sy1) = self.to_screen(x + w, y + h);
        let cx = (sx0 + sx1) * 0.5;
        let cy = (sy0 + sy1) * 0.5;
        self.rect_fill(cx, cy, (sx1 - sx0) * 0.5, (sy1 - sy0) * 0.5, color, alpha);
    }

    pub fn screen_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32, alpha: f32) {
        self.rect_fill(x + w * 0.5, y + h * 0.5, w * 0.5, h * 0.5, color, alpha);
    }
}

fn point_in_tri(x: f32, y: f32, t: [(f32, f32); 3]) -> bool {
    let sign = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| {
        (p.0 - r.0) * (q.1 - r.1) - (q.0 - r.0) * (p.1 - r.1)
    };
    let d1 = sign((x, y), t[0], t[1]);
    let d2 = sign((x, y), t[1], t[2]);
    let d3 = sign((x, y), t[2], t[0]);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

// ---------------------------------------------------------------------------
// Animated sprites
// ---------------------------------------------------------------------------

/// Player ship: a chevron hull with engine flame, animated bat-wing tips.
pub fn draw_player(c: &mut Canvas, g: &Game) {
    // Invulnerability blink: skip alternate frames.
    if g.invuln > 0.0 && ((g.time * 18.0) as i32) % 2 == 0 {
        return;
    }
    draw_ship_at(c, g.player_pos, g.aim, g.time, g.dash_time > 0.0);
}

/// Ship drawing with explicit state, so menus can pose it without a Game.
pub fn draw_ship_at(c: &mut Canvas, p: Vec2, aim: Vec2, time: f32, dashing: bool) {
    let ang = aim.y.atan2(aim.x);
    let (s, co) = ang.sin_cos();
    let rot =
        |lx: f32, ly: f32| -> Vec2 { Vec2::new(p.x + lx * co - ly * s, p.y + lx * s + ly * co) };

    // Engine flame, animated length.
    let flame = 10.0 + (time * 30.0).sin().abs() * 8.0;
    let thr = if dashing { 0x9FE8FF } else { 0xFFB24D };
    let t1 = rot(-14.0, -5.0);
    let t2 = rot(-14.0, 5.0);
    let t3 = rot(-14.0 - flame, 0.0);
    c.tri(t1, t2, t3, thr, 0.95);

    // Hull.
    let n = rot(PLAYER_RADIUS + 4.0, 0.0);
    let a = rot(-8.0, -PLAYER_RADIUS);
    let b = rot(-8.0, PLAYER_RADIUS);
    c.tri(n, a, b, 0xEAF6FF, 1.0);
    // Inner cockpit accent.
    let cn = rot(6.0, 0.0);
    let ca = rot(-4.0, -6.0);
    let cb = rot(-4.0, 6.0);
    c.tri(cn, ca, cb, 0x5AC8FF, 1.0);

    // Wing tips animate outward as a pulse.
    let wob = 1.0 + (time * 6.0).sin() * 0.15;
    let wl = rot(-2.0, -PLAYER_RADIUS * wob);
    let wr = rot(-2.0, PLAYER_RADIUS * wob);
    c.tri(a, wl, rot(-10.0, -4.0), 0x8FD0FF, 1.0);
    c.tri(b, wr, rot(-10.0, 4.0), 0x8FD0FF, 1.0);
}

/// Enemies with a distinct silhouette per kind plus a spawn-in animation.
pub fn draw_enemy(
    c: &mut Canvas,
    kind: EnemyKind,
    pos: Vec2,
    flash: f32,
    spawn_t: f32,
    t: f32,
    seed: u32,
) {
    let color = gameaday_2026_10_21::kind_color(kind);
    let pop = if spawn_t > 0.0 {
        // Grow in from 40% as the enemy materialises.
        0.4 + 0.6 * (1.0 - spawn_t / 0.6).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let r = kind.radius() * pop;
    let body = if flash > 0.0 { 0xFFFFFF } else { color };
    let phase = t * 3.0 + seed as f32 * 0.001;

    match kind {
        EnemyKind::Drifter => {
            // Pulsing blob with orbiting spikes.
            c.circle(pos.x, pos.y, r, body);
            c.circle(pos.x, pos.y, r * 0.55, blend(body, 0xFFFFFF, 0.35));
            let spikes = 5;
            for i in 0..spikes {
                let a = phase * 0.6 + i as f32 * std::f32::consts::TAU / spikes as f32;
                let sx = pos.x + a.cos() * (r + 4.0);
                let sy = pos.y + a.sin() * (r + 4.0);
                c.circle(sx, sy, 2.2, body);
            }
        }
        EnemyKind::Darter => {
            // Sharp arrow that flips toward travel direction.
            let a = phase * 2.0;
            let (s, co) = a.sin_cos();
            let rot =
                |lx: f32, ly: f32| Vec2::new(pos.x + lx * co - ly * s, pos.y + lx * s + ly * co);
            let n = rot(r + 4.0, 0.0);
            let p1 = rot(-r * 0.7, -r * 0.9);
            let p2 = rot(-r * 0.7, r * 0.9);
            c.tri(n, p1, p2, body, 1.0);
            c.tri(rot(-2.0, 0.0), p1, p2, blend(body, 0x000000, 0.3), 1.0);
        }
        EnemyKind::Bloater => {
            // Heavy ringed hulk with a slowly rotating outer band.
            c.circle(pos.x, pos.y, r, body);
            c.circle(pos.x, pos.y, r * 0.7, blend(body, 0x000000, 0.25));
            let ring_r = r + 3.0 + (phase).sin() * 2.0;
            c.ring(pos.x, pos.y, ring_r, 2.0, 0xD9C6FF, 0.9);
            for i in 0..6 {
                let a = phase * 0.4 + i as f32 * std::f32::consts::TAU / 6.0;
                c.circle(
                    pos.x + a.cos() * ring_r,
                    pos.y + a.sin() * ring_r,
                    2.5,
                    0xD9C6FF,
                );
            }
        }
        EnemyKind::Warden => {
            // Boss: layered hexagon shells, counter-rotating.
            for layer in 0..3 {
                let lr = r - layer as f32 * 9.0;
                let sides = 6;
                let off =
                    phase * (0.3 + layer as f32 * 0.35) * if layer % 2 == 0 { 1.0 } else { -1.0 };
                let pts: Vec<Vec2> = (0..sides)
                    .map(|i| {
                        let a = off + i as f32 * std::f32::consts::TAU / sides as f32;
                        Vec2::new(pos.x + a.cos() * lr, pos.y + a.sin() * lr)
                    })
                    .collect();
                let col = match layer {
                    0 => body,
                    1 => blend(body, 0xFFFFFF, 0.25),
                    _ => blend(body, 0xFF4D6D, 0.4),
                };
                for i in 0..sides {
                    let p0 = pts[i];
                    let p1 = pts[(i + 1) % sides];
                    let ctr = Vec2::new(pos.x, pos.y);
                    c.tri(p0, p1, ctr, col, if layer == 0 { 1.0 } else { 0.9 });
                }
            }
            // Menacing eye, blinking.
            if (phase * 0.8).sin() > -0.6 {
                c.circle(pos.x, pos.y, 5.0, 0xFF2E4D);
                c.circle(pos.x, pos.y, 2.4, 0xFFE0E6);
            }
        }
    }
}

pub fn draw_orb(c: &mut Canvas, pos: Vec2, anim: f32, life: f32) {
    let fade = if life < 2.0 {
        (life / 2.0).clamp(0.2, 1.0)
    } else {
        1.0
    };
    let pulse = 5.0 + (anim).sin() * 1.5;
    c.ring(
        pos.x,
        pos.y,
        9.0 + (anim * 0.5).sin() * 2.0,
        1.5,
        0xFFE066,
        0.5 * fade,
    );
    c.circle(pos.x, pos.y, pulse, 0xFFE066);
    c.circle(pos.x, pos.y, pulse * 0.45, 0xFFFFFF);
}

pub fn draw_powerup(c: &mut Canvas, pos: Vec2, kind: PowerKind, anim: f32, life: f32) {
    let fade = if life < 3.0 {
        (life / 3.0).clamp(0.25, 1.0)
    } else {
        1.0
    };
    let col = kind.color();
    let r = 11.0;
    let bob = (anim).sin() * 2.0;
    let p = Vec2::new(pos.x, pos.y + bob);
    c.ring(p.x, p.y, r + 3.0, 2.0, col, 0.6 * fade);
    c.circle(p.x, p.y, r, col);
    c.circle(p.x, p.y, r * 0.6, blend(col, 0x000000, 0.45));
    // A glyph that hints at the effect.
    match kind {
        PowerKind::Repair => {
            c.rect_fill(p.x, p.y, 2.0, 4.5, 0x0A1014, 1.0);
            c.rect_fill(p.x, p.y, 4.5, 2.0, 0x0A1014, 1.0);
        }
        PowerKind::RapidFire => {
            c.rect_fill(p.x - 2.0, p.y + 1.5, 2.6, 1.6, 0x0A1014, 1.0);
            c.rect_fill(p.x + 1.5, p.y - 2.0, 2.6, 1.6, 0x0A1014, 1.0);
        }
        PowerKind::Charge => {
            c.circle(p.x, p.y, 2.2, 0x0A1014);
        }
    }
}

/// Simple blocky 5x7 pixel font for the HUD and menus.
pub fn draw_text(c: &mut Canvas, x: f32, y: f32, s: &str, scale: f32, color: u32) {
    let mut cx = x;
    for ch in s.chars() {
        if ch == ' ' {
            cx += 4.0 * scale;
            continue;
        }
        let glyph = glyph(ch);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    let px = cx + col as f32 * scale;
                    let py = y + row as f32 * scale;
                    c.screen_rect(px, py, scale, scale, color, 1.0);
                }
            }
        }
        cx += 6.0 * scale;
    }
}

pub fn text_width(s: &str, scale: f32) -> f32 {
    s.chars().count() as f32 * 6.0 * scale
}

/// Centered text helper.
pub fn draw_text_centered(c: &mut Canvas, cx: f32, y: f32, s: &str, scale: f32, color: u32) {
    let w = text_width(s, scale);
    draw_text(c, cx - w * 0.5, y, s, scale, color);
}

fn glyph(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        '3' => [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '!' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
        '?' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100,
        ],
        '.' => [0, 0, 0, 0, 0, 0, 0b00100],
        ',' => [0, 0, 0, 0, 0, 0b00100, 0b01000],
        ':' => [0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '%' => [
            0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011,
        ],
        '\'' => [0b00100, 0b00100, 0, 0, 0, 0, 0],
        _ => [
            0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
        ],
    }
}

/// Draws the full HUD: health, energy, score, wave, combo, boss bar.
pub fn draw_hud(c: &mut Canvas, g: &Game) {
    let pad = 16.0;
    // Health bar.
    let hw = 240.0;
    let hh = 14.0;
    let hy = 18.0;
    c.screen_rect(pad - 2.0, hy - 2.0, hw + 4.0, hh + 4.0, 0x000000, 0.5);
    c.screen_rect(pad, hy, hw, hh, 0x2A2F3A, 1.0);
    let hp_frac = (g.health / g.max_health).clamp(0.0, 1.0);
    let hp_col = if hp_frac > 0.55 {
        0x4DE08A
    } else if hp_frac > 0.28 {
        0xFFC24D
    } else {
        0xFF5C6C
    };
    c.screen_rect(pad, hy, hw * hp_frac, hh, hp_col, 1.0);
    draw_text(c, pad + 4.0, hy + 3.0, "HULL", 1.0, 0x0A1014);
    draw_text(
        c,
        pad + hw + 10.0,
        hy + 3.0,
        &format!("{}", g.health.ceil() as i32),
        1.0,
        0xEAF6FF,
    );

    // Energy bar (for the pulse).
    let ey = hy + hh + 10.0;
    c.screen_rect(pad, ey, hw, 10.0, 0x2A2F3A, 1.0);
    let e_frac = (g.energy / g.max_energy).clamp(0.0, 1.0);
    let e_col = if e_frac >= 1.0 { 0x8FE9FF } else { 0x3E7FA8 };
    c.screen_rect(pad, ey, hw * e_frac, 10.0, e_col, 1.0);
    let label = if e_frac >= 1.0 {
        "PULSE READY"
    } else {
        "PULSE"
    };
    draw_text(c, pad + 4.0, ey + 2.0, label, 1.0, 0x0A1014);

    // Score / wave, top right.
    let sr = format!("SCORE {}", g.score);
    let sw = text_width(&sr, 2.0);
    draw_text(c, c.w as f32 - pad - sw, 16.0, &sr, 2.0, 0xEAF6FF);
    let wr = format!("WAVE {}", g.wave);
    draw_text(
        c,
        c.w as f32 - pad - text_width(&wr, 1.2),
        44.0,
        &wr,
        1.2,
        0x9FE8FF,
    );
    let br = format!("BEST {}", g.best_score.max(g.score));
    draw_text(
        c,
        c.w as f32 - pad - text_width(&br, 1.0),
        62.0,
        &br,
        1.0,
        0x8A93A6,
    );

    // Combo with a shrinking timer bar.
    if g.combo > 1 {
        let mult = g.combo_multiplier();
        let txt = format!("X{} COMBO {}", mult, g.combo);
        let tw = text_width(&txt, 1.4);
        let tx = c.w as f32 * 0.5 - tw * 0.5;
        let col = if mult >= 3 { 0xFFE066 } else { 0xFFFFFF };
        draw_text(c, tx, c.h as f32 - 46.0, &txt, 1.4, col);
        let frac = (g.combo_timer / 2.5).clamp(0.0, 1.0);
        c.screen_rect(tx, c.h as f32 - 30.0, tw * frac, 3.0, col, 0.9);
    }

    // Wave banner.
    if g.wave_banner > 0.0 {
        let a = (g.wave_banner.min(1.0)).clamp(0.0, 1.0);
        let msg = if g.is_boss_wave() {
            "!! WARDEN INCOMING !!"
        } else {
            "WAVE"
        };
        let y = c.h as f32 * 0.32;
        if g.is_boss_wave() {
            draw_text_centered(c, c.w as f32 * 0.5, y, msg, 2.4, 0xD7B3FF);
        } else {
            draw_text_centered(c, c.w as f32 * 0.5, y, msg, 2.4, 0xEAF6FF);
            let num = format!("{}", g.wave);
            draw_text_centered(c, c.w as f32 * 0.5, y + 30.0, &num, 3.0, 0x9FE8FF);
        }
        let _ = a;
    }

    // Boss health bar.
    if let Some(bar) = boss_bar(g) {
        let bw = c.w as f32 * 0.55;
        let bx = (c.w as f32 - bw) * 0.5;
        let by = c.h as f32 - 70.0;
        c.screen_rect(bx - 2.0, by - 2.0, bw + 4.0, 14.0, 0x000000, 0.55);
        c.screen_rect(bx, by, bw, 10.0, 0x2A2F3A, 1.0);
        let frac = (bar.hp as f32 / bar.max_hp as f32).clamp(0.0, 1.0);
        c.screen_rect(bx, by, bw * frac, 10.0, 0xD7B3FF, 1.0);
        let t = "WARDEN";
        draw_text_centered(c, c.w as f32 * 0.5, by - 16.0, t, 1.2, 0xEAD9FF);
    }

    // Pickup banner.
    if g.pickup_banner > 0.0 {
        let a = (g.pickup_banner / 1.8).clamp(0.0, 1.0);
        let y = c.h as f32 * 0.5 - 18.0;
        let col = blend(0x000000, 0xEAF6FF, a);
        draw_text_centered(c, c.w as f32 * 0.5, y, g.pickup_label, 1.6, col);
    }

    // Dash cooldown pip next to the ship's HUD block.
    let dy = ey + 16.0;
    let frac = 1.0 - (g.dash_cooldown / gameaday_2026_10_21::DASH_COOLDOWN).clamp(0.0, 1.0);
    c.screen_rect(pad, dy, 60.0, 6.0, 0x2A2F3A, 1.0);
    c.screen_rect(pad, dy, 60.0 * frac, 6.0, 0x7BE0FF, 1.0);
    draw_text(c, pad + 66.0, dy - 1.0, "DASH", 1.0, 0x8A93A6);
}

pub fn boss_bar(g: &Game) -> Option<BossBar> {
    g.enemies
        .iter()
        .find(|e| e.kind.is_boss())
        .map(|e| BossBar {
            hp: e.hp.max(0),
            max_hp: e.kind.hp() + (g.wave as i32 / 5) * 14,
        })
}

/// Parallax starfield drawn behind everything.
pub struct Starfield {
    pub stars: Vec<(f32, f32, f32, u32)>,
    pub dust: Vec<(f32, f32, f32)>,
}

impl Starfield {
    pub fn new(n: usize) -> Self {
        let mut r = gameaday_2026_10_21::Rng32::new(0xC0FFEE);
        let mut stars = Vec::new();
        for _ in 0..n {
            stars.push((
                r.range(0.0, WORLD_W),
                r.range(0.0, WORLD_H),
                r.range(0.4, 2.0),
                r.next_u32() % 3,
            ));
        }
        let mut dust = Vec::new();
        for _ in 0..40 {
            dust.push((
                r.range(0.0, WORLD_W),
                r.range(0.0, WORLD_H),
                r.range(20.0, 70.0),
            ));
        }
        Starfield { stars, dust }
    }

    pub fn draw(&self, c: &mut Canvas, t: f32) {
        for (x, y, size, tint) in &self.stars {
            let tw = (t * 1.5 + *x * 0.02).sin() * 0.5 + 0.5;
            let col = match tint {
                0 => 0x8A93A6,
                1 => 0x5AC8FF,
                _ => 0xFFE066,
            };
            let a = 0.25 + tw * 0.5;
            if *size > 1.4 {
                c.world_px_blend(*x, *y, col, a);
                c.world_px_blend(*x + 1.0, *y, col, a * 0.5);
            } else {
                c.world_px_blend(*x, *y, col, a);
            }
        }
        // Slow-drifting nebula dust for depth.
        for (x, y, r) in &self.dust {
            let ox = (t * 6.0 + *x).rem_euclid(WORLD_W + 200.0) - 100.0;
            c.ring(ox, *y, *r, 1.0, 0x2A3B5A, 0.25);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gameaday_2026_10_21::{EnemyKind, PowerKind, Vec2, WORLD_H, WORLD_W};

    #[test]
    fn clear_then_write_leaves_no_untouched_pixels() {
        let mut c = Canvas::new(64, 48);
        c.clear(0x05070D);
        c.set_viewport(0.0, 0.0);
        c.world_rect(0.0, 0.0, WORLD_W, WORLD_H, 0x0A0E18, 1.0);
        assert!(c.buf.iter().all(|p| *p != 0), "buffer had stale pixels");
    }

    #[test]
    fn world_to_screen_maps_the_arena_into_the_window() {
        let mut c = Canvas::new(1000, 690);
        c.set_viewport(0.0, 0.0);
        assert!(c.scale > 0.0);
        // Centre of the world lands near the centre of the window.
        let (sx, sy) = c.to_screen(WORLD_W * 0.5, WORLD_H * 0.5);
        assert!((sx - 500.0).abs() < 1.0);
        assert!((sy - 345.0).abs() < 1.0);
    }

    #[test]
    fn shake_offsets_the_viewport() {
        let mut a = Canvas::new(400, 300);
        a.set_viewport(0.0, 0.0);
        let mut b = Canvas::new(400, 300);
        b.set_viewport(10.0, -6.0);
        assert!((a.off_x - b.off_x - 10.0).abs() < 0.001);
        assert!((a.off_y - b.off_y + 6.0).abs() < 0.001);
    }

    #[test]
    fn text_width_scales_with_length_and_size() {
        assert!(text_width("AB", 1.0) > text_width("A", 1.0));
        assert!(text_width("A", 2.0) > text_width("A", 1.0));
        assert!(text_width("", 1.0) == 0.0);
    }

    #[test]
    fn every_enemy_kind_draws_something() {
        for kind in [
            EnemyKind::Drifter,
            EnemyKind::Darter,
            EnemyKind::Bloater,
            EnemyKind::Warden,
        ] {
            let mut c = Canvas::new(200, 200);
            c.clear(0x05070D);
            c.set_viewport(0.0, 0.0);
            draw_enemy(
                &mut c,
                kind,
                Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5),
                0.0,
                0.0,
                1.0,
                7,
            );
            let changed = c.buf.iter().filter(|p| **p != 0x05070D).count();
            assert!(changed > 0, "{kind:?} drew nothing");
        }
    }

    #[test]
    fn powerups_draw_all_kinds() {
        for kind in [PowerKind::Repair, PowerKind::RapidFire, PowerKind::Charge] {
            let mut c = Canvas::new(200, 200);
            c.clear(0x05070D);
            c.set_viewport(0.0, 0.0);
            draw_powerup(
                &mut c,
                Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5),
                kind,
                0.0,
                5.0,
            );
            let changed = c.buf.iter().filter(|p| **p != 0x05070D).count();
            assert!(changed > 0, "{kind:?} drew nothing");
        }
    }

    #[test]
    fn text_glyphs_are_five_by_seven() {
        for ch in "NEBULA DRIFT 0123456789!?:.,-+/%'".chars() {
            let g = glyph(ch);
            assert_eq!(g.len(), 7);
            // No glyph row may use bits beyond the 5px width.
            assert!(g.iter().all(|r| *r <= 0b11111), "glyph {ch:?} too wide");
        }
    }
}
