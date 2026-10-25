//! Mixer: sampled effects (vendored Kenney impacts and UI clicks, RMS-
//! normalised at load), synthesised engines, skid and wind beds, and an
//! adaptive music sequencer. Every effect belongs to a loudness class
//! (quiet / normal / heavy) so the mix is set in one place.

use crate::assets;
use crate::race::Sfx;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ui {
    Move,
    Select,
    Back,
    Error,
    Buy,
    Reveal,
    Fanfare,
    Defeat,
    Medal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound {
    Race(Sfx, f32, f32),
    Ui(Ui),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Engine {
    pub rpm: f32,
    pub load: f32,
    pub vol: f32,
    pub pan: f32,
    pub boost: bool,
    pub bike: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Music {
    /// 0 menu, 1..=5 venues (Greenvale..Grand Prix), 6 results.
    pub theme: u8,
    pub intensity: f32,
    pub final_lap: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mix {
    pub engines: Vec<Engine>,
    pub skid: f32,
    pub wind: f32,
    pub music: Music,
    pub music_vol: f32,
    pub sfx_vol: f32,
}

const QUIET: f32 = 0.28;
const NORMAL: f32 = 0.5;
const HEAVY: f32 = 0.85;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Osc {
    Sine,
    Tri,
    Saw,
    Square,
    Noise,
}

#[derive(Clone, Debug)]
struct Voice {
    osc: Osc,
    f0: f32,
    f1: f32,
    dur: f32,
    t: f32,
    delay: f32,
    attack: f32,
    vol: f32,
    pan: f32,
    lp: f32,
    lps: f32,
    hp: bool,
    hps: f32,
    phase: f32,
    detune: f32,
    phase2: f32,
    noise: u32,
    music: bool,
    curve: f32,
}

fn voice(osc: Osc, f0: f32, f1: f32, dur: f32, vol: f32) -> Voice {
    Voice {
        osc,
        f0,
        f1,
        dur,
        t: 0.0,
        delay: 0.0,
        attack: 0.003,
        vol,
        pan: 0.0,
        lp: 1.0,
        lps: 0.0,
        hp: false,
        hps: 0.0,
        phase: 0.0,
        detune: 0.0,
        phase2: 0.0,
        noise: 0x9E37_79B9,
        music: false,
        curve: 1.5,
    }
}

impl Voice {
    fn at(mut self, d: f32) -> Self {
        self.delay = d;
        self
    }
    fn lp(mut self, k: f32) -> Self {
        self.lp = k;
        self
    }
    fn attack(mut self, a: f32) -> Self {
        self.attack = a;
        self
    }
    fn pan(mut self, p: f32) -> Self {
        self.pan = p;
        self
    }
    fn hp(mut self) -> Self {
        self.hp = true;
        self
    }
    fn detune(mut self, d: f32) -> Self {
        self.detune = d;
        self
    }
    fn music(mut self) -> Self {
        self.music = true;
        self
    }
    fn curve(mut self, c: f32) -> Self {
        self.curve = c;
        self
    }

    fn osc_at(osc: Osc, ph: f32, noise: &mut u32) -> f32 {
        match osc {
            Osc::Sine => (ph * std::f32::consts::TAU).sin(),
            Osc::Tri => 1.0 - 4.0 * (ph - 0.5).abs(),
            Osc::Saw => 2.0 * ph - 1.0,
            Osc::Square => {
                if ph < 0.5 {
                    0.7
                } else {
                    -0.7
                }
            }
            Osc::Noise => {
                *noise ^= *noise << 13;
                *noise ^= *noise >> 17;
                *noise ^= *noise << 5;
                (*noise as f32 / u32::MAX as f32) * 2.0 - 1.0
            }
        }
    }

    /// Returns (sample, done).
    fn next(&mut self, dt: f32) -> (f32, bool) {
        if self.delay > 0.0 {
            self.delay -= dt;
            return (0.0, false);
        }
        self.t += dt;
        if self.t >= self.dur {
            return (0.0, true);
        }
        let u = self.t / self.dur;
        let f = self.f0 * (self.f1 / self.f0).powf(u);
        self.phase = (self.phase + f * dt).fract();
        let mut x = Self::osc_at(self.osc, self.phase, &mut self.noise);
        if self.detune != 0.0 {
            self.phase2 = (self.phase2 + f * (1.0 + self.detune) * dt).fract();
            x = 0.5 * (x + Self::osc_at(self.osc, self.phase2, &mut self.noise));
        }
        self.lps += (x - self.lps) * self.lp;
        let mut y = self.lps;
        if self.hp {
            self.hps += (y - self.hps) * 0.15;
            y -= self.hps;
        }
        let env = (self.t / self.attack).min(1.0) * (1.0 - u).powf(self.curve);
        (y * env * self.vol, false)
    }
}

#[derive(Clone, Debug)]
struct SampleVoice {
    data: &'static [f32],
    pos: f32,
    rate: f32,
    vol: f32,
    pan: f32,
}

#[derive(Clone, Debug, Default)]
struct EngineOsc {
    phase: f32,
    sub: f32,
    lp: f32,
    f: f32,
    vol: f32,
    pan: f32,
    noise: u32,
}

pub struct Synth {
    rate: f32,
    voices: Vec<Voice>,
    samples: Vec<SampleVoice>,
    engines: Vec<EngineOsc>,
    mix: Mix,
    skid_lp: f32,
    skid_v: f32,
    wind_v: f32,
    wind_lp: f32,
    noise: u32,
    // sequencer
    step_t: f32,
    step: u32,
    music_gain: f32,
    pub sfx_gain: f32,
    lead_seed: u32,
}

/// Note to frequency (MIDI numbering).
fn hz(n: i32) -> f32 {
    440.0 * 2f32.powf((n as f32 - 69.0) / 12.0)
}

struct Theme {
    root: i32,
    scale: [i32; 7],
    tempo: f32,
    prog: [usize; 4],
    swing: f32,
    drums: bool,
}

const MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];
const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];
const MIXO: [i32; 7] = [0, 2, 4, 5, 7, 9, 10];

fn theme(id: u8) -> Theme {
    match id {
        0 => Theme {
            root: 50,
            scale: MAJOR,
            tempo: 96.0,
            prog: [0, 5, 3, 4],
            swing: 0.0,
            drums: false,
        },
        1 => Theme {
            root: 52,
            scale: MAJOR,
            tempo: 132.0,
            prog: [0, 4, 5, 3],
            swing: 0.0,
            drums: true,
        },
        2 => Theme {
            root: 50,
            scale: DORIAN,
            tempo: 124.0,
            prog: [0, 3, 0, 6],
            swing: 0.08,
            drums: true,
        },
        3 => Theme {
            root: 45,
            scale: MIXO,
            tempo: 138.0,
            prog: [0, 6, 3, 0],
            swing: 0.12,
            drums: true,
        },
        4 => Theme {
            root: 49,
            scale: MINOR,
            tempo: 128.0,
            prog: [0, 5, 2, 6],
            swing: 0.0,
            drums: true,
        },
        5 => Theme {
            root: 48,
            scale: MINOR,
            tempo: 142.0,
            prog: [0, 5, 3, 4],
            swing: 0.0,
            drums: true,
        },
        _ => Theme {
            root: 53,
            scale: MAJOR,
            tempo: 110.0,
            prog: [0, 3, 4, 0],
            swing: 0.0,
            drums: false,
        },
    }
}

impl Synth {
    pub fn new(rate: f32) -> Synth {
        Synth {
            rate,
            voices: Vec::new(),
            samples: Vec::new(),
            engines: Vec::new(),
            mix: Mix {
                music_vol: 0.8,
                sfx_vol: 1.0,
                ..Mix::default()
            },
            skid_lp: 0.0,
            skid_v: 0.0,
            wind_v: 0.0,
            wind_lp: 0.0,
            noise: 0x1234_5678,
            step_t: 0.0,
            step: 0,
            music_gain: 0.0,
            sfx_gain: 1.0,
            lead_seed: 7,
        }
    }

    pub fn set_mix(&mut self, m: Mix) {
        if m.music.theme != self.mix.music.theme {
            self.step = 0;
            self.step_t = 0.0;
            self.music_gain = 0.0;
            self.lead_seed = 7 + m.music.theme as u32 * 31;
        }
        self.mix = m;
    }

    fn sample(&mut self, name: &str, vol: f32, pan: f32, rate: f32) {
        let a = assets::get();
        if let Some(d) = a.sounds.get(name) {
            let data: &'static [f32] = d.as_slice();
            self.samples.push(SampleVoice {
                data,
                pos: 0.0,
                rate: rate * a.sound_rate / self.rate,
                vol,
                pan,
            });
            if self.samples.len() > 24 {
                self.samples.remove(0);
            }
        }
    }

    fn push(&mut self, v: Voice) {
        self.voices.push(v);
        if self.voices.len() > 96 {
            // drop the oldest music voices first
            if let Some(i) = self.voices.iter().position(|v| v.music) {
                self.voices.remove(i);
            } else {
                self.voices.remove(0);
            }
        }
    }

    pub fn play(&mut self, s: Sound) {
        match s {
            Sound::Ui(u) => self.ui(u),
            Sound::Race(sfx, vol, pan) => self.race(sfx, vol, pan),
        }
    }

    fn ui(&mut self, u: Ui) {
        match u {
            Ui::Move => self.sample("tick", QUIET, 0.0, 1.0),
            Ui::Select => self.sample("select", NORMAL, 0.0, 1.0),
            Ui::Back => self.sample("back", QUIET, 0.0, 1.0),
            Ui::Error => self.sample("error", NORMAL, 0.0, 1.0),
            Ui::Buy => {
                self.sample("buy", NORMAL, 0.0, 1.0);
                self.push(voice(Osc::Square, hz(84), hz(84), 0.12, 0.12).at(0.05));
                self.push(voice(Osc::Square, hz(91), hz(91), 0.2, 0.12).at(0.11));
            }
            Ui::Reveal => self.sample("reveal", NORMAL, 0.0, 1.0),
            Ui::Fanfare => {
                self.sample("fanfare", NORMAL, 0.0, 1.0);
                for (k, n) in [72, 76, 79, 84, 79, 84, 88].iter().enumerate() {
                    let d = [0.0, 0.12, 0.24, 0.36, 0.6, 0.72, 0.84][k];
                    let len = if k == 6 { 0.9 } else { 0.2 };
                    self.push(voice(Osc::Square, hz(*n), hz(*n), len, 0.16).at(d).lp(0.4));
                    self.push(voice(Osc::Tri, hz(*n - 12), hz(*n - 12), len, 0.16).at(d));
                }
            }
            Ui::Defeat => {
                for (k, n) in [67, 65, 62, 55].iter().enumerate() {
                    self.push(
                        voice(Osc::Tri, hz(*n), hz(*n), 0.4, 0.2)
                            .at(k as f32 * 0.22)
                            .lp(0.3),
                    );
                }
            }
            Ui::Medal => {
                self.sample("sparkle", NORMAL, 0.0, 1.0);
                for (k, n) in [79, 83, 86, 91].iter().enumerate() {
                    self.push(voice(Osc::Sine, hz(*n), hz(*n), 0.35, 0.18).at(k as f32 * 0.08));
                }
            }
        }
    }

    fn race(&mut self, sfx: Sfx, vol: f32, pan: f32) {
        match sfx {
            Sfx::CrashHeavy => {
                self.sample(
                    if self.noise & 1 == 0 {
                        "crash_heavy"
                    } else {
                        "crash_heavy2"
                    },
                    HEAVY * vol,
                    pan,
                    1.0,
                );
                self.push(voice(Osc::Sine, 90.0, 40.0, 0.35, 0.5 * vol).pan(pan));
                self.noise = self.noise.wrapping_mul(1664525).wrapping_add(1013904223);
            }
            Sfx::CrashMedium => self.sample("crash_medium", NORMAL * vol, pan, 1.1),
            Sfx::Bump => self.sample(
                if self.noise & 2 == 0 { "bump" } else { "bump2" },
                NORMAL * vol,
                pan,
                1.0,
            ),
            Sfx::Ram => {
                self.sample("ram", HEAVY * vol, pan, 0.9);
                self.sample("crash_medium", NORMAL * vol, pan, 0.8);
            }
            Sfx::Cone => self.sample("cone", QUIET * vol, pan, 1.3),
            Sfx::Pickup => {
                self.sample("pickup", NORMAL, 0.0, 1.0);
                self.push(voice(Osc::Sine, 900.0, 1800.0, 0.18, 0.15));
            }
            Sfx::Repair => {
                self.sample("pickup", NORMAL, 0.0, 0.8);
                self.push(voice(Osc::Tri, 500.0, 900.0, 0.25, 0.18));
            }
            Sfx::Turbo(t) => {
                let v = [0.0, 0.25, 0.32, 0.4][t as usize] * vol;
                self.push(
                    voice(Osc::Noise, 1.0, 1.0, 0.5 + 0.15 * t as f32, v)
                        .lp(0.25)
                        .attack(0.01)
                        .pan(pan),
                );
                self.push(
                    voice(Osc::Saw, 110.0, 330.0 + 80.0 * t as f32, 0.35, v * 0.6)
                        .lp(0.2)
                        .pan(pan),
                );
            }
            Sfx::TierUp(t) => {
                let n = [0, 76, 80, 83][t as usize];
                self.push(voice(Osc::Square, hz(n), hz(n + 12), 0.12, 0.12).lp(0.5));
            }
            Sfx::NitroStart => {
                self.push(voice(Osc::Noise, 1.0, 1.0, 0.6, 0.25).lp(0.35).attack(0.05))
            }
            Sfx::Pad => {
                self.push(voice(Osc::Saw, 220.0, 660.0, 0.3, 0.18).lp(0.3));
                self.push(voice(Osc::Noise, 1.0, 1.0, 0.4, 0.15).lp(0.3));
            }
            Sfx::Shift => self.push(voice(Osc::Noise, 1.0, 1.0, 0.06, 0.08).lp(0.5)),
            Sfx::Spin => {
                self.push(
                    voice(Osc::Noise, 1.0, 1.0, 0.7, 0.3 * vol)
                        .lp(0.6)
                        .hp()
                        .pan(pan),
                );
                self.push(
                    voice(Osc::Saw, 700.0, 300.0, 0.6, 0.06 * vol)
                        .lp(0.3)
                        .pan(pan),
                );
            }
            Sfx::CountBeep => self.push(
                voice(Osc::Square, hz(69), hz(69), 0.22, 0.3)
                    .lp(0.5)
                    .curve(0.4),
            ),
            Sfx::GoBeep => self.push(
                voice(Osc::Square, hz(81), hz(81), 0.5, 0.35)
                    .lp(0.5)
                    .curve(0.6),
            ),
            Sfx::Lap => {
                self.sample("bell", NORMAL, 0.0, 1.4);
            }
            Sfx::FinalLap => {
                self.sample("bell", HEAVY * 0.8, 0.0, 1.0);
                self.push(
                    voice(Osc::Square, hz(76), hz(76), 0.15, 0.15)
                        .lp(0.4)
                        .at(0.1),
                );
                self.push(
                    voice(Osc::Square, hz(83), hz(83), 0.35, 0.15)
                        .lp(0.4)
                        .at(0.25),
                );
            }
            Sfx::Finish => {
                self.sample("bell", HEAVY, 0.0, 1.0);
                self.sample("bell", NORMAL, 0.0, 1.5);
            }
            Sfx::Style => self.push(voice(Osc::Sine, 1300.0, 1700.0, 0.08, 0.06)),
            Sfx::Slingshot => {
                self.push(voice(Osc::Saw, 200.0, 900.0, 0.4, 0.2).lp(0.3));
                self.push(voice(Osc::Noise, 1.0, 1.0, 0.5, 0.2).lp(0.2).attack(0.02));
            }
            Sfx::Overtake => self.push(voice(Osc::Tri, hz(79), hz(84), 0.14, 0.14)),
            Sfx::BullyWarn => {
                for k in 0..2 {
                    self.push(
                        voice(Osc::Square, 880.0, 880.0, 0.1, 0.18 * vol.max(0.6))
                            .at(k as f32 * 0.15)
                            .lp(0.6)
                            .pan(pan),
                    );
                }
            }
            Sfx::Stall => self.push(voice(Osc::Saw, 120.0, 40.0, 0.6, 0.3).lp(0.2)),
            Sfx::PerfectStart => {
                self.push(voice(Osc::Saw, 150.0, 600.0, 0.5, 0.25).lp(0.3));
                self.push(voice(Osc::Square, hz(84), hz(88), 0.2, 0.12).at(0.05));
            }
        }
    }

    // ---- music --------------------------------------------------------------

    fn music_step(&mut self) {
        let m = self.mix.music;
        let th = theme(m.theme);
        let st = self.step % 64;
        let bar = (st / 16) as usize;
        let beat = st % 16;
        let lift = if m.final_lap { 2 } else { 0 };
        let deg = th.prog[bar];
        let root = th.root + lift;
        let note = |d: usize, oct: i32| root + th.scale[d % 7] + 12 * (d as i32 / 7) + 12 * oct;
        let int = m.intensity.clamp(0.0, 1.0);
        let g = 1.0;
        // pad: chord at bar start
        if beat == 0 {
            for (k, d) in [deg, deg + 2, deg + 4].iter().enumerate() {
                let n = note(*d, 0);
                let len = 60.0 / th.tempo * 4.0;
                self.push(
                    voice(Osc::Saw, hz(n), hz(n), len * 1.05, 0.045 * g)
                        .detune(0.006)
                        .lp(0.05)
                        .attack(0.3)
                        .curve(0.6)
                        .pan(k as f32 * 0.4 - 0.4)
                        .music(),
                );
            }
        }
        // bass
        if beat % 2 == 0 && (th.drums || beat % 4 == 0) {
            let n = note(deg, -2) + if beat % 8 == 6 { 12 } else { 0 };
            self.push(
                voice(Osc::Saw, hz(n), hz(n), 0.2, 0.14 * g)
                    .lp(0.08)
                    .music(),
            );
        }
        if th.drums {
            if beat % 4 == 0 && int > 0.1 {
                self.push(
                    voice(Osc::Sine, 130.0, 42.0, 0.22, 0.42 * g)
                        .curve(2.0)
                        .music(),
                );
            }
            if (beat == 4 || beat == 12) && int > 0.3 {
                self.push(
                    voice(Osc::Noise, 1.0, 1.0, 0.16, 0.14 * g)
                        .lp(0.5)
                        .hp()
                        .music(),
                );
                self.push(voice(Osc::Tri, 190.0, 150.0, 0.1, 0.12 * g).music());
            }
            let hat = if int > 0.65 { true } else { beat % 2 == 0 };
            if hat && int > 0.2 {
                let acc = if beat % 4 == 2 { 1.0 } else { 0.6 };
                self.push(
                    voice(Osc::Noise, 1.0, 1.0, 0.04, 0.05 * acc * g)
                        .hp()
                        .music(),
                );
            }
            if m.final_lap && beat == 14 && bar == 3 {
                for k in 0..4 {
                    self.push(
                        voice(Osc::Noise, 1.0, 1.0, 0.08, 0.1 * g)
                            .lp(0.5)
                            .hp()
                            .at(k as f32 * 0.04)
                            .music(),
                    );
                }
            }
        }
        // arpeggio
        if int > 0.4 || !th.drums {
            let chord = [deg, deg + 2, deg + 4, deg + 7];
            let d = chord[(beat as usize) % 4];
            let n = note(d, if th.drums { 1 } else { 0 });
            let v = if th.drums { 0.045 } else { 0.05 };
            if th.drums || beat % 2 == 0 {
                self.push(
                    voice(Osc::Square, hz(n), hz(n), 0.11, v * g)
                        .lp(0.25)
                        .pan(if beat % 2 == 0 { -0.3 } else { 0.3 })
                        .music(),
                );
            }
        }
        // lead: a seeded phrase that repeats every two bars
        if (int > 0.72 || m.final_lap) && th.drums {
            let k = st % 32;
            let mut h = self
                .lead_seed
                .wrapping_mul(2654435761)
                .wrapping_add(k.wrapping_mul(40503));
            h ^= h >> 13;
            let play = k % 2 == 0 && h % 5 != 0;
            if play {
                let d = deg + [0usize, 2, 4, 5, 7, 4, 2, 1][(h as usize >> 3) % 8];
                let n = note(d, 1);
                self.push(
                    voice(Osc::Square, hz(n), hz(n) * 0.998, 0.22, 0.06 * g)
                        .detune(0.004)
                        .lp(0.18)
                        .pan(0.1)
                        .music(),
                );
            }
        }
    }

    /// Fill an interleaved buffer.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let dt = 1.0 / self.rate;
        let frames = out.len() / channels.max(1);
        // engine voices follow the mix
        while self.engines.len() < self.mix.engines.len() {
            self.engines.push(EngineOsc {
                noise: 0x55AA_1234 + self.engines.len() as u32,
                ..EngineOsc::default()
            });
        }
        let th = theme(self.mix.music.theme);
        let tempo = th.tempo * if self.mix.music.final_lap { 1.06 } else { 1.0 };
        let step_len = 60.0 / tempo / 4.0;
        for f in 0..frames {
            // sequencer
            self.step_t += dt;
            let swing = if self.step % 2 == 1 {
                th.swing * step_len
            } else {
                0.0
            };
            if self.step_t >= step_len + swing {
                self.step_t -= step_len + swing;
                if self.mix.music_vol > 0.0 {
                    self.music_step();
                }
                self.step = self.step.wrapping_add(1);
            }
            self.music_gain += (1.0 - self.music_gain) * 0.00005;
            let (mut ml, mut mr) = (0.0f32, 0.0f32);
            let (mut sl, mut sr) = (0.0f32, 0.0f32);
            let mut i = 0;
            while i < self.voices.len() {
                let (x, done) = self.voices[i].next(dt);
                if done {
                    self.voices.swap_remove(i);
                    continue;
                }
                let p = self.voices[i].pan;
                let (l, r) = ((1.0 - p).min(1.0), (1.0 + p).min(1.0));
                if self.voices[i].music {
                    ml += x * l;
                    mr += x * r;
                } else {
                    sl += x * l;
                    sr += x * r;
                }
                i += 1;
            }
            let mut i = 0;
            while i < self.samples.len() {
                let s = &mut self.samples[i];
                let idx = s.pos as usize;
                if idx + 1 >= s.data.len() {
                    self.samples.swap_remove(i);
                    continue;
                }
                let fr = s.pos - idx as f32;
                let x = (s.data[idx] + (s.data[idx + 1] - s.data[idx]) * fr) * s.vol;
                s.pos += s.rate;
                sl += x * (1.0 - s.pan).min(1.0);
                sr += x * (1.0 + s.pan).min(1.0);
                i += 1;
            }
            // engines
            for (k, e) in self.engines.iter_mut().enumerate() {
                let Some(m) = self.mix.engines.get(k) else {
                    e.vol *= 0.999;
                    continue;
                };
                let base = if m.bike { 70.0 } else { 42.0 };
                let target_f = base + m.rpm * if m.bike { 200.0 } else { 150.0 };
                e.f += (target_f - e.f) * 0.002;
                e.vol += (m.vol - e.vol) * 0.002;
                e.pan += (m.pan - e.pan) * 0.002;
                e.phase = (e.phase + e.f * dt).fract();
                e.sub = (e.sub + e.f * 0.5 * dt).fract();
                let saw = 2.0 * e.phase - 1.0;
                let sq = if e.sub < 0.5 { 0.6 } else { -0.6 };
                let mut x = saw * 0.6 + sq * 0.5;
                if m.boost {
                    e.noise ^= e.noise << 13;
                    e.noise ^= e.noise >> 17;
                    e.noise ^= e.noise << 5;
                    x += (e.noise as f32 / u32::MAX as f32 - 0.5) * 0.8;
                }
                let cut = 0.04 + 0.12 * m.load.clamp(0.0, 1.0);
                e.lp += (x - e.lp) * cut;
                let y = e.lp * e.vol;
                sl += y * (1.0 - e.pan).min(1.0);
                sr += y * (1.0 + e.pan).min(1.0);
            }
            // skid and wind beds
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            let n = self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0;
            self.skid_v += (self.mix.skid - self.skid_v) * 0.001;
            self.wind_v += (self.mix.wind - self.wind_v) * 0.0005;
            let hi = n - self.skid_lp;
            self.skid_lp += (n - self.skid_lp) * 0.3;
            self.wind_lp += (n - self.wind_lp) * 0.02;
            let bed = hi * self.skid_v * 0.12 * (1.0 + 0.3 * (self.step_t * 300.0).sin())
                + self.wind_lp * self.wind_v * 0.5;
            sl += bed;
            sr += bed;

            let mg = self.mix.music_vol * self.music_gain * 0.8;
            let sg = self.mix.sfx_vol * self.sfx_gain;
            let l = soft(ml * mg + sl * sg);
            let r = soft(mr * mg + sr * sg);
            let base = f * channels;
            if channels == 1 {
                out[base] = (l + r) * 0.5;
            } else {
                out[base] = l;
                out[base + 1] = r;
                for c in 2..channels {
                    out[base + c] = 0.0;
                }
            }
        }
    }
}

fn soft(x: f32) -> f32 {
    if x.abs() < 0.6 {
        x
    } else {
        x.signum() * (0.6 + (1.0 - (-(x.abs() - 0.6) * 2.5).exp()) * 0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_finite_bounded_audio() {
        let mut s = Synth::new(48000.0);
        s.set_mix(Mix {
            engines: vec![Engine {
                rpm: 0.7,
                load: 1.0,
                vol: 0.3,
                pan: 0.0,
                boost: true,
                bike: false,
            }],
            skid: 1.0,
            wind: 0.5,
            music: Music {
                theme: 5,
                intensity: 1.0,
                final_lap: true,
            },
            music_vol: 1.0,
            sfx_vol: 1.0,
        });
        for sfx in [
            Sfx::CrashHeavy,
            Sfx::Ram,
            Sfx::Turbo(3),
            Sfx::Finish,
            Sfx::Slingshot,
        ] {
            s.play(Sound::Race(sfx, 1.0, 0.3));
        }
        s.play(Sound::Ui(Ui::Fanfare));
        let mut buf = vec![0.0f32; 48000 * 2];
        s.render(&mut buf, 2);
        assert!(buf.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        let rms = (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt();
        assert!(rms > 0.01, "silent: {rms}");
    }
}
