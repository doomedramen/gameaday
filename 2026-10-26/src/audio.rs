//! Soft, deterministic instrumental synthesis. No device or engine types.
use std::f32::consts::TAU;
#[derive(Clone, Copy)]
pub struct AudioState {
    pub mute: bool,
    pub season: usize,
    pub battle: bool,
    pub boss: bool,
    pub event: u64,
    pub impact: i32,
}
pub struct Synth {
    rate: f32,
    sample: u64,
    last_event: u64,
    age: f32,
    impact: i32,
}
impl Synth {
    pub fn new(rate: u32) -> Self {
        Self {
            rate: rate as f32,
            sample: 0,
            last_event: 0,
            age: 9.,
            impact: 0,
        }
    }
    pub fn next_sample(&mut self, s: AudioState) -> f32 {
        if s.event != self.last_event {
            self.last_event = s.event;
            self.age = 0.;
            self.impact = s.impact;
        }
        let t = self.sample as f32 / self.rate;
        self.sample += 1;
        self.age += 1. / self.rate;
        let bpm = if s.boss {
            88.
        } else if s.battle {
            76.
        } else {
            64.
        };
        let beat = t * bpm / 60.;
        let step = beat as usize;
        let part = beat.fract() * 60. / bpm;
        let scales: [[f32; 8]; 4] = [
            [0., 4., 7., 12., 7., 4., 2., 7.],
            [0., 3., 7., 10., 7., 3., 5., 10.],
            [0., 3., 7., 12., 10., 7., 5., 3.],
            [0., 5., 7., 12., 7., 5., 2., 7.],
        ];
        let base = if s.season == 3 { 174.61 } else { 196. };
        let f = base * 2_f32.powf(scales[s.season.min(3)][step % 8] / 12.);
        let note = (TAU * f * part).sin() * (-part * 5.).exp()
            + (TAU * f * 2. * part).sin() * (-part * 9.).exp() * 0.22;
        let chord_phase = (beat / 4.).fract() * 240. / bpm;
        let bass = (TAU * base * 0.5 * chord_phase).sin() * (-chord_phase * 1.5).exp();
        let mut out = note * 0.065 + bass * 0.035;
        if s.battle {
            let p = (beat * 2.).fract();
            out += (TAU * 880. * t).sin() * (-p * 32.).exp() * 0.005;
        }
        if self.age < 0.8 {
            let a = self.age;
            let effect = match self.impact {
                1 => (TAU * (580. + a * 300.) * a).sin() * (-a * 16.).exp() * 0.09,
                2 => (TAU * (260. - a * 80.) * a).sin() * (-a * 12.).exp() * 0.16,
                3 => {
                    (TAU * 392. * a).sin() * (-a * 7.).exp() * 0.15
                        + (TAU * 587. * a).sin() * (-a * 9.).exp() * 0.08
                }
                4 => (TAU * (105. - a * 45.) * a).sin() * (-a * 8.).exp() * 0.24,
                5 => {
                    (TAU * 523.25 * a).sin() * (-a * 4.).exp() * 0.12
                        + (TAU * 659.25 * a).sin() * (-a * 5.).exp() * 0.09
                        + (TAU * 783.99 * a).sin() * (-a * 6.).exp() * 0.07
                }
                _ => 0.,
            };
            out += effect * (a * 160.).min(1.);
        }
        if s.mute { 0. } else { out.clamp(-0.6, 0.6) }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mix_is_finite_and_bounded() {
        let mut synth = Synth::new(48000);
        for impact in 0..6 {
            let mut peak = 0_f32;
            for _ in 0..48000 {
                let n = synth.next_sample(AudioState {
                    mute: false,
                    season: 3,
                    battle: true,
                    boss: true,
                    event: impact as u64,
                    impact,
                });
                assert!(n.is_finite());
                peak = peak.max(n.abs());
            }
            assert!(peak < 0.61);
        }
        assert_eq!(
            synth.next_sample(AudioState {
                mute: true,
                season: 0,
                battle: false,
                boss: false,
                event: 0,
                impact: 0
            }),
            0.
        );
    }
}
