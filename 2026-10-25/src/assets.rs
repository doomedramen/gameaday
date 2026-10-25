//! Vendored third-party assets (Kenney, CC0): sprites decoded from PNG, sound
//! effects from 16-bit mono WAV. Embedded in the binary; decoded once.

use crate::canvas::Sprite;
use std::collections::HashMap;
use std::sync::OnceLock;

macro_rules! sprites {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../assets/sprites/", $name, ".png")) as &[u8])),*]
    };
}

static SPRITES: &[(&str, &[u8])] = sprites![
    "arrow_white",
    "arrow_yellow",
    "barrel_blue",
    "barrel_red",
    "barrier_red_race",
    "barrier_white_race",
    "car_black_1",
    "car_black_2",
    "car_black_3",
    "car_black_4",
    "car_black_5",
    "car_blue_1",
    "car_blue_2",
    "car_blue_3",
    "car_blue_4",
    "car_blue_5",
    "car_green_1",
    "car_green_2",
    "car_green_3",
    "car_green_4",
    "car_green_5",
    "car_red_1",
    "car_red_2",
    "car_red_3",
    "car_red_4",
    "car_red_5",
    "car_yellow_1",
    "car_yellow_2",
    "car_yellow_3",
    "car_yellow_4",
    "car_yellow_5",
    "character_black_blue",
    "character_black_green",
    "character_black_red",
    "character_black_white",
    "character_blonde_blue",
    "character_blonde_green",
    "character_blonde_red",
    "character_blonde_white",
    "character_brown_blue",
    "character_brown_green",
    "character_brown_red",
    "character_brown_white",
    "cone_down",
    "cone_straight",
    "land_dirt05",
    "land_dirt12",
    "land_grass11",
    "land_sand12",
    "light_white",
    "light_yellow",
    "lights",
    "motorcycle_black",
    "motorcycle_blue",
    "motorcycle_green",
    "motorcycle_red",
    "motorcycle_yellow",
    "oil",
    "racer_black",
    "racer_blue",
    "racer_green",
    "racer_red",
    "racer_yellow",
    "rock1",
    "rock2",
    "rock3",
    "skidmark_short_1",
    "tent_blue",
    "tent_blue_large",
    "tent_red",
    "tent_red_large",
    "tires_red",
    "tires_red_alt",
    "tires_white",
    "tires_white_alt",
    "tree_large",
    "tree_small",
    "tribune_empty",
    "tribune_full",
    "tribune_overhang_red",
    "tribune_overhang_striped",
];

macro_rules! sounds {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../assets/sfx/", $name, ".wav")) as &[u8])),*]
    };
}

static SOUNDS: &[(&str, &[u8])] = sounds![
    "back",
    "barrel",
    "bell",
    "bump",
    "bump2",
    "buy",
    "click",
    "cone",
    "confirm",
    "crash_heavy",
    "crash_heavy2",
    "crash_medium",
    "error",
    "fanfare",
    "pickup",
    "ram",
    "reveal",
    "rock",
    "select",
    "sparkle",
    "switch",
    "tick",
];

pub struct Assets {
    /// Full-resolution sprites (UI, big portraits).
    pub full: HashMap<&'static str, Sprite>,
    /// Half-resolution sprites: the world scale.
    pub half: HashMap<&'static str, Sprite>,
    pub sounds: HashMap<&'static str, Vec<f32>>,
    pub sound_rate: f32,
}

static ASSETS: OnceLock<Assets> = OnceLock::new();

pub fn get() -> &'static Assets {
    ASSETS.get_or_init(load)
}

impl Assets {
    pub fn s(&self, name: &str) -> &Sprite {
        self.half
            .get(name)
            .unwrap_or_else(|| panic!("missing sprite {name}"))
    }
    pub fn big(&self, name: &str) -> &Sprite {
        self.full
            .get(name)
            .unwrap_or_else(|| panic!("missing sprite {name}"))
    }
}

fn load() -> Assets {
    let mut full = HashMap::new();
    let mut half = HashMap::new();
    for (name, bytes) in SPRITES {
        let s = decode_png(bytes).unwrap_or_else(|e| panic!("decode {name}: {e}"));
        half.insert(*name, s.half());
        full.insert(*name, s);
    }
    let mut sounds = HashMap::new();
    for (name, bytes) in SOUNDS {
        sounds.insert(
            *name,
            decode_wav(bytes).unwrap_or_else(|| panic!("decode {name}")),
        );
    }
    Assets {
        full,
        half,
        sounds,
        sound_rate: 22050.0,
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<Sprite, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size().ok_or("size")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width as usize, info.height as usize);
    let mut s = Sprite::new(w, h);
    let ch = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return Err("unexpanded palette".into()),
    };
    for i in 0..w * h {
        let p = &buf[i * ch..i * ch + ch];
        let (r, g, b, a) = match ch {
            4 => (p[0], p[1], p[2], p[3]),
            3 => (p[0], p[1], p[2], 255),
            2 => (p[0], p[0], p[0], p[1]),
            _ => (p[0], p[0], p[0], 255),
        };
        s.px[i] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
    }
    Ok(s)
}

/// Parse a 16-bit PCM mono WAV and normalise it to a common RMS loudness so
/// the mixer's per-class volumes mean the same thing for every sample.
pub fn decode_wav(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        if id == b"data" {
            let end = (i + 8 + len).min(bytes.len());
            let mut out: Vec<f32> = bytes[i + 8..end]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                .collect();
            // RMS over the loud part (ignore the tail).
            let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            let loud: Vec<f32> = out
                .iter()
                .copied()
                .filter(|x| x.abs() > peak * 0.1)
                .collect();
            let rms = (loud.iter().map(|x| x * x).sum::<f32>() / loud.len().max(1) as f32).sqrt();
            if rms > 1e-4 {
                let k = (0.25 / rms).min(0.95 / peak.max(1e-4));
                for x in out.iter_mut() {
                    *x *= k;
                }
            }
            return Some(out);
        }
        i += 8 + len + (len & 1);
    }
    None
}
