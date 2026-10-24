//! Parallax backdrops, one per biome, drawn behind the world.

use super::canvas::{Canvas, H, W, hash2, lerp_color, pack, scale, split};
use super::palette::Pal;
use crate::level::Biome;

fn vnoise(x: f32, seed: i32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let a = (hash2(i as i32, seed) & 0xffff) as f32 / 65535.0;
    let b = (hash2(i as i32 + 1, seed) & 0xffff) as f32 / 65535.0;
    let t = f * f * (3.0 - 2.0 * f);
    a + (b - a) * t
}

fn fbm(x: f32, seed: i32) -> f32 {
    vnoise(x, seed) * 0.6 + vnoise(x * 2.3, seed + 7) * 0.3 + vnoise(x * 5.1, seed + 13) * 0.1
}

/// A silhouette ridge scrolling at `speed` of the camera.
#[allow(clippy::too_many_arguments)]
fn ridge(
    c: &mut Canvas,
    cam_x: f32,
    cam_y: f32,
    speed: f32,
    base_y: f32,
    amp: f32,
    freq: f32,
    seed: i32,
    top: u32,
    bot: u32,
    spiky: bool,
) {
    let w = W as i32;
    let off_y = -cam_y * speed * 0.5;
    let mut lut = [0u32; 64];
    for (i, l) in lut.iter_mut().enumerate() {
        *l = lerp_color(top, bot, i as f32 / 63.0);
    }
    for x in 0..w {
        let u = (x as f32 + cam_x * speed) * freq;
        let mut h = fbm(u, seed);
        if spiky {
            h = (h - 0.5).abs() * 2.0;
            h = 1.0 - h;
            h = h * h;
        }
        let y0 = (base_y + off_y - h * amp).max(0.0) as i32;
        let span = (H as i32 - y0).max(1);
        for y in y0..H as i32 {
            let idx = (((y - y0) * 63) / span).min(63) as usize;
            c.px[y as usize * W + x as usize] = lut[idx];
        }
    }
}

fn sky(c: &mut Canvas, pal: &Pal, cam_y: f32) {
    let shift = (cam_y * 0.0003).clamp(-0.3, 0.3);
    for y in 0..H {
        let t = (y as f32 / H as f32 - shift).clamp(0.0, 1.0);
        let col = if t < 0.55 {
            lerp_color(pal.sky_top, pal.sky_mid, t / 0.55)
        } else {
            lerp_color(pal.sky_mid, pal.sky_bot, (t - 0.55) / 0.45)
        };
        let row = &mut c.px[y * W..(y + 1) * W];
        row.fill(col);
    }
}

pub fn draw_background(c: &mut Canvas, biome: Biome, pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    sky(c, pal, cam_y);
    match biome {
        Biome::Mossbluff => mossbluff(c, pal, cam_x, cam_y, time),
        Biome::Glimmerdeep => glimmerdeep(c, pal, cam_x, cam_y, time),
        Biome::Frostspire => frostspire(c, pal, cam_x, cam_y, time),
        Biome::Cinderworks => cinderworks(c, pal, cam_x, cam_y, time),
        Biome::Stormcrown => stormcrown(c, pal, cam_x, cam_y, time),
        Biome::Beacon => beacon(c, pal, cam_x, cam_y, time),
    }
}

struct CloudTex {
    alpha: Vec<u8>,
    w: usize,
    h: usize,
    /// Row of the texture that sits on the cloud base line.
    base: usize,
}

const CLOUD_W: usize = 3200;

fn make_clouds(count: i32, seed: i32, size: f32) -> CloudTex {
    let margin = (size * 0.7) as usize + 40;
    let h = margin * 2;
    let mut mask = vec![0.0f32; CLOUD_W * h];
    for i in 0..count {
        let wx = (hash2(i, seed) & 0xffff) as f32 / 65535.0 * CLOUD_W as f32;
        let jy = ((hash2(i, seed + 5) & 0xff) as f32 - 128.0) * 0.35;
        let s = size * (0.7 + (hash2(i, seed + 9) & 0xff) as f32 / 512.0);
        for k in 0..5 {
            let ox = (k as f32 - 2.0) * s * 0.42;
            let oy = -((k as f32 - 2.0).abs()) * s * 0.07 + ((k % 2) as f32) * s * 0.05;
            let (rx, ry) = (s * 0.55, s * 0.26);
            let cy = margin as f32 + jy + oy;
            let m = rx.min(ry);
            for wrap in [-1i32, 0, 1] {
                let cx = wx + ox + wrap as f32 * CLOUD_W as f32;
                let x0 = ((cx - rx - 1.0).floor() as i32).max(0);
                let x1 = ((cx + rx + 1.0).ceil() as i32).min(CLOUD_W as i32);
                let y0 = ((cy - ry - 1.0).floor() as i32).max(0);
                let y1 = ((cy + ry + 1.0).ceil() as i32).min(h as i32);
                for y in y0..y1 {
                    for x in x0..x1 {
                        let dx = (x as f32 + 0.5 - cx) / rx;
                        let dy = (y as f32 + 0.5 - cy) / ry;
                        let d = (dx * dx + dy * dy).sqrt();
                        let cov = ((1.0 - d) * m + 0.5).clamp(0.0, 1.0);
                        let i = y as usize * CLOUD_W + x as usize;
                        if cov > mask[i] {
                            mask[i] = cov;
                        }
                    }
                }
            }
        }
    }
    CloudTex {
        alpha: mask.iter().map(|v| (v * 255.0) as u8).collect(),
        w: CLOUD_W,
        h,
        base: margin,
    }
}

thread_local! {
    static CLOUD_CACHE: std::cell::RefCell<Vec<((i32, i32, u32), std::rc::Rc<CloudTex>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn cloud_tex(count: i32, seed: i32, size: f32) -> std::rc::Rc<CloudTex> {
    let key = (count, seed, size.to_bits());
    CLOUD_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some((_, t)) = c.iter().find(|(k, _)| *k == key) {
            return t.clone();
        }
        let t = std::rc::Rc::new(make_clouds(count, seed, size));
        c.push((key, t.clone()));
        t
    })
}

#[allow(clippy::too_many_arguments)]
fn clouds(
    c: &mut Canvas,
    cam_x: f32,
    y: f32,
    col: u32,
    a: f32,
    time: f32,
    count: i32,
    seed: i32,
    speed: f32,
    size: f32,
) {
    let tex = cloud_tex(count, seed, size);
    let scroll = (cam_x * speed - time * 18.0 * speed).rem_euclid(tex.w as f32) as usize + 400;
    let top = y as i32 - tex.base as i32;
    let (cr, cg, cb) = split(col);
    for ty in 0..tex.h {
        let sy = top + ty as i32;
        if sy < 0 || sy >= H as i32 {
            continue;
        }
        let row = &tex.alpha[ty * tex.w..(ty + 1) * tex.w];
        for x in 0..W {
            let al = row[(x + scroll) % tex.w];
            if al == 0 {
                continue;
            }
            let k = al as f32 / 255.0 * a;
            let p = &mut c.px[sy as usize * W + x];
            let (r0, g0, b0) = split(*p);
            *p = pack(r0 + (cr - r0) * k, g0 + (cg - g0) * k, b0 + (cb - b0) * k);
        }
    }
}

fn mossbluff(c: &mut Canvas, pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    // sun
    let sx = 940.0 - cam_x * 0.02;
    let sy = 190.0 - cam_y * 0.03;
    c.glow(sx, sy, 380.0, 0xffe9b0, 0.55);
    c.circle(sx, sy, 46.0, 0xfff4cc, 1.0);
    c.circle(sx, sy, 40.0, 0xffffff, 0.6);
    // slow god rays fanning out of the sun
    for k in 0..6 {
        let a = 1.75 + k as f32 * 0.16 + (time * 0.15 + k as f32).sin() * 0.03;
        let (a0, a1) = (a - 0.035, a + 0.035);
        c.poly(
            &[
                (sx, sy),
                (sx + a0.cos() * 1500.0, sy + a0.sin() * 1500.0),
                (sx + a1.cos() * 1500.0, sy + a1.sin() * 1500.0),
            ],
            0xfff4c0,
            0.05,
        );
    }
    clouds(c, cam_x, 150.0, 0xffffff, 0.35, time, 7, 11, 0.06, 150.0);
    ridge(
        c,
        cam_x,
        cam_y,
        0.08,
        470.0,
        130.0,
        0.0022,
        3,
        pal.far[0],
        scale(pal.far[0], 0.95),
        true,
    );
    clouds(c, cam_x, 330.0, 0xffffff, 0.22, time, 6, 21, 0.12, 190.0);
    ridge(
        c, cam_x, cam_y, 0.16, 520.0, 110.0, 0.0035, 8, pal.far[1], pal.fog, false,
    );
    ridge(
        c, cam_x, cam_y, 0.3, 600.0, 90.0, 0.0055, 14, 0x5a9a70, 0x3a7a58, false,
    );
    // distant trees on the nearest hills
    for i in 0..60 {
        let wx = i as f32 * 61.0 + (hash2(i, 3) & 31) as f32;
        let x = (wx - cam_x * 0.3).rem_euclid(3660.0) - 200.0;
        if !(-40.0..W as f32 + 40.0).contains(&x) {
            continue;
        }
        let u = (x + cam_x * 0.3) * 0.0055;
        let hgt = fbm(u, 14);
        let y = 600.0 - cam_y * 0.15 - hgt * 90.0;
        let s = 14.0 + (hash2(i, 9) & 7) as f32;
        c.poly(
            &[
                (x, y - s * 1.7),
                (x - s * 0.55, y + 2.0),
                (x + s * 0.55, y + 2.0),
            ],
            0x2f6a4a,
            1.0,
        );
    }
    ridge(
        c, cam_x, cam_y, 0.5, 690.0, 70.0, 0.008, 19, 0x3f8a52, 0x2b6a3e, false,
    );
}

fn glimmerdeep(c: &mut Canvas, pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    // faint glowing haze
    for i in 0..6 {
        let x = 200.0 + i as f32 * 230.0 - (cam_x * 0.05) % 230.0;
        c.glow(
            x,
            400.0 + (time * 0.6 + i as f32).sin() * 30.0,
            260.0,
            0x4a3aa8,
            0.12,
        );
    }
    // layers of crystal spires rising from below
    for (layer, (speed, base, col_top, glow_a)) in [
        (0.1f32, 560.0f32, 0x1e1c50u32, 0.0f32),
        (0.22, 620.0, 0x2a2868, 0.0),
        (0.4, 700.0, 0x34308a, 0.0),
    ]
    .iter()
    .enumerate()
    {
        ridge(
            c,
            cam_x,
            cam_y,
            *speed,
            *base,
            190.0 - layer as f32 * 30.0,
            0.004 + layer as f32 * 0.002,
            30 + layer as i32,
            *col_top,
            pal.far[2],
            true,
        );
        let _ = glow_a;
    }
    // hanging stalactite silhouettes from the top
    for x in 0..W as i32 {
        let u = (x as f32 + cam_x * 0.35) * 0.006;
        let h = (fbm(u, 77) - 0.35).max(0.0) * 260.0 - cam_y * 0.05;
        let col_a = 0x0c0b22u32;
        for y in 0..h.max(0.0) as i32 {
            c.px[y as usize * W + x as usize] = lerp_color(col_a, 0x1c1844, y as f32 / h.max(1.0));
        }
    }
    // glimmering crystal points
    for i in 0..70 {
        let wx = (hash2(i, 5) & 0xfff) as f32 * 1.4;
        let wy = (hash2(i, 6) & 0x3ff) as f32 * 0.6 + 60.0;
        let x = (wx - cam_x * 0.2).rem_euclid(5800.0) - 300.0;
        let y = wy - cam_y * 0.1;
        if x < -20.0 || x > W as f32 + 20.0 || y < -20.0 || y > H as f32 {
            continue;
        }
        let tw = ((time * 1.7 + i as f32 * 1.3).sin() * 0.5 + 0.5) * 0.9 + 0.1;
        let col = if i % 3 == 0 {
            0x62f2ff
        } else if i % 3 == 1 {
            0xc77dff
        } else {
            0x9bb0ff
        };
        c.glow(x, y, 16.0 + (i % 4) as f32 * 5.0, col, 0.7 * tw);
        c.circle(x, y, 1.6, 0xffffff, tw);
    }
}

fn frostspire(c: &mut Canvas, _pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    // aurora ribbons
    for k in 0..3 {
        let base = 120.0 + k as f32 * 60.0;
        let col = [0x7cf0d0u32, 0x8aa8ff, 0xc8a0ff][k];
        for x in (0..W as i32).step_by(3) {
            let u = (x as f32 + cam_x * 0.04) * 0.004 + time * 0.12 + k as f32 * 3.0;
            let y = base + (u * 2.0).sin() * 40.0 + vnoise(u * 1.5, 40 + k as i32) * 60.0;
            let hgt = 70.0 + vnoise(u * 3.0, 50 + k as i32) * 70.0;
            for dy in 0..hgt as i32 {
                let a = (1.0 - dy as f32 / hgt) * 0.14;
                for xx in 0..3 {
                    c.add(x + xx, y as i32 + dy, col, a);
                }
            }
        }
    }
    clouds(c, cam_x, 200.0, 0xffffff, 0.35, time, 6, 12, 0.05, 160.0);
    ridge(
        c, cam_x, cam_y, 0.08, 480.0, 190.0, 0.0028, 4, 0xdcecf8, 0xaccae4, true,
    );
    ridge(
        c, cam_x, cam_y, 0.16, 560.0, 150.0, 0.0038, 9, 0xb4cce6, 0x92b2d4, true,
    );
    // snow caps highlight
    ridge(
        c, cam_x, cam_y, 0.3, 640.0, 120.0, 0.0055, 15, 0x86a6c8, 0x6a8ab0, true,
    );
    ridge(
        c, cam_x, cam_y, 0.5, 720.0, 80.0, 0.008, 22, 0x6a8cb4, 0x4e6e96, false,
    );
    // pines
    for i in 0..90 {
        let wx = i as f32 * 47.0 + (hash2(i, 3) & 31) as f32;
        let x = (wx - cam_x * 0.5).rem_euclid(4230.0) - 200.0;
        if !(-40.0..W as f32 + 40.0).contains(&x) {
            continue;
        }
        let u = (x + cam_x * 0.5) * 0.008;
        let y = 720.0 - cam_y * 0.25 - fbm(u, 22) * 80.0;
        let s = 14.0 + (hash2(i, 9) & 7) as f32;
        for k in 0..3 {
            let yy = y - k as f32 * s * 0.55;
            let w = s * (0.8 - k as f32 * 0.2);
            c.poly(
                &[(x, yy - s * 0.9), (x - w, yy), (x + w, yy)],
                0x3a5a7c,
                1.0,
            );
            c.poly(
                &[
                    (x, yy - s * 0.9),
                    (x - w * 0.5, yy - s * 0.4),
                    (x + w * 0.5, yy - s * 0.4),
                ],
                0xe8f4ff,
                0.7,
            );
        }
    }
}

fn cinderworks(c: &mut Canvas, pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    // red horizon glow
    c.glow(
        640.0 - cam_x * 0.02,
        700.0 - cam_y * 0.05,
        700.0,
        0xff5a20,
        0.5,
    );
    // volcano
    let vx = 880.0 - cam_x * 0.08;
    let vy = 640.0 - cam_y * 0.05;
    c.poly(
        &[
            (vx - 360.0, vy + 200.0),
            (vx - 60.0, vy - 130.0),
            (vx + 60.0, vy - 130.0),
            (vx + 380.0, vy + 200.0),
        ],
        0x2a0c0c,
        1.0,
    );
    c.glow(vx, vy - 130.0, 140.0, 0xff7a30, 0.8);
    for i in 0..5 {
        let t = (time * 0.2 + i as f32 * 0.2) % 1.0;
        c.glow(
            vx + (i as f32 - 2.0) * 14.0 + t * 20.0,
            vy - 140.0 - t * 280.0,
            40.0 + t * 90.0,
            0x5a2a20,
            0.5 * (1.0 - t),
        );
    }
    ridge(
        c, cam_x, cam_y, 0.12, 600.0, 160.0, 0.003, 5, 0x40100c, pal.far[2], true,
    );
    ridge(
        c, cam_x, cam_y, 0.25, 680.0, 130.0, 0.0045, 10, 0x2e0a08, 0x1c0605, true,
    );
    // lava lake glow at the bottom
    for y in (H as i32 - 180)..H as i32 {
        let t = (y - (H as i32 - 180)) as f32 / 180.0;
        for x in 0..W as i32 {
            c.add(
                x,
                y,
                0xff5a10,
                t * t * 0.55 * (0.8 + 0.2 * ((x as f32 * 0.02 + time * 1.5).sin())),
            );
        }
    }
}

fn stormcrown(c: &mut Canvas, _pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    // lightning flash every so often
    let flash_phase = (time * 0.37).fract();
    let flash = if flash_phase < 0.03 {
        1.0 - flash_phase / 0.03
    } else {
        0.0
    };
    if flash > 0.0 {
        for y in 0..H {
            for x in 0..W {
                let p = &mut c.px[y * W + x];
                let (r, g, b) = split(*p);
                *p = pack(r + 70.0 * flash, g + 60.0 * flash, b + 90.0 * flash);
            }
        }
    }
    clouds(c, cam_x, 160.0, 0x2a2060, 0.55, time, 9, 31, 0.04, 330.0);
    clouds(c, cam_x, 300.0, 0x4a3a88, 0.4, time, 8, 32, 0.08, 300.0);
    ridge(
        c, cam_x, cam_y, 0.1, 560.0, 200.0, 0.003, 6, 0x30286a, 0x1e1848, true,
    );
    // distant spires with beacon lights
    for i in 0..8 {
        let wx = i as f32 * 520.0 + 100.0;
        let x = (wx - cam_x * 0.16).rem_euclid(4160.0) - 200.0;
        let y = 640.0 - cam_y * 0.08;
        c.poly(
            &[
                (x - 18.0, y + 300.0),
                (x - 6.0, y - 150.0 - (i % 3) as f32 * 40.0),
                (x + 6.0, y - 150.0 - (i % 3) as f32 * 40.0),
                (x + 18.0, y + 300.0),
            ],
            0x1a1440,
            1.0,
        );
        let on = ((time * 2.0 + i as f32).sin() * 0.5 + 0.5) * 0.8 + 0.2;
        c.glow(
            x,
            y - 150.0 - (i % 3) as f32 * 40.0,
            22.0,
            0xffe566,
            on * 0.8,
        );
    }
    ridge(
        c, cam_x, cam_y, 0.28, 660.0, 130.0, 0.0045, 12, 0x241c58, 0x160f3c, false,
    );
    clouds(c, cam_x, 560.0, 0x6a5aa8, 0.28, time, 7, 33, 0.18, 360.0);
    if flash > 0.0 {
        // a bolt in the distance
        let bx = 300.0 + (hash2((time * 0.37).floor() as i32, 4) & 0x3ff) as f32;
        let mut x = bx;
        let mut y = 0.0;
        for k in 0..14 {
            let nx = x + ((hash2(k, (time * 0.37).floor() as i32) & 63) as f32 - 32.0);
            let ny = y + 40.0;
            c.line(x, y, nx, ny, 3.0, 0xffffff, flash);
            c.glow(nx, ny, 30.0, 0xc8b8ff, 0.4 * flash);
            x = nx;
            y = ny;
        }
    }
}

fn vortex_tex() -> std::rc::Rc<(Vec<u8>, usize, usize)> {
    thread_local! {
        static V: std::cell::RefCell<Option<std::rc::Rc<(Vec<u8>, usize, usize)>>> = const { std::cell::RefCell::new(None) };
    }
    V.with(|v| {
        let mut v = v.borrow_mut();
        if let Some(t) = v.as_ref() {
            return t.clone();
        }
        let (tw, th) = (1900usize, 800usize);
        let mut cv = Canvas::new(tw, th);
        for ring in 0..9 {
            let r = 90.0 + ring as f32 * 60.0;
            cv.ring(
                tw as f32 / 2.0,
                th as f32 / 2.0,
                r * 1.5,
                r * 0.55,
                18.0,
                0xffffff,
                1.0,
            );
        }
        let alpha: Vec<u8> = cv.px.iter().map(|p| (p & 0xff) as u8).collect();
        let t = std::rc::Rc::new((alpha, tw, th));
        *v = Some(t.clone());
        t
    })
}

fn beacon(c: &mut Canvas, _pal: &Pal, cam_x: f32, cam_y: f32, time: f32) {
    for i in 0..140 {
        let x = (hash2(i, 1) & 0x7ff) as f32 * 0.7;
        let y = (hash2(i, 2) & 0x3ff) as f32 * 0.55;
        let sx = (x - cam_x * 0.03).rem_euclid(1500.0) - 100.0;
        let tw = ((time * 2.0 + i as f32).sin() * 0.5 + 0.5) * 0.7 + 0.3;
        c.circle(sx, y - cam_y * 0.02, 1.2, 0xffffff, tw * 0.8);
    }
    // swirling storm vortex around the beacon spire (cached rings, additive)
    let vx = 640.0 - cam_x * 0.03;
    let vy = 260.0 - cam_y * 0.04;
    let tex = vortex_tex();
    let (alpha, tw, th) = (&tex.0, tex.1, tex.2);
    let ox = vx as i32 - tw as i32 / 2;
    let oy = vy as i32 - th as i32 / 2;
    let pulse = 0.10 + 0.03 * (time * 0.8).sin();
    for ty in 0..th as i32 {
        let sy = oy + ty;
        if sy < 0 || sy >= H as i32 {
            continue;
        }
        for tx in 0..tw as i32 {
            let sx = ox + tx;
            if sx < 0 || sx >= W as i32 {
                continue;
            }
            let al = alpha[ty as usize * tw + tx as usize];
            if al == 0 {
                continue;
            }
            let k = al as f32 / 255.0 * pulse;
            let p = &mut c.px[sy as usize * W + sx as usize];
            let (r0, g0, b0) = split(*p);
            *p = pack(
                r0 + (0x6a as f32 - r0) * k,
                g0 + (0x3a as f32 - g0) * k,
                b0 + (0x9a as f32 - b0) * k,
            );
        }
    }
    c.glow(vx, vy, 340.0, 0xb06aff, 0.35);
    // the Beacon: a dark tower waiting to be lit
    c.poly(
        &[
            (vx - 30.0, vy + 600.0),
            (vx - 12.0, vy - 40.0),
            (vx + 12.0, vy - 40.0),
            (vx + 30.0, vy + 600.0),
        ],
        0x140c30,
        1.0,
    );
    c.glow(
        vx,
        vy - 50.0,
        50.0,
        0xffe9a0,
        0.5 + 0.2 * (time * 3.0).sin(),
    );
    ridge(
        c, cam_x, cam_y, 0.2, 640.0, 160.0, 0.004, 8, 0x2a1c58, 0x180f38, true,
    );
    ridge(
        c, cam_x, cam_y, 0.4, 700.0, 100.0, 0.007, 18, 0x20164a, 0x120c30, false,
    );
}
