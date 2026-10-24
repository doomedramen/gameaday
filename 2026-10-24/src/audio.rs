//! Real-time synthesiser: one-shot sound effects plus an adaptive
//! procedural score. Each biome has its own key, tempo and feel; layers
//! (pad, bass, arpeggio, drums, lead) fade in with the game's intensity.
//! Pure sample generation - `main.rs` feeds it to the audio device.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Click,
    Select,
    Draft,
    Error,
    Banner,
    Step,
    Jump,
    WallJump,
    AirJump,
    /// 0 = soft, 1 = normal, 2 = hard landing.
    Land(u8),
    HookHit,
    HookMiss,
    Release,
    RopeSnap,
    Brittle,
    Crystal(u32),
    EchoOrb,
    Heart,
    Chest,
    Hurt,
    Stomp,
    Ram,
    Smash,
    Pit,
    Checkpoint,
    SpitterCharge,
    SpitterFire,
    GeyserWarn,
    GeyserBurst,
    StrikeWarn,
    StrikeHit,
    Bounce,
    Crumble,
    Cleared,
    Shield,
    BossHit,
    BossStun,
    BossRoar,
    BossAttack,
    BossDown,
    Dying,
    Revive,
    RewindStart,
    RewindTick,
    RewindEnd,
    Medal,
    Victory,
    Defeat,
    Slam,
}

/// What the score should be doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicState {
    /// 0 = calm, 1 = full intensity.
    pub intensity: f32,
    /// 0..=4 for the biomes, 5 for the Stormheart.
    pub theme: u8,
    pub rewinding: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Wave {
    Sine,
    Tri,
    Square,
    Saw,
    Noise,
}

#[derive(Clone, Debug)]
struct Voice {
    wave: Wave,
    f0: f32,
    f1: f32,
    dur: f32,
    t: f32,
    delay: f32,
    attack: f32,
    curve: f32,
    vol: f32,
    pan: f32,
    lp: f32,
    lp_state: f32,
    phase: f32,
    noise: u32,
    held: f32,
    music: bool,
}

fn v(wave: Wave, f0: f32, f1: f32, dur: f32, vol: f32) -> Voice {
    Voice {
        wave,
        f0,
        f1,
        dur,
        t: 0.0,
        delay: 0.0,
        attack: 0.004,
        curve: 1.6,
        vol,
        pan: 0.0,
        lp: 1.0,
        lp_state: 0.0,
        phase: 0.0,
        noise: 0x1234_5678,
        held: 0.0,
        music: false,
    }
}

impl Voice {
    fn lp(mut self, k: f32) -> Self {
        self.lp = k;
        self
    }
    fn at(mut self, d: f32) -> Self {
        self.delay = d;
        self
    }
    fn attack(mut self, a: f32) -> Self {
        self.attack = a;
        self
    }
    fn curve(mut self, c: f32) -> Self {
        self.curve = c;
        self
    }
    fn pan(mut self, p: f32) -> Self {
        self.pan = p;
        self
    }

    /// Returns (left, right) and whether the voice is still alive.
    fn sample(&mut self, dt: f32) -> (f32, f32, bool) {
        if self.delay > 0.0 {
            self.delay -= dt;
            return (0.0, 0.0, true);
        }
        self.t += dt;
        if self.t >= self.dur {
            return (0.0, 0.0, false);
        }
        let p = self.t / self.dur;
        let freq = self.f0 * (self.f1 / self.f0).powf(p);
        self.phase += freq * dt;
        if self.phase > 1.0 {
            self.phase -= self.phase.floor();
            if self.wave == Wave::Noise {
                // Sample-and-hold noise: frequency controls its colour.
                self.noise ^= self.noise << 13;
                self.noise ^= self.noise >> 17;
                self.noise ^= self.noise << 5;
                self.held = (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0;
            }
        }
        let ph = self.phase;
        let x = match self.wave {
            Wave::Sine => (ph * std::f32::consts::TAU).sin(),
            Wave::Tri => 1.0 - 4.0 * (ph - 0.5).abs(),
            Wave::Square => {
                if ph < 0.5 {
                    0.7
                } else {
                    -0.7
                }
            }
            Wave::Saw => 2.0 * ph - 1.0,
            Wave::Noise => self.held,
        };
        self.lp_state += self.lp * (x - self.lp_state);
        let env = (self.t / self.attack).min(1.0) * (1.0 - p).powf(self.curve);
        let s = self.lp_state * env * self.vol;
        let l = s * (1.0 - self.pan.max(0.0));
        let r = s * (1.0 + self.pan.min(0.0));
        (l, r, true)
    }
}

fn midi(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

fn sfx_voices(s: Sfx) -> Vec<Voice> {
    use Wave::*;
    match s {
        Sfx::Click => vec![v(Square, 1200.0, 900.0, 0.04, 0.06).lp(0.5)],
        Sfx::Select => vec![
            v(Tri, 660.0, 990.0, 0.08, 0.12),
            v(Sine, 1320.0, 1320.0, 0.12, 0.06).at(0.04),
        ],
        Sfx::Draft => vec![
            v(Sine, 660.0, 660.0, 0.2, 0.09),
            v(Sine, 880.0, 880.0, 0.3, 0.09).at(0.07),
            v(Sine, 1320.0, 1320.0, 0.4, 0.05).at(0.14),
        ],
        Sfx::Error => vec![v(Square, 150.0, 140.0, 0.1, 0.08).lp(0.3)],
        Sfx::Banner => vec![
            v(Tri, 220.0, 220.0, 0.6, 0.07).attack(0.05),
            v(Tri, 330.0, 330.0, 0.6, 0.06).attack(0.05),
            v(Sine, 660.0, 660.0, 0.8, 0.03).attack(0.1),
        ],
        Sfx::Step => vec![
            v(Noise, 2500.0, 1200.0, 0.04, 0.04).lp(0.2),
            v(Sine, 120.0, 80.0, 0.04, 0.05),
        ],
        Sfx::Jump => vec![
            v(Sine, 300.0, 620.0, 0.12, 0.1),
            v(Noise, 3000.0, 1000.0, 0.05, 0.03).lp(0.3),
        ],
        Sfx::WallJump => vec![
            v(Sine, 360.0, 780.0, 0.14, 0.11),
            v(Noise, 5000.0, 1500.0, 0.07, 0.07).lp(0.4),
            v(Tri, 540.0, 1000.0, 0.1, 0.05),
        ],
        Sfx::AirJump => vec![
            v(Sine, 520.0, 1040.0, 0.15, 0.1),
            v(Tri, 780.0, 1560.0, 0.12, 0.05).at(0.03),
        ],
        Sfx::Land(k) => {
            let g = [0.05, 0.1, 0.22][k.min(2) as usize];
            vec![
                v(Sine, 140.0, 60.0, 0.1 + 0.04 * k as f32, g),
                v(Noise, 2500.0, 600.0, 0.07, g * 0.7).lp(0.25),
            ]
        }
        Sfx::HookHit => vec![
            v(Noise, 6000.0, 2000.0, 0.05, 0.14).lp(0.6),
            v(Tri, 880.0, 1760.0, 0.09, 0.12),
            v(Square, 220.0, 220.0, 0.1, 0.04).lp(0.3),
        ],
        Sfx::HookMiss => vec![
            v(Noise, 3000.0, 800.0, 0.14, 0.08).lp(0.5),
            v(Sine, 500.0, 300.0, 0.1, 0.04),
        ],
        Sfx::Release => vec![v(Noise, 4000.0, 800.0, 0.1, 0.08).lp(0.5)],
        Sfx::RopeSnap => vec![
            v(Square, 800.0, 200.0, 0.12, 0.08).lp(0.4),
            v(Noise, 5000.0, 1000.0, 0.1, 0.1).lp(0.5),
        ],
        Sfx::Brittle => vec![
            v(Noise, 7000.0, 1500.0, 0.2, 0.2).lp(0.6),
            v(Tri, 1400.0, 500.0, 0.2, 0.08),
            v(Square, 600.0, 120.0, 0.15, 0.06).lp(0.3),
        ],
        Sfx::Crystal(n) => {
            let f = 660.0 * 2f32.powf((n.min(14) as f32 * 2.0) / 12.0);
            vec![
                v(Sine, f, f, 0.2, 0.1),
                v(Tri, f * 2.0, f * 2.0, 0.14, 0.04).at(0.03),
                v(Sine, f * 1.5, f * 1.5, 0.22, 0.04).at(0.07),
            ]
        }
        Sfx::EchoOrb => vec![
            v(Sine, 440.0, 880.0, 0.35, 0.12),
            v(Sine, 660.0, 1320.0, 0.4, 0.08).at(0.06),
            v(Sine, 990.0, 1980.0, 0.5, 0.05).at(0.12),
        ],
        Sfx::Heart => vec![
            v(Sine, 523.0, 523.0, 0.3, 0.12),
            v(Sine, 659.0, 659.0, 0.3, 0.12).at(0.07),
            v(Sine, 784.0, 784.0, 0.35, 0.12).at(0.14),
            v(Sine, 1046.0, 1046.0, 0.5, 0.07).at(0.21),
        ],
        Sfx::Chest => vec![
            v(Noise, 800.0, 5000.0, 0.3, 0.15).lp(0.3).attack(0.05),
            v(Tri, 523.0, 523.0, 0.5, 0.12).at(0.1),
            v(Tri, 659.0, 659.0, 0.5, 0.12).at(0.17),
            v(Tri, 784.0, 784.0, 0.5, 0.12).at(0.24),
            v(Tri, 1046.0, 1046.0, 0.7, 0.1).at(0.31),
        ],
        Sfx::Hurt => vec![
            v(Square, 340.0, 110.0, 0.28, 0.2).lp(0.35),
            v(Noise, 3000.0, 700.0, 0.16, 0.18).lp(0.4),
            v(Sine, 120.0, 50.0, 0.3, 0.25),
        ],
        Sfx::Stomp => vec![
            v(Sine, 220.0, 90.0, 0.14, 0.26),
            v(Noise, 4000.0, 900.0, 0.08, 0.16).lp(0.4),
            v(Tri, 880.0, 1320.0, 0.1, 0.06),
        ],
        Sfx::Ram => vec![
            v(Sine, 120.0, 40.0, 0.35, 0.5),
            v(Noise, 5000.0, 700.0, 0.2, 0.3).lp(0.35),
            v(Saw, 260.0, 90.0, 0.2, 0.1).lp(0.3),
        ],
        Sfx::Smash => vec![
            v(Noise, 9000.0, 1500.0, 0.45, 0.4).lp(0.7),
            v(Sine, 110.0, 38.0, 0.45, 0.5),
            v(Tri, 2200.0, 700.0, 0.3, 0.12).at(0.02),
            v(Tri, 3300.0, 1100.0, 0.28, 0.09).at(0.05),
            v(Square, 1500.0, 300.0, 0.15, 0.06).lp(0.4),
        ],
        Sfx::Pit => vec![
            v(Sine, 800.0, 80.0, 0.7, 0.13).curve(0.6),
            v(Noise, 1200.0, 300.0, 0.6, 0.05).lp(0.2).attack(0.2),
        ],
        Sfx::Checkpoint => vec![
            v(Sine, 523.0, 523.0, 0.35, 0.1),
            v(Sine, 784.0, 784.0, 0.4, 0.1).at(0.08),
            v(Sine, 1046.0, 1046.0, 0.6, 0.08).at(0.16),
        ],
        Sfx::SpitterCharge => vec![
            v(Saw, 180.0, 540.0, 0.8, 0.07).lp(0.12).attack(0.3),
            v(Sine, 360.0, 1080.0, 0.8, 0.05).attack(0.4),
        ],
        Sfx::SpitterFire => vec![
            v(Noise, 4000.0, 900.0, 0.12, 0.2).lp(0.5),
            v(Square, 700.0, 180.0, 0.14, 0.1).lp(0.4),
        ],
        Sfx::GeyserWarn => vec![
            v(Noise, 600.0, 900.0, 0.6, 0.1).lp(0.1).attack(0.3),
            v(Sine, 90.0, 120.0, 0.6, 0.1),
        ],
        Sfx::GeyserBurst => vec![
            v(Noise, 700.0, 4000.0, 0.7, 0.28).lp(0.25).attack(0.04),
            v(Saw, 90.0, 55.0, 0.6, 0.12).lp(0.1),
        ],
        Sfx::StrikeWarn => vec![
            v(Sine, 1500.0, 1500.0, 0.12, 0.06),
            v(Sine, 1500.0, 1500.0, 0.12, 0.06).at(0.3),
            v(Sine, 1500.0, 1500.0, 0.12, 0.06).at(0.6),
        ],
        Sfx::StrikeHit => vec![
            v(Noise, 9000.0, 800.0, 0.5, 0.45).lp(0.8).attack(0.002),
            v(Sine, 90.0, 35.0, 0.55, 0.5),
            v(Saw, 1800.0, 200.0, 0.2, 0.1).lp(0.5),
        ],
        Sfx::Bounce => vec![
            v(Sine, 200.0, 700.0, 0.2, 0.14),
            v(Tri, 400.0, 1400.0, 0.15, 0.06),
        ],
        Sfx::Crumble => vec![
            v(Noise, 2000.0, 500.0, 0.3, 0.14).lp(0.2),
            v(Sine, 90.0, 50.0, 0.25, 0.1),
        ],
        Sfx::Cleared => [523.0, 659.0, 784.0, 1046.0, 1318.0]
            .iter()
            .enumerate()
            .map(|(i, f)| v(Tri, *f, *f, 0.5, 0.11).at(i as f32 * 0.09))
            .chain(std::iter::once(
                v(Saw, 262.0, 262.0, 1.2, 0.05).lp(0.08).attack(0.2),
            ))
            .collect(),
        Sfx::Shield => vec![
            v(Tri, 300.0, 900.0, 0.2, 0.14),
            v(Sine, 1800.0, 1800.0, 0.35, 0.07).at(0.05),
            v(Noise, 6000.0, 3000.0, 0.12, 0.06).lp(0.5),
        ],
        Sfx::BossHit => vec![
            v(Sine, 75.0, 28.0, 0.8, 0.8),
            v(Noise, 3500.0, 300.0, 0.55, 0.5).lp(0.3),
            v(Saw, 140.0, 55.0, 0.45, 0.2).lp(0.25),
            v(Square, 900.0, 220.0, 0.15, 0.1).lp(0.5),
            v(Tri, 1800.0, 400.0, 0.4, 0.12).at(0.03),
        ],
        Sfx::BossStun => vec![
            v(Sine, 60.0, 25.0, 0.8, 0.55),
            v(Noise, 1800.0, 200.0, 0.5, 0.32).lp(0.2),
            v(Saw, 220.0, 70.0, 0.5, 0.1).lp(0.15),
        ],
        Sfx::BossRoar => vec![
            v(Saw, 60.0, 90.0, 1.6, 0.2).lp(0.05).attack(0.4),
            v(Saw, 63.0, 94.0, 1.6, 0.18).lp(0.05).attack(0.4),
            v(Noise, 700.0, 150.0, 1.5, 0.3).lp(0.07).attack(0.3),
            v(Sine, 45.0, 30.0, 1.6, 0.5),
        ],
        Sfx::BossAttack => vec![
            v(Saw, 120.0, 400.0, 0.5, 0.14).lp(0.15).attack(0.05),
            v(Noise, 2000.0, 6000.0, 0.45, 0.14).lp(0.3).attack(0.1),
        ],
        Sfx::BossDown => vec![
            v(Sine, 55.0, 22.0, 2.0, 0.8).curve(1.0),
            v(Noise, 3000.0, 100.0, 2.0, 0.4).lp(0.1),
            v(Saw, 440.0, 55.0, 1.5, 0.12).lp(0.2),
            v(Tri, 1200.0, 200.0, 1.2, 0.1).at(0.1),
        ],
        Sfx::Dying => vec![
            v(Saw, 420.0, 55.0, 0.8, 0.12).lp(0.25),
            v(Noise, 2500.0, 300.0, 0.6, 0.16).lp(0.3),
            v(Tri, 330.0, 165.0, 1.0, 0.1),
        ],
        Sfx::Revive => vec![
            v(Sine, 262.0, 1046.0, 0.9, 0.15),
            v(Tri, 392.0, 1568.0, 0.9, 0.08),
            v(Noise, 1000.0, 8000.0, 0.8, 0.1).lp(0.3).attack(0.2),
        ],
        Sfx::RewindStart => vec![
            v(Noise, 6000.0, 600.0, 0.35, 0.16).lp(0.5),
            v(Saw, 880.0, 110.0, 0.35, 0.07).lp(0.3),
        ],
        Sfx::RewindTick => vec![
            v(Square, 1400.0, 500.0, 0.03, 0.05).lp(0.4),
            v(Noise, 5000.0, 2000.0, 0.03, 0.04).lp(0.6),
        ],
        Sfx::RewindEnd => vec![
            v(Noise, 600.0, 6000.0, 0.3, 0.14).lp(0.5).attack(0.05),
            v(Saw, 110.0, 880.0, 0.3, 0.06).lp(0.3),
        ],
        Sfx::Medal => vec![
            v(Tri, 784.0, 784.0, 0.4, 0.12),
            v(Tri, 1046.0, 1046.0, 0.5, 0.12).at(0.1),
            v(Sine, 1568.0, 1568.0, 0.9, 0.08).at(0.2),
        ],
        Sfx::Victory => [392.0, 494.0, 587.0, 784.0, 988.0, 1175.0]
            .iter()
            .enumerate()
            .map(|(i, f)| v(Tri, *f, *f, 0.7, 0.12).at(i as f32 * 0.12))
            .chain(std::iter::once(
                v(Saw, 196.0, 196.0, 2.2, 0.07).lp(0.08).attack(0.4),
            ))
            .collect(),
        Sfx::Defeat => [330.0, 311.0, 294.0, 247.0]
            .iter()
            .enumerate()
            .map(|(i, f)| v(Saw, *f, *f * 0.98, 0.8, 0.08).lp(0.1).at(i as f32 * 0.3))
            .collect(),
        Sfx::Slam => vec![
            v(Sine, 80.0, 28.0, 0.5, 0.7),
            v(Noise, 2500.0, 250.0, 0.35, 0.4).lp(0.2),
        ],
    }
}

struct Theme {
    bpm: f32,
    /// MIDI root of the key.
    root: f32,
    /// Four chords as semitone offsets from the root.
    chords: [[f32; 3]; 4],
    /// Scale for arpeggios and melody.
    scale: [f32; 5],
    /// Bass hits per 16-step bar.
    bass: [u8; 16],
    /// Arp pattern: scale degree index per step, 255 = rest.
    arp: [u8; 16],
    /// Lead motif: scale degree per 8th note over two bars, 255 = rest.
    lead: [u8; 16],
    pad_wave: Wave,
    arp_wave: Wave,
    lead_wave: Wave,
    arp_oct: f32,
    drums: bool,
}

fn theme(t: u8) -> Theme {
    match t {
        0 => Theme {
            // Mossbluff: bright D major
            bpm: 104.0,
            root: 62.0,
            chords: [
                [0.0, 4.0, 7.0],
                [7.0, 11.0, 14.0],
                [9.0, 12.0, 16.0],
                [5.0, 9.0, 12.0],
            ],
            scale: [0.0, 2.0, 4.0, 7.0, 9.0],
            bass: [1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0],
            arp: [
                0, 255, 2, 255, 1, 255, 3, 255, 2, 255, 4, 255, 3, 255, 1, 255,
            ],
            lead: [4, 255, 3, 4, 255, 2, 255, 3, 4, 255, 4, 3, 2, 255, 1, 255],
            pad_wave: Wave::Saw,
            arp_wave: Wave::Tri,
            lead_wave: Wave::Square,
            arp_oct: 12.0,
            drums: true,
        },
        1 => Theme {
            // Glimmerdeep: sparse A minor with bell tones
            bpm: 84.0,
            root: 57.0,
            chords: [
                [0.0, 3.0, 7.0],
                [8.0, 12.0, 15.0],
                [3.0, 7.0, 10.0],
                [10.0, 14.0, 17.0],
            ],
            scale: [0.0, 3.0, 5.0, 7.0, 10.0],
            bass: [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0],
            arp: [
                0, 255, 255, 2, 255, 255, 4, 255, 255, 3, 255, 255, 1, 255, 2, 255,
            ],
            lead: [
                255, 255, 4, 255, 3, 255, 255, 2, 255, 255, 1, 255, 2, 255, 255, 255,
            ],
            pad_wave: Wave::Tri,
            arp_wave: Wave::Sine,
            lead_wave: Wave::Tri,
            arp_oct: 24.0,
            drums: false,
        },
        2 => Theme {
            // Frostspire: icy E dorian
            bpm: 96.0,
            root: 64.0,
            chords: [
                [0.0, 3.0, 7.0],
                [5.0, 9.0, 12.0],
                [10.0, 14.0, 17.0],
                [0.0, 3.0, 7.0],
            ],
            scale: [0.0, 2.0, 3.0, 7.0, 9.0],
            bass: [1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0],
            arp: [0, 2, 4, 2, 3, 1, 4, 1, 0, 2, 4, 3, 2, 1, 3, 255],
            lead: [
                4, 255, 255, 3, 255, 4, 255, 255, 2, 255, 3, 255, 255, 1, 255, 255,
            ],
            pad_wave: Wave::Tri,
            arp_wave: Wave::Tri,
            lead_wave: Wave::Sine,
            arp_oct: 24.0,
            drums: true,
        },
        3 => Theme {
            // Cinderworks: driving C phrygian
            bpm: 120.0,
            root: 60.0,
            chords: [
                [0.0, 3.0, 7.0],
                [1.0, 5.0, 8.0],
                [0.0, 3.0, 7.0],
                [10.0, 14.0, 17.0],
            ],
            scale: [0.0, 1.0, 3.0, 7.0, 8.0],
            bass: [1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 1],
            arp: [0, 255, 0, 2, 255, 3, 255, 4, 0, 255, 0, 2, 255, 3, 255, 1],
            lead: [4, 255, 3, 255, 4, 4, 255, 2, 3, 255, 1, 255, 2, 3, 255, 255],
            pad_wave: Wave::Saw,
            arp_wave: Wave::Square,
            lead_wave: Wave::Saw,
            arp_oct: 12.0,
            drums: true,
        },
        4 => Theme {
            // Stormcrown: fast F# minor
            bpm: 132.0,
            root: 54.0,
            chords: [
                [0.0, 3.0, 7.0],
                [8.0, 12.0, 15.0],
                [5.0, 8.0, 12.0],
                [7.0, 11.0, 14.0],
            ],
            scale: [0.0, 3.0, 5.0, 7.0, 10.0],
            bass: [1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1],
            arp: [0, 2, 4, 2, 1, 3, 4, 3, 0, 2, 4, 2, 1, 3, 2, 4],
            lead: [4, 255, 4, 3, 255, 2, 3, 255, 4, 255, 3, 2, 255, 1, 2, 255],
            pad_wave: Wave::Saw,
            arp_wave: Wave::Tri,
            lead_wave: Wave::Saw,
            arp_oct: 24.0,
            drums: true,
        },
        _ => Theme {
            // The Stormheart: heavy D minor
            bpm: 144.0,
            root: 50.0,
            chords: [
                [0.0, 3.0, 7.0],
                [8.0, 12.0, 15.0],
                [5.0, 8.0, 12.0],
                [7.0, 11.0, 14.0],
            ],
            scale: [0.0, 3.0, 5.0, 7.0, 10.0],
            bass: [1, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1],
            arp: [
                0, 255, 4, 255, 0, 255, 3, 255, 0, 255, 4, 255, 2, 255, 3, 255,
            ],
            lead: [4, 4, 255, 3, 4, 255, 2, 3, 4, 4, 255, 3, 2, 1, 2, 255],
            pad_wave: Wave::Saw,
            arp_wave: Wave::Square,
            lead_wave: Wave::Saw,
            arp_oct: 12.0,
            drums: true,
        },
    }
}

struct Music {
    step: usize,
    acc: f32,
    intensity: f32,
    target: MusicState,
    theme_id: u8,
}

impl Music {
    fn step_len(&self) -> f32 {
        60.0 / theme(self.theme_id).bpm / 4.0
    }

    fn tick(&mut self, out: &mut Vec<Voice>) {
        let th = theme(self.theme_id);
        let i = self.intensity;
        let s = self.step % 16;
        let bar = self.step / 16;
        let chord = th.chords[bar % 4];
        let step_len = 60.0 / th.bpm / 4.0;
        let bar_len = step_len * 16.0;
        let boss = self.theme_id >= 5;
        let mut push = |mut vc: Voice| {
            vc.music = true;
            out.push(vc);
        };
        // ambience bed: wind, drips, rumble or rain by biome
        if s == 0 && bar % 2 == 0 {
            match self.theme_id {
                0 => push(
                    v(Wave::Noise, 900.0, 900.0, bar_len * 2.2, 0.018)
                        .lp(0.03)
                        .attack(1.2),
                ),
                2 => push(
                    v(Wave::Noise, 1400.0, 1400.0, bar_len * 2.2, 0.04)
                        .lp(0.04)
                        .attack(1.5),
                ),
                3 => {
                    push(v(Wave::Sine, 38.0, 36.0, bar_len * 2.2, 0.07).attack(0.8));
                    push(
                        v(Wave::Noise, 300.0, 300.0, bar_len * 2.2, 0.03)
                            .lp(0.01)
                            .attack(1.0),
                    );
                }
                4 => push(
                    v(Wave::Noise, 5000.0, 5000.0, bar_len * 2.2, 0.022)
                        .lp(0.45)
                        .attack(0.6),
                ),
                _ => {}
            }
        }
        if self.theme_id == 1 && s == 9 && (bar + s).is_multiple_of(2) {
            let f = 1500.0 + ((bar * 37 + s * 11) % 7) as f32 * 160.0;
            push(
                v(Wave::Sine, f, f * 0.7, 0.14, 0.03)
                    .curve(3.0)
                    .pan(((bar % 3) as f32 - 1.0) * 0.5),
            );
        }
        if self.theme_id == 4 && s == 0 && bar % 7 == 3 {
            push(v(Wave::Noise, 500.0, 120.0, 1.6, 0.06).lp(0.05).attack(0.2));
        }
        // pad: always present, opens up with intensity
        if s == 0 {
            for (k, off) in chord.iter().enumerate() {
                let f = midi(th.root + off);
                for det in [0.996, 1.005] {
                    push(
                        v(
                            th.pad_wave,
                            f * det,
                            f * det,
                            bar_len * 1.15,
                            0.03 * (0.55 + 0.7 * i),
                        )
                        .lp(0.025 + 0.05 * i)
                        .attack(0.6)
                        .curve(0.6)
                        .pan(match k {
                            0 => -0.3,
                            2 => 0.3,
                            _ => 0.0,
                        }),
                    );
                }
            }
        }
        // bass
        if th.bass[s] == 1 && (i > 0.25 || (s == 0 && !th.drums)) {
            let f = midi(th.root + chord[0] - 24.0);
            let wave = if boss || self.theme_id == 3 {
                Wave::Saw
            } else {
                Wave::Tri
            };
            let lpk = if wave == Wave::Saw { 0.07 } else { 1.0 };
            push(
                v(wave, f, f * 0.995, step_len * 2.2, 0.16 * (0.5 + 0.7 * i))
                    .curve(1.3)
                    .lp(lpk),
            );
        }
        // arpeggio over chord tones and scale
        let a = th.arp[s];
        if a != 255 && i > 0.18 {
            let deg = th.scale[(a as usize) % 5];
            let tone = if (a as usize).is_multiple_of(2) {
                chord[(a as usize / 2) % 3]
            } else {
                deg
            };
            let f = midi(th.root + tone + th.arp_oct);
            let len = if th.arp_wave == Wave::Sine { 0.9 } else { 0.2 };
            push(
                v(th.arp_wave, f, f, len, 0.05 * (0.4 + i))
                    .curve(2.2)
                    .pan(if (s / 2).is_multiple_of(2) { -0.4 } else { 0.4 }),
            );
        }
        // drums
        if th.drums {
            if i > 0.45 && (s == 0 || s == 8 || (boss && s == 6)) {
                push(v(Wave::Sine, 150.0, 45.0, 0.22, 0.3 * i));
            }
            if i > 0.7 && s % 8 == 4 {
                push(v(Wave::Noise, 4500.0, 4500.0, 0.11, 0.1 * i).lp(0.5));
                push(v(Wave::Sine, 220.0, 140.0, 0.08, 0.05 * i));
            }
            if i > 0.5 && s % 2 == 0 {
                push(v(Wave::Noise, 9000.0, 9000.0, 0.03, 0.025 * i).lp(0.9));
            }
        }
        // lead motif every other bar once things heat up
        if (i > 0.78 || boss) && s % 2 == 0 {
            let idx = ((bar % 2) * 8 + s / 2) % 16;
            let l = th.lead[idx];
            if l != 255 {
                let f = midi(th.root + th.scale[(l as usize) % 5] + 24.0);
                push(
                    v(th.lead_wave, f, f, step_len * 3.0, 0.045 * i)
                        .lp(0.2)
                        .curve(1.4)
                        .pan(0.15),
                );
            }
        }
        // calm sparkle
        if i < 0.45 && (s == 3 || s == 11) && !(bar + s).is_multiple_of(3) {
            let m = th.scale[(bar * 7 + s) % 5] + th.root + 24.0;
            let f = midi(m);
            push(v(Wave::Sine, f, f, 1.0, 0.03).curve(2.5).pan(0.5));
        }
        if boss && s == 0 {
            push(
                v(
                    Wave::Saw,
                    midi(th.root - 24.0),
                    midi(th.root - 24.0),
                    bar_len,
                    0.05,
                )
                .lp(0.03)
                .attack(0.4),
            );
        }
    }
}

pub struct Synth {
    rate: f32,
    voices: Vec<Voice>,
    music: Music,
    pub music_volume: f32,
    pub sfx_volume: f32,
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    delay_pos: usize,
}

impl Synth {
    pub fn new(sample_rate: f32) -> Synth {
        let len = (sample_rate * 0.357) as usize;
        Synth {
            rate: sample_rate,
            voices: Vec::with_capacity(128),
            music: Music {
                step: 0,
                acc: 0.0,
                intensity: 0.2,
                target: MusicState {
                    intensity: 0.2,
                    theme: 0,
                    rewinding: false,
                },
                theme_id: 0,
            },
            music_volume: 0.8,
            sfx_volume: 1.0,
            delay_l: vec![0.0; len.max(1)],
            delay_r: vec![0.0; len.max(1)],
            delay_pos: 0,
        }
    }

    pub fn play(&mut self, s: Sfx) {
        for vc in sfx_voices(s) {
            self.voices.push(vc);
        }
        if self.voices.len() > 110 {
            let excess = self.voices.len() - 110;
            self.voices.drain(..excess);
        }
    }

    pub fn set_music(&mut self, m: MusicState) {
        self.music.target = m;
    }

    pub fn theme(&self) -> u8 {
        self.music.theme_id
    }

    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Fill an interleaved buffer with `channels` channels.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let dt = 1.0 / self.rate;
        let channels = channels.max(1);
        for frame in out.chunks_mut(channels) {
            self.music.acc += dt;
            if self.music.acc >= self.music.step_len() {
                self.music.acc -= self.music.step_len();
                let target = self.music.target.intensity;
                self.music.intensity += (target - self.music.intensity) * 0.2;
                if self.music.target.theme != self.music.theme_id {
                    self.music.theme_id = self.music.target.theme;
                    self.music.step = 0;
                }
                // time stands still while rewinding: the score goes quiet
                if !self.music.target.rewinding {
                    let mut new = vec![];
                    self.music.tick(&mut new);
                    self.music.step += 1;
                    self.voices.extend(new);
                }
            }
            let (mut l, mut r) = (0.0f32, 0.0f32);
            let mut i = 0;
            while i < self.voices.len() {
                let (vl, vr, alive) = self.voices[i].sample(dt);
                let g = if self.voices[i].music {
                    self.music_volume
                } else {
                    self.sfx_volume
                };
                l += vl * g;
                r += vr * g;
                if alive {
                    i += 1;
                } else {
                    self.voices.swap_remove(i);
                }
            }
            let dl = self.delay_l[self.delay_pos];
            let dr = self.delay_r[self.delay_pos];
            self.delay_l[self.delay_pos] = l + dr * 0.35;
            self.delay_r[self.delay_pos] = r + dl * 0.35;
            self.delay_pos = (self.delay_pos + 1) % self.delay_l.len();
            let ol = (l + dl * 0.28).tanh() * 0.8;
            let or = (r + dr * 0.28).tanh() * 0.8;
            if channels == 1 {
                frame[0] = (ol + or) * 0.5;
            } else {
                frame[0] = ol;
                frame[1] = or;
                for extra in frame.iter_mut().skip(2) {
                    *extra = 0.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(s: &mut Synth, secs: f32) -> f32 {
        let mut buf = vec![0.0; (s.rate * secs) as usize * 2];
        s.render(&mut buf, 2);
        assert!(buf.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32
    }

    fn all_sfx() -> Vec<Sfx> {
        vec![
            Sfx::Click,
            Sfx::Select,
            Sfx::Draft,
            Sfx::Error,
            Sfx::Banner,
            Sfx::Step,
            Sfx::Jump,
            Sfx::WallJump,
            Sfx::AirJump,
            Sfx::Land(0),
            Sfx::Land(2),
            Sfx::HookHit,
            Sfx::HookMiss,
            Sfx::Release,
            Sfx::RopeSnap,
            Sfx::Brittle,
            Sfx::Crystal(3),
            Sfx::EchoOrb,
            Sfx::Heart,
            Sfx::Chest,
            Sfx::Hurt,
            Sfx::Stomp,
            Sfx::Ram,
            Sfx::Smash,
            Sfx::Pit,
            Sfx::Checkpoint,
            Sfx::SpitterCharge,
            Sfx::SpitterFire,
            Sfx::GeyserWarn,
            Sfx::GeyserBurst,
            Sfx::StrikeWarn,
            Sfx::StrikeHit,
            Sfx::Bounce,
            Sfx::Crumble,
            Sfx::Cleared,
            Sfx::Shield,
            Sfx::BossHit,
            Sfx::BossStun,
            Sfx::BossRoar,
            Sfx::BossAttack,
            Sfx::BossDown,
            Sfx::Dying,
            Sfx::Revive,
            Sfx::RewindStart,
            Sfx::RewindTick,
            Sfx::RewindEnd,
            Sfx::Medal,
            Sfx::Victory,
            Sfx::Defeat,
            Sfx::Slam,
        ]
    }

    fn sfx_energy(s: Sfx) -> f32 {
        let mut syn = Synth::new(22050.0);
        syn.music_volume = 0.0;
        syn.play(s);
        energy(&mut syn, 0.6)
    }

    #[test]
    fn every_sfx_makes_sound() {
        for s in all_sfx() {
            assert!(sfx_energy(s) > 1e-6, "{s:?} silent");
        }
    }

    #[test]
    fn important_events_are_louder_than_routine_ones() {
        let jump = sfx_energy(Sfx::Jump);
        let step = sfx_energy(Sfx::Step);
        for heavy in [Sfx::Smash, Sfx::BossHit, Sfx::Ram, Sfx::Hurt] {
            assert!(sfx_energy(heavy) > jump * 3.0, "{heavy:?}");
            assert!(sfx_energy(heavy) > step * 6.0, "{heavy:?}");
        }
        assert!(sfx_energy(Sfx::BossHit) > sfx_energy(Sfx::Stomp) * 2.0);
        assert!(sfx_energy(Sfx::Land(2)) > sfx_energy(Sfx::Land(0)) * 2.0);
    }

    #[test]
    fn music_intensity_adds_layers_in_every_theme() {
        for theme in 0..6u8 {
            let mut calm = Synth::new(22050.0);
            calm.set_music(MusicState {
                intensity: 0.1,
                theme,
                rewinding: false,
            });
            let mut hot = Synth::new(22050.0);
            hot.set_music(MusicState {
                intensity: 1.0,
                theme,
                rewinding: false,
            });
            let e_calm = energy(&mut calm, 6.0);
            let e_hot = energy(&mut hot, 6.0);
            assert!(e_calm > 0.0);
            assert!(e_hot > e_calm * 1.4, "theme {theme}: {e_hot} vs {e_calm}");
        }
    }

    #[test]
    fn rewinding_silences_new_music() {
        let mut s = Synth::new(22050.0);
        s.set_music(MusicState {
            intensity: 0.8,
            theme: 2,
            rewinding: true,
        });
        assert!(energy(&mut s, 3.0) < 1e-5);
    }
}
