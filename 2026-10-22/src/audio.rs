//! Real-time synthesiser: one-shot sound effects plus an adaptive
//! procedural score whose layers follow the game's tension. Pure sample
//! generation - `main.rs` feeds it to the audio device.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Click,
    Select,
    Step,
    Slash,
    Bash,
    Arrow,
    Flare,
    Bolt,
    Curse,
    Claw,
    Slam,
    Hit,
    BossHit,
    Hurt,
    Heal,
    Kindle,
    Snuff,
    Death,
    SquadDeath,
    Fall,
    Splash,
    Summon,
    Eclipse,
    Guard,
    Pickup,
    Chain(u32),
    Banner,
    Victory,
    Defeat,
    Draft,
    Gloom,
    Error,
    Turn,
}

/// What the score should be doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicState {
    /// 0 = calm, 1 = full battle.
    pub intensity: f32,
    pub boss: bool,
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
        Sfx::Step => vec![
            v(Noise, 3000.0, 1500.0, 0.05, 0.05).lp(0.15),
            v(Sine, 110.0, 80.0, 0.05, 0.08),
        ],
        Sfx::Slash => vec![
            v(Noise, 8000.0, 2000.0, 0.13, 0.22).lp(0.6),
            v(Saw, 900.0, 200.0, 0.1, 0.07).lp(0.4),
        ],
        Sfx::Bash => vec![
            v(Sine, 150.0, 45.0, 0.22, 0.5),
            v(Noise, 4000.0, 800.0, 0.12, 0.25).lp(0.3),
            v(Square, 220.0, 110.0, 0.08, 0.08).lp(0.4),
        ],
        Sfx::Arrow => vec![
            v(Noise, 6000.0, 1500.0, 0.16, 0.1).lp(0.5).attack(0.05),
            v(Tri, 1700.0, 800.0, 0.06, 0.1).at(0.13),
        ],
        Sfx::Flare => vec![
            v(Noise, 500.0, 6000.0, 0.38, 0.18).lp(0.35).attack(0.08),
            v(Sine, 400.0, 900.0, 0.3, 0.09),
            v(Saw, 200.0, 600.0, 0.25, 0.04).lp(0.2),
        ],
        Sfx::Bolt => vec![
            v(Sine, 900.0, 300.0, 0.2, 0.13),
            v(Square, 1800.0, 600.0, 0.15, 0.03).lp(0.3),
        ],
        Sfx::Curse => vec![
            v(Saw, 220.0, 200.0, 0.45, 0.07).lp(0.15).attack(0.05),
            v(Saw, 233.0, 212.0, 0.45, 0.07).lp(0.15).attack(0.05),
            v(Tri, 110.0, 80.0, 0.4, 0.12),
        ],
        Sfx::Claw => vec![
            v(Noise, 5000.0, 2000.0, 0.08, 0.2).lp(0.45),
            v(Square, 400.0, 180.0, 0.07, 0.06).lp(0.4),
        ],
        Sfx::Slam => vec![
            v(Sine, 95.0, 32.0, 0.42, 0.6),
            v(Noise, 2000.0, 300.0, 0.28, 0.3).lp(0.2),
        ],
        Sfx::Hit => vec![
            v(Noise, 5000.0, 1500.0, 0.07, 0.2).lp(0.5),
            v(Square, 320.0, 140.0, 0.06, 0.07).lp(0.5),
        ],
        Sfx::BossHit => vec![
            v(Sine, 75.0, 28.0, 0.6, 0.75),
            v(Noise, 3000.0, 300.0, 0.38, 0.45).lp(0.25),
            v(Saw, 140.0, 55.0, 0.32, 0.15).lp(0.25),
            v(Square, 900.0, 220.0, 0.1, 0.08).lp(0.5),
        ],
        Sfx::Hurt => vec![
            v(Square, 300.0, 120.0, 0.2, 0.12).lp(0.35),
            v(Noise, 3000.0, 1000.0, 0.1, 0.1).lp(0.4),
        ],
        Sfx::Heal => vec![
            v(Sine, 523.0, 523.0, 0.3, 0.1),
            v(Sine, 659.0, 659.0, 0.3, 0.1).at(0.06),
            v(Sine, 784.0, 784.0, 0.35, 0.1).at(0.12),
            v(Sine, 1046.0, 1046.0, 0.5, 0.05).at(0.18),
        ],
        Sfx::Kindle => vec![
            v(Noise, 300.0, 3000.0, 0.45, 0.2).lp(0.3).attack(0.06),
            v(Sine, 300.0, 620.0, 0.3, 0.08),
            v(Noise, 9000.0, 9000.0, 0.5, 0.05).lp(0.8).at(0.1),
        ],
        Sfx::Snuff => vec![v(Noise, 7000.0, 400.0, 0.4, 0.14).lp(0.7).attack(0.02)],
        Sfx::Death => vec![
            v(Saw, 420.0, 55.0, 0.42, 0.09).lp(0.25),
            v(Noise, 2500.0, 300.0, 0.4, 0.14).lp(0.3),
            v(Sine, 800.0, 100.0, 0.3, 0.05),
        ],
        Sfx::SquadDeath => vec![
            v(Tri, 440.0, 220.0, 0.7, 0.18),
            v(Sine, 330.0, 330.0, 0.9, 0.08).at(0.15),
            v(Sine, 262.0, 262.0, 1.1, 0.08).at(0.35),
        ],
        Sfx::Fall => vec![
            v(Sine, 900.0, 70.0, 0.85, 0.14).curve(0.6),
            v(Noise, 1500.0, 400.0, 0.8, 0.05).lp(0.2).attack(0.3),
        ],
        Sfx::Splash => vec![
            v(Noise, 4000.0, 800.0, 0.45, 0.3).lp(0.45),
            v(Sine, 220.0, 60.0, 0.25, 0.15),
        ],
        Sfx::Summon => vec![
            v(Saw, 55.0, 120.0, 0.65, 0.16).lp(0.1).attack(0.2),
            v(Noise, 800.0, 200.0, 0.55, 0.2).lp(0.12).attack(0.2),
            v(Sine, 1200.0, 400.0, 0.4, 0.04).at(0.2),
        ],
        Sfx::Eclipse => vec![
            v(Sine, 55.0, 24.0, 1.5, 0.75).curve(1.0),
            v(Saw, 110.0, 38.0, 1.3, 0.16).lp(0.08),
            v(Noise, 900.0, 120.0, 1.3, 0.3).lp(0.06).attack(0.1),
            v(Square, 55.0, 52.0, 1.1, 0.05).lp(0.1),
        ],
        Sfx::Guard => vec![
            v(Tri, 300.0, 620.0, 0.16, 0.13),
            v(Sine, 1240.0, 1240.0, 0.35, 0.05).at(0.08),
        ],
        Sfx::Pickup => vec![
            v(Sine, 880.0, 1760.0, 0.12, 0.11),
            v(Sine, 1320.0, 1320.0, 0.25, 0.07).at(0.08),
        ],
        Sfx::Chain(n) => {
            let f = 523.0 * 2f32.powf((n.min(8) as f32 * 2.0) / 12.0);
            vec![
                v(Sine, f, f, 0.22, 0.13),
                v(Tri, f * 2.0, f * 2.0, 0.18, 0.05).at(0.04),
                v(Sine, f * 1.5, f * 1.5, 0.25, 0.06).at(0.08),
            ]
        }
        Sfx::Banner => vec![
            v(Tri, 220.0, 220.0, 0.6, 0.07).attack(0.05),
            v(Tri, 330.0, 330.0, 0.6, 0.06).attack(0.05),
        ],
        Sfx::Victory => [440.0, 554.0, 659.0, 880.0, 1109.0]
            .iter()
            .enumerate()
            .map(|(i, f)| v(Tri, *f, *f, 0.6, 0.12).at(i as f32 * 0.11))
            .chain(std::iter::once(
                v(Saw, 220.0, 220.0, 1.6, 0.06).lp(0.08).attack(0.3),
            ))
            .collect(),
        Sfx::Defeat => [330.0, 311.0, 294.0, 247.0]
            .iter()
            .enumerate()
            .map(|(i, f)| v(Saw, *f, *f * 0.98, 0.7, 0.08).lp(0.1).at(i as f32 * 0.28))
            .collect(),
        Sfx::Draft => vec![
            v(Sine, 660.0, 660.0, 0.2, 0.09),
            v(Sine, 880.0, 880.0, 0.3, 0.09).at(0.07),
        ],
        Sfx::Gloom => vec![
            v(Saw, 55.0, 40.0, 1.1, 0.18).lp(0.05).attack(0.2),
            v(Sine, 110.0, 104.0, 1.1, 0.1),
        ],
        Sfx::Error => vec![v(Square, 150.0, 140.0, 0.1, 0.08).lp(0.3)],
        Sfx::Turn => vec![
            v(Tri, 392.0, 523.0, 0.15, 0.1),
            v(Sine, 784.0, 784.0, 0.25, 0.06).at(0.1),
        ],
    }
}

struct Music {
    step: usize,
    acc: f32,
    intensity: f32,
    target: MusicState,
}

impl Music {
    const STEP: f32 = 60.0 / 84.0 / 4.0;

    fn chord(&self, bar: usize) -> [f32; 3] {
        if self.target.boss {
            [
                [57.0, 60.0, 64.0],
                [58.0, 62.0, 65.0],
                [57.0, 60.0, 64.0],
                [56.0, 59.0, 64.0],
            ][bar % 4]
        } else {
            [
                [57.0, 60.0, 64.0],
                [53.0, 57.0, 60.0],
                [48.0, 52.0, 55.0],
                [55.0, 59.0, 62.0],
            ][bar % 4]
        }
    }

    fn tick(&mut self, out: &mut Vec<Voice>) {
        let i = self.intensity;
        let s = self.step % 16;
        let bar = self.step / 16;
        let chord = self.chord(bar);
        let bar_len = Music::STEP * 16.0;
        let mut push = |mut vc: Voice| {
            vc.music = true;
            out.push(vc);
        };
        if s == 0 {
            for (k, m) in chord.iter().enumerate() {
                let f = midi(*m);
                for det in [0.997, 1.004] {
                    push(
                        v(
                            Wave::Saw,
                            f * det,
                            f * det,
                            bar_len * 1.1,
                            0.035 * (0.6 + 0.6 * i),
                        )
                        .lp(0.03 + 0.04 * i)
                        .attack(0.7)
                        .curve(0.6)
                        .pan(if k == 0 {
                            -0.3
                        } else if k == 2 {
                            0.3
                        } else {
                            0.0
                        }),
                    );
                }
            }
        }
        if i > 0.3 && (s == 0 || s == 6 || s == 10) {
            let f = midi(chord[0] - 12.0);
            push(v(Wave::Tri, f, f, 0.45, 0.16 * i).curve(1.2));
        }
        if i > 0.45 && s.is_multiple_of(2) {
            let pattern = [0, 1, 2, 1, 0, 2, 1, 2];
            let m = chord[pattern[(s / 2) % 8]] + 12.0;
            let f = midi(m);
            push(
                v(Wave::Tri, f, f, 0.22, 0.045 * i)
                    .curve(2.0)
                    .pan(if s.is_multiple_of(4) { -0.4 } else { 0.4 }),
            );
        }
        if i > 0.65 {
            if s == 0 || s == 8 {
                push(v(Wave::Sine, 130.0, 42.0, 0.25, 0.32 * i));
            }
            if s % 4 == 2 {
                push(v(Wave::Noise, 9000.0, 9000.0, 0.04, 0.035 * i).lp(0.9));
            }
            if self.target.boss && s >= 12 {
                let f = 90.0 - (s - 12) as f32 * 8.0;
                push(v(Wave::Sine, f, f * 0.6, 0.15, 0.2));
            }
        }
        if self.target.boss && s == 0 {
            push(v(Wave::Saw, 55.0, 55.0, bar_len, 0.05).lp(0.03).attack(0.5));
        }
        if i < 0.4 && (s == 3 || s == 11) && !(bar + s).is_multiple_of(3) {
            let pent = [69.0, 72.0, 74.0, 76.0, 79.0, 81.0];
            let m = pent[(bar * 7 + s) % pent.len()] + 12.0;
            let f = midi(m);
            push(v(Wave::Sine, f, f, 0.9, 0.035).curve(2.5).pan(0.5));
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
                    boss: false,
                },
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

    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Fill an interleaved buffer with `channels` channels.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let dt = 1.0 / self.rate;
        let channels = channels.max(1);
        for frame in out.chunks_mut(channels) {
            self.music.acc += dt;
            if self.music.acc >= Music::STEP {
                self.music.acc -= Music::STEP;
                let target = self.music.target.intensity;
                self.music.intensity += (target - self.music.intensity) * 0.25;
                let mut new = vec![];
                self.music.tick(&mut new);
                self.music.step += 1;
                self.voices.extend(new);
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

    #[test]
    fn every_sfx_makes_sound() {
        let all = [
            Sfx::Click,
            Sfx::Select,
            Sfx::Step,
            Sfx::Slash,
            Sfx::Bash,
            Sfx::Arrow,
            Sfx::Flare,
            Sfx::Bolt,
            Sfx::Curse,
            Sfx::Claw,
            Sfx::Slam,
            Sfx::Hit,
            Sfx::BossHit,
            Sfx::Hurt,
            Sfx::Heal,
            Sfx::Kindle,
            Sfx::Snuff,
            Sfx::Death,
            Sfx::SquadDeath,
            Sfx::Fall,
            Sfx::Splash,
            Sfx::Summon,
            Sfx::Eclipse,
            Sfx::Guard,
            Sfx::Pickup,
            Sfx::Chain(3),
            Sfx::Banner,
            Sfx::Victory,
            Sfx::Defeat,
            Sfx::Draft,
            Sfx::Gloom,
            Sfx::Error,
            Sfx::Turn,
        ];
        for s in all {
            let mut syn = Synth::new(22050.0);
            syn.music_volume = 0.0;
            syn.play(s);
            assert!(energy(&mut syn, 0.4) > 1e-6, "{s:?} silent");
        }
    }

    #[test]
    fn boss_hit_is_heavier_than_hit() {
        let mut a = Synth::new(22050.0);
        a.music_volume = 0.0;
        a.play(Sfx::Hit);
        let mut b = Synth::new(22050.0);
        b.music_volume = 0.0;
        b.play(Sfx::BossHit);
        assert!(energy(&mut b, 0.5) > energy(&mut a, 0.5) * 3.0);
    }

    #[test]
    fn music_intensity_adds_layers() {
        let mut calm = Synth::new(22050.0);
        calm.set_music(MusicState {
            intensity: 0.1,
            boss: false,
        });
        let mut hot = Synth::new(22050.0);
        hot.set_music(MusicState {
            intensity: 1.0,
            boss: true,
        });
        let e_calm = energy(&mut calm, 4.0);
        let e_hot = energy(&mut hot, 4.0);
        assert!(e_calm > 0.0);
        assert!(e_hot > e_calm * 1.5, "{e_hot} vs {e_calm}");
    }
}
