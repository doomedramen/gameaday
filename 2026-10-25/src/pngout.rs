//! PNG writing for screenshots.

use std::io::BufWriter;
use std::path::Path;

pub fn write_rgb(path: &Path, w: usize, h: usize, px: &[u32]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    let mut data = Vec::with_capacity(w * h * 3);
    for &p in px.iter().take(w * h) {
        data.push((p >> 16) as u8);
        data.push((p >> 8) as u8);
        data.push(p as u8);
    }
    wr.write_image_data(&data).map_err(|e| e.to_string())
}
