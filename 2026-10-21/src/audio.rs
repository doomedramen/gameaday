//! Dependency-free sound: we synthesise short WAV clips at startup into the
//! system temp dir and play them with macOS `afplay` as detached child
//! processes. Nothing here can crash the game - every failure is swallowed and
//! simply means silence.
//!
//! A tiny voice limiter keeps a burst of the same sound from spawning dozens
//! of processes at once.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

const SAMPLE_RATE: u32 = 22_050;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sound {
    Shoot,
    Hit,
    Explode,
    Orb,
    Hurt,
    WaveStart,
    Pulse,
    Dash,
    BossSpawn,
    Powerup,
    Spit,
}

pub struct Audio {
    files: HashMap<Sound, PathBuf>,
    /// Last time each sound was played, for the per-sound rate limit.
    last: HashMap<Sound, Instant>,
    enabled: bool,
}

/// Minimum gap between two plays of the same sound (seconds).
fn min_gap(s: Sound) -> f32 {
    match s {
        Sound::Shoot => 0.06,
        Sound::Hit => 0.045,
        Sound::Explode => 0.05,
        Sound::Orb => 0.08,
        Sound::Hurt => 0.25,
        Sound::Spit => 0.12,
        _ => 0.0,
    }
}

impl Audio {
    /// Build every clip on disk. Returns a silent Audio if anything fails
    /// (e.g. running on a machine without `afplay`).
    pub fn new(enabled: bool) -> Audio {
        let mut a = Audio {
            files: HashMap::new(),
            last: HashMap::new(),
            enabled: enabled && afplay_available(),
        };
        if !a.enabled {
            return a;
        }
        let dir = std::env::temp_dir().join("nebula_drift_sfx");
        if std::fs::create_dir_all(&dir).is_err() {
            a.enabled = false;
            return a;
        }
        let clips: [(Sound, Vec<f32>); 11] = [
            (Sound::Shoot, shoot()),
            (Sound::Hit, hit()),
            (Sound::Explode, explode()),
            (Sound::Orb, orb()),
            (Sound::Hurt, hurt()),
            (Sound::WaveStart, wave_start()),
            (Sound::Pulse, pulse()),
            (Sound::Dash, dash()),
            (Sound::BossSpawn, boss_spawn()),
            (Sound::Powerup, powerup()),
            (Sound::Spit, spit()),
        ];
        for (s, samples) in clips {
            let path = dir.join(format!("{}.wav", name(s)));
            if std::fs::write(&path, encode_wav(&samples, SAMPLE_RATE)).is_ok() {
                a.files.insert(s, path);
            }
        }
        a
    }

    pub fn play(&mut self, s: Sound) {
        if !self.enabled {
            return;
        }
        let gap = min_gap(s);
        if gap > 0.0
            && let Some(t) = self.last.get(&s)
            && t.elapsed().as_secs_f32() < gap
        {
            return;
        }
        let Some(path) = self.files.get(&s).cloned() else {
            return;
        };
        self.last.insert(s, Instant::now());
        // Detached, output suppressed. If this fails we stay silent.
        let _ = Command::new("afplay")
            .arg("-v")
            .arg("0.45")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn();
    }
}

fn afplay_available() -> bool {
    Command::new("afplay")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn name(s: Sound) -> &'static str {
    match s {
        Sound::Shoot => "shoot",
        Sound::Hit => "hit",
        Sound::Explode => "explode",
        Sound::Orb => "orb",
        Sound::Hurt => "hurt",
        Sound::WaveStart => "wave",
        Sound::Pulse => "pulse",
        Sound::Dash => "dash",
        Sound::BossSpawn => "boss",
        Sound::Powerup => "powerup",
        Sound::Spit => "spit",
    }
}

// ---------------------------------------------------------------------------
// Synth voices
// ---------------------------------------------------------------------------

fn env(i: usize, n: usize, attack: f32, release: f32) -> f32 {
    let t = i as f32 / n as f32;
    let a = (t / attack.max(1e-4)).min(1.0);
    let r = ((1.0 - t) / release.max(1e-4)).min(1.0);
    (a * r).clamp(0.0, 1.0)
}

fn sine(freq: f32, t: f32) -> f32 {
    (t * freq * std::f32::consts::TAU).sin()
}

fn square(freq: f32, t: f32) -> f32 {
    if (t * freq).fract() < 0.5 { 1.0 } else { -1.0 }
}

fn saw(freq: f32, t: f32) -> f32 {
    (t * freq).fract() * 2.0 - 1.0
}

/// Frequency sweep sine (chirp).
fn chirp(f0: f32, f1: f32, dur: f32, i: usize, n: usize) -> f32 {
    let t = i as f32 / SAMPLE_RATE as f32;
    let p = t / dur;
    let f = f0 + (f1 - f0) * p;
    let phase = t * f * std::f32::consts::TAU * 0.5 * (1.0 + p * 0.5);
    let _ = n;
    phase.sin()
}

fn n_samples(dur: f32) -> usize {
    (dur * SAMPLE_RATE as f32) as usize
}

fn shoot() -> Vec<f32> {
    let n = n_samples(0.09);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let s = chirp(1200.0, 380.0, 0.09, i, n);
            let sq = square(900.0 - t * 3000.0, t) * 0.25;
            (s * 0.7 + sq) * env(i, n, 0.01, 1.0) * 0.5
        })
        .collect()
}

fn hit() -> Vec<f32> {
    let n = n_samples(0.06);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let noise = (i as u32).wrapping_mul(2654435761) as f32 / u32::MAX as f32 * 2.0 - 1.0;
            (noise * 0.5 + sine(700.0, t) * 0.5) * env(i, n, 0.005, 1.0) * 0.45
        })
        .collect()
}

fn explode() -> Vec<f32> {
    let n = n_samples(0.34);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let p = i as f32 / n as f32;
            let noise = (i as u32).wrapping_mul(40503) as f32 / u32::MAX as f32 * 2.0 - 1.0;
            let low = sine(140.0 * (1.0 - p * 0.6), t);
            (noise * (1.0 - p) * 0.7 + low * 0.6) * env(i, n, 0.005, 1.0) * 0.6
        })
        .collect()
}

fn orb() -> Vec<f32> {
    let n = n_samples(0.14);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let s = sine(880.0, t) * 0.5 + sine(1320.0, t) * 0.3;
            let p = i as f32 / n as f32;
            s * env(i, n, 0.02, 1.0) * 0.4 * (1.0 + p * 0.3)
        })
        .collect()
}
fn hurt() -> Vec<f32> {
    let n = n_samples(0.30);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let p = i as f32 / n as f32;
            let s = saw(220.0 * (1.0 - p * 0.5), t);
            let sq = square(110.0, t) * 0.4;
            (s * 0.6 + sq) * env(i, n, 0.01, 1.0) * 0.55
        })
        .collect()
}

fn wave_start() -> Vec<f32> {
    let n = n_samples(0.42);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let p = i as f32 / n as f32;
            // Two-tone rising arpeggio.
            let f = if p < 0.5 { 440.0 } else { 660.0 };
            let s = sine(f, t) * 0.5 + sine(f * 2.0, t) * 0.2;
            s * env(i, n, 0.02, 1.0) * 0.42
        })
        .collect()
}

fn pulse() -> Vec<f32> {
    let n = n_samples(0.5);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let s = chirp(160.0, 1800.0, 0.5, i, n);
            let boom = sine(70.0 * (1.0 - t), t) * 0.5;
            (s * 0.6 + boom) * env(i, n, 0.01, 1.0) * 0.6
        })
        .collect()
}

fn dash() -> Vec<f32> {
    let n = n_samples(0.18);
    (0..n)
        .map(|i| {
            let noise = (i as u32).wrapping_mul(2246822519) as f32 / u32::MAX as f32 * 2.0 - 1.0;
            (noise * 0.4 + chirp(1800.0, 500.0, 0.18, i, n) * 0.4) * env(i, n, 0.005, 1.0) * 0.4
        })
        .collect()
}

fn boss_spawn() -> Vec<f32> {
    let n = n_samples(0.9);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let p = i as f32 / n as f32;
            let s = saw(60.0 + p * 30.0, t) * 0.5 + square(55.0, t) * 0.3;
            let sub = sine(40.0, t) * 0.5;
            (s + sub) * env(i, n, 0.05, 1.0) * 0.65
        })
        .collect()
}

fn powerup() -> Vec<f32> {
    let n = n_samples(0.35);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let p = i as f32 / n as f32;
            let f = 520.0 * (1.0 + p * 1.5);
            let s = sine(f, t) * 0.5 + sine(f * 1.5, t) * 0.25;
            s * env(i, n, 0.02, 1.0) * 0.4
        })
        .collect()
}

fn spit() -> Vec<f32> {
    let n = n_samples(0.22);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let p = i as f32 / n as f32;
            let s = saw(320.0 * (1.0 - p * 0.7), t);
            let sq = square(160.0, t) * 0.4;
            (s * 0.5 + sq) * env(i, n, 0.01, 1.0) * 0.42
        })
        .collect()
}

/// A short tone used by the headless selftest to prove the WAV encoder runs.
pub fn test_clip() -> Vec<f32> {
    shoot()
}

/// Minimal 16-bit mono PCM WAV encoder.
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_well_formed() {
        let samples = vec![0.0f32; 100];
        let wav = encode_wav(&samples, 22_050);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // 44-byte header + 100 samples * 2 bytes.
        assert_eq!(wav.len(), 44 + 200);
        let data_len = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]);
        assert_eq!(data_len, 200);
    }

    #[test]
    fn samples_are_clamped_to_i16_range() {
        let wav = encode_wav(&[5.0, -5.0], 22_050);
        let s0 = i16::from_le_bytes([wav[44], wav[45]]);
        let s1 = i16::from_le_bytes([wav[46], wav[47]]);
        assert_eq!(s0, 32767);
        assert_eq!(s1, -32767);
    }

    #[test]
    fn every_synth_voice_produces_non_silent_audio() {
        let voices: [(&str, Vec<f32>); 11] = [
            ("shoot", shoot()),
            ("hit", hit()),
            ("explode", explode()),
            ("orb", orb()),
            ("hurt", hurt()),
            ("wave", wave_start()),
            ("pulse", pulse()),
            ("dash", dash()),
            ("boss", boss_spawn()),
            ("powerup", powerup()),
            ("spit", spit()),
        ];
        for (name, samples) in voices {
            assert!(!samples.is_empty(), "{name} produced no samples");
            let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.01, "{name} is silent");
            assert!(peak <= 1.5, "{name} is far too loud (peak {peak})");
            assert!(
                samples.iter().all(|s| s.is_finite()),
                "{name} contains NaN/inf"
            );
        }
    }

    #[test]
    fn audio_starts_disabled_when_asked() {
        let a = Audio::new(false);
        assert!(!a.enabled, "audio should honour the disable flag");
    }
}
