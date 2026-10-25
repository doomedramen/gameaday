//! Arcade car model. Velocity is split into forward and lateral parts; the
//! lateral part is bled off by tyre grip up to a grip limit, so fast sharp
//! turns slide naturally and the handbrake turns that into a controlled
//! drift. Drifting charges a three-tier mini-turbo; slipstreaming and style
//! fill the nitro meter.

use crate::math::{V2, approach, wrap_angle};
use crate::track::Surface;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    /// -1 brake/reverse .. 1 full throttle.
    pub throttle: f32,
    /// -1 left .. 1 right.
    pub steer: f32,
    pub drift: bool,
    pub nitro: bool,
}

/// Tunable performance numbers. Garage levels and parts modify these.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub top: f32,
    pub accel: f32,
    pub brake: f32,
    pub steer: f32,
    /// Lateral grip limit in px/s^2.
    pub grip: f32,
    pub mass: f32,
    /// Damage taken multiplier.
    pub fragility: f32,
    /// Nitro seconds at full meter.
    pub nitro_secs: f32,
    pub nitro_power: f32,
    /// Slipstream charge rate multiplier.
    pub draft_rate: f32,
    pub draft_range: f32,
    /// Drift charge rate multiplier.
    pub drift_rate: f32,
    pub turbo_len: f32,
    pub offroad: f32,
    pub regen: f32,
    pub nitro_regen: f32,
    pub kerb_nitro: bool,
    pub style_cap: u32,
    pub launch: bool,
    pub pickup_mult: f32,
}

impl Default for Stats {
    fn default() -> Stats {
        Stats {
            top: 640.0,
            accel: 560.0,
            brake: 1050.0,
            steer: 3.25,
            grip: 1350.0,
            mass: 1.0,
            fragility: 1.0,
            nitro_secs: 2.6,
            nitro_power: 1.0,
            draft_rate: 1.0,
            draft_range: 1.0,
            drift_rate: 1.0,
            turbo_len: 1.0,
            offroad: 1.0,
            regen: 0.0,
            nitro_regen: 0.0,
            kerb_nitro: false,
            style_cap: 5,
            launch: false,
            pickup_mult: 1.0,
        }
    }
}

impl Stats {
    /// Maximum yaw rate the steering can produce at speed v (rad/s).
    pub fn yaw_max(&self, v: f32) -> f32 {
        let low = (v / 170.0).clamp(0.0, 1.0);
        let high = 1.0 - 0.38 * ((v - 380.0) / 420.0).clamp(0.0, 1.0);
        self.steer * low * high
    }
}

pub const CAR_LEN: f32 = 62.0;
pub const CAR_RAD: f32 = 17.0;

/// Mini-turbo tier thresholds (seconds of charge).
pub const TIERS: [f32; 3] = [0.55, 1.25, 2.1];

#[derive(Clone, Debug)]
pub struct Car {
    pub pos: V2,
    pub vel: V2,
    pub heading: f32,
    pub yaw: f32,
    pub stats: Stats,
    pub hp: f32,
    pub nitro: f32,
    pub boost_t: f32,
    pub boost_k: f32,
    pub nitro_on: bool,
    pub drifting: bool,
    pub drift_charge: f32,
    pub drift_dir: f32,
    /// Seconds in a slipstream right now.
    pub draft: f32,
    pub surface: Surface,
    pub spin_t: f32,
    pub slip: f32,
    pub hit_flash: f32,
    /// Suspension bob for visuals.
    pub bob: f32,
    pub bob_v: f32,
    /// Simulated gear and rpm (sound and HUD).
    pub gear: u8,
    pub rpm: f32,
    pub shift_t: f32,
    pub last_turbo_tier: u8,
}

/// Something the car did this step that the race cares about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarEvent {
    Turbo(u8),
    TierUp(u8),
    Shift,
    OilSpin,
}

impl Car {
    pub fn new(pos: V2, heading: f32, stats: Stats) -> Car {
        Car {
            pos,
            vel: V2::default(),
            heading,
            yaw: 0.0,
            stats,
            hp: 100.0,
            nitro: 0.25,
            boost_t: 0.0,
            boost_k: 0.0,
            nitro_on: false,
            drifting: false,
            drift_charge: 0.0,
            drift_dir: 0.0,
            draft: 0.0,
            surface: Surface::Road,
            spin_t: 0.0,
            slip: 0.0,
            hit_flash: 0.0,
            bob: 0.0,
            bob_v: 0.0,
            gear: 1,
            rpm: 0.0,
            shift_t: 0.0,
            last_turbo_tier: 0,
        }
    }

    pub fn fwd(&self) -> V2 {
        V2::from_angle(self.heading)
    }
    /// Right-hand side vector in screen space.
    pub fn right(&self) -> V2 {
        -self.fwd().perp()
    }
    pub fn speed(&self) -> f32 {
        self.vel.len()
    }
    pub fn fwd_speed(&self) -> f32 {
        self.vel.dot(self.fwd())
    }
    /// Collision circles (front and rear).
    pub fn circles(&self) -> [V2; 2] {
        let f = self.fwd() * 15.0;
        [self.pos + f, self.pos - f]
    }
    pub fn drift_tier(&self) -> u8 {
        TIERS.iter().filter(|t| self.drift_charge >= **t).count() as u8
    }
    pub fn boosting(&self) -> bool {
        self.boost_t > 0.0 || self.nitro_on
    }

    /// Current top speed after surface, damage and boosts.
    pub fn top_speed(&self) -> f32 {
        let s = &self.stats;
        let surf = match self.surface {
            Surface::Road | Surface::Kerb => 1.0,
            Surface::Oil => 0.95,
            Surface::Runoff => 1.0 - 0.42 / s.offroad.max(0.5),
            Surface::Mud => 1.0 - 0.45 / s.offroad.max(0.5),
            Surface::Sand => 1.0 - 0.25 / s.offroad.max(0.5),
        };
        let dmg = 0.82 + 0.18 * (self.hp / 100.0);
        let draft = 1.0 + 0.07 * (self.draft / 0.6).min(1.0);
        let mut v = s.top * surf * dmg * draft;
        if self.boost_t > 0.0 {
            v *= 1.0 + 0.28 * self.boost_k;
        }
        if self.nitro_on {
            v *= 1.0 + 0.3 * s.nitro_power;
        }
        v
    }

    pub fn step(&mut self, c: Controls, road_grip: f32, dt: f32, events: &mut Vec<CarEvent>) {
        let s = self.stats;
        let fwd = self.fwd();
        let right = self.right();
        let mut vf = self.vel.dot(fwd);
        let mut vr = self.vel.dot(right);
        let speed = self.vel.len();

        // --- nitro and boost timers
        let wants_nitro = c.nitro && self.nitro > 0.02 && self.spin_t <= 0.0;
        self.nitro_on = wants_nitro && (self.nitro_on || self.nitro > 0.12);
        if self.nitro_on {
            self.nitro = (self.nitro - dt / s.nitro_secs).max(0.0);
        }
        self.nitro = (self.nitro + s.nitro_regen * dt).min(1.0);
        self.boost_t = (self.boost_t - dt).max(0.0);
        self.hp = (self.hp + s.regen * dt).min(100.0);

        // --- surface grip
        let surf_grip = match self.surface {
            Surface::Road => 1.0,
            Surface::Kerb => 1.05,
            Surface::Runoff => 0.7,
            Surface::Oil => 0.12,
            Surface::Mud => 0.6,
            Surface::Sand => 0.55,
        } * road_grip;
        if self.surface == Surface::Oil && self.spin_t <= 0.0 && speed > 220.0 && !self.drifting {
            self.spin_t = 0.75;
            events.push(CarEvent::OilSpin);
        }
        self.spin_t = (self.spin_t - dt).max(0.0);

        // --- drift state
        let steer = if self.spin_t > 0.0 {
            0.0
        } else {
            c.steer.clamp(-1.0, 1.0)
        };
        if c.drift && !self.drifting && steer.abs() > 0.3 && vf > 230.0 {
            self.drifting = true;
            self.drift_dir = steer.signum();
            self.drift_charge = 0.0;
        }
        if self.drifting && (!c.drift || vf < 140.0 || self.spin_t > 0.0) {
            self.drifting = false;
            let tier = self.drift_tier();
            if tier > 0 && self.spin_t <= 0.0 {
                let len = [0.0, 0.55, 0.95, 1.45][tier as usize] * s.turbo_len;
                self.boost_t = self.boost_t.max(len);
                self.boost_k = [0.0, 0.7, 0.9, 1.1][tier as usize];
                events.push(CarEvent::Turbo(tier));
                // a little kick forward
                vf += 60.0 + 30.0 * tier as f32;
            }
            self.drift_charge = 0.0;
        }

        // --- engine and brakes
        let top = self.top_speed();
        let throttle = c.throttle.clamp(-1.0, 1.0);
        let boost_acc = if self.nitro_on {
            700.0 * s.nitro_power
        } else {
            0.0
        } + if self.boost_t > 0.0 {
            650.0 * self.boost_k
        } else {
            0.0
        };
        if throttle > 0.0 || boost_acc > 0.0 {
            let room = (1.0 - (vf / top).max(0.0).powf(2.2)).max(0.0);
            let acc = s.accel * throttle.max(0.0) * (0.55 + 0.45 * room) * room.sqrt();
            vf += (acc + boost_acc * room.sqrt().max(0.25)) * dt;
        }
        if throttle < 0.0 {
            if vf > 15.0 {
                vf = (vf - s.brake * -throttle * dt).max(0.0);
            } else {
                vf = (vf - 380.0 * -throttle * dt).max(-210.0);
            }
        }
        // drag and rolling resistance
        let over = (vf - top).max(0.0);
        vf -= (over * 2.2
            + vf.signum()
                * (40.0 + vf.abs() * 0.15)
                * if throttle.abs() < 0.05 { 1.6 } else { 0.4 })
            * dt;
        if self.surface == Surface::Runoff || self.surface == Surface::Mud {
            vf -= vf * 0.6 * dt / s.offroad.max(0.5);
        }

        // --- steering
        let nf;
        let nr;
        if self.drifting && self.spin_t <= 0.0 {
            // Arcade drift: the velocity arcs at a steerable rate while the
            // nose holds a slip angle into the turn. Never spins out.
            let trim = steer * self.drift_dir; // +1 tighter, -1 wider
            let v = fwd * vf + right * vr;
            let sp = v.len().max(1.0);
            let turn = self.drift_dir
                * s.yaw_max(sp)
                * (0.86 + 0.32 * trim)
                * (0.85 + 0.15 * surf_grip.min(1.0));
            let vang = v.angle() + turn * dt;
            let sp = sp - sp * 0.07 * dt;
            self.vel = V2::from_angle(vang) * sp;
            let slip_target = 0.42 + 0.14 * trim;
            let want = vang + self.drift_dir * slip_target;
            let diff = wrap_angle(want - self.heading);
            let old_h = self.heading;
            self.heading = wrap_angle(self.heading + diff * (9.0 * dt).min(1.0));
            self.yaw = wrap_angle(self.heading - old_h) / dt.max(1e-4);
            nf = self.fwd();
            nr = self.right();
        } else {
            let mut yaw_target = steer * s.yaw_max(vf.abs()) * vf.signum();
            if self.spin_t > 0.0 {
                yaw_target = 9.0 * if self.yaw >= 0.0 { 1.0 } else { -1.0 };
            }
            self.yaw = approach(
                self.yaw,
                yaw_target,
                16.0 * dt * (1.0 + (yaw_target - self.yaw).abs()),
            );
            self.heading = wrap_angle(self.heading + self.yaw * dt);

            // lateral grip: bleed lateral velocity up to the grip limit
            let lat_kill = s.grip * surf_grip * dt;
            if vr.abs() <= lat_kill {
                vr = 0.0;
            } else {
                vr -= vr.signum() * lat_kill;
            }
            // the old forward speed follows the nose only as much as grip
            // allows; the rest shows up as slide next step
            nf = self.fwd();
            nr = self.right();
            let old = fwd * vf + right * vr;
            self.vel = nf * old.dot(nf) + nr * old.dot(nr);
        }
        let side = self.vel.dot(nr);
        let _ = nf;

        self.slip = if speed > 60.0 {
            (side / speed.max(1.0)).abs()
        } else {
            0.0
        };
        if self.drifting {
            let before = self.drift_tier();
            if self.slip > 0.12 && speed > 200.0 {
                self.drift_charge += dt * s.drift_rate * (0.7 + 0.6 * (speed / 600.0).min(1.0));
            }
            let after = self.drift_tier();
            if after > before {
                events.push(CarEvent::TierUp(after));
            }
        }

        self.pos += self.vel * dt;

        // --- visuals: suspension bob, gears
        let bump = match self.surface {
            Surface::Kerb => 1.0,
            Surface::Runoff | Surface::Mud | Surface::Sand => 0.6,
            _ => 0.0,
        } * (speed / 600.0);
        self.bob_v += (-self.bob * 120.0 - self.bob_v * 9.0) * dt
            + bump * ((self.pos.x + self.pos.y) * 0.3).sin() * 40.0 * dt;
        self.bob += self.bob_v * dt;
        self.hit_flash = (self.hit_flash - dt * 4.0).max(0.0);
        let gear_top = [0.0, 0.2, 0.38, 0.56, 0.76, 1.01];
        let frac = (vf.abs() / s.top).clamp(0.0, 1.2);
        let mut g = 1u8;
        while (g as usize) < 5 && frac > gear_top[g as usize] {
            g += 1;
        }
        if g != self.gear {
            if g > self.gear {
                events.push(CarEvent::Shift);
                self.shift_t = 0.12;
            }
            self.gear = g;
        }
        self.shift_t = (self.shift_t - dt).max(0.0);
        let lo = gear_top[g as usize - 1];
        let hi = gear_top[g as usize];
        let target_rpm = ((frac - lo) / (hi - lo)).clamp(0.0, 1.1) * 0.8
            + 0.2 * throttle.max(0.0)
            + if self.drifting { 0.15 } else { 0.0 };
        self.rpm += (target_rpm - self.rpm) * (12.0 * dt).min(1.0);
    }

    /// Apply damage from an impact of relative normal speed `v`. Returns the
    /// damage dealt.
    pub fn impact(&mut self, v: f32, scale: f32) -> f32 {
        let d = ((v - 110.0).max(0.0) * 0.06 * scale * self.stats.fragility).min(30.0);
        self.hp = (self.hp - d).max(0.0);
        if d > 0.5 {
            self.hit_flash = 1.0;
            // a hard knock breaks a drift
            if d > 6.0 {
                self.drifting = false;
                self.drift_charge = 0.0;
            }
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::v2;

    fn run(car: &mut Car, c: Controls, secs: f32) {
        let mut ev = Vec::new();
        let steps = (secs * 120.0) as usize;
        for _ in 0..steps {
            car.step(c, 1.0, 1.0 / 120.0, &mut ev);
        }
    }

    #[test]
    fn accelerates_to_top_speed() {
        let mut car = Car::new(v2(0.0, 0.0), 0.0, Stats::default());
        run(
            &mut car,
            Controls {
                throttle: 1.0,
                ..Default::default()
            },
            1.5,
        );
        assert!(car.speed() > 380.0, "slow off the line: {}", car.speed());
        run(
            &mut car,
            Controls {
                throttle: 1.0,
                ..Default::default()
            },
            6.0,
        );
        assert!((car.speed() - 640.0).abs() < 40.0, "top {}", car.speed());
    }

    #[test]
    fn brakes_hard() {
        let mut car = Car::new(v2(0.0, 0.0), 0.0, Stats::default());
        car.vel = v2(600.0, 0.0);
        run(
            &mut car,
            Controls {
                throttle: -1.0,
                ..Default::default()
            },
            0.5,
        );
        assert!(car.speed() < 120.0, "{}", car.speed());
    }

    #[test]
    fn drift_charges_a_turbo() {
        let mut car = Car::new(v2(0.0, 0.0), 0.0, Stats::default());
        car.vel = v2(560.0, 0.0);
        let mut ev = Vec::new();
        for _ in 0..(1.6 * 120.0) as usize {
            car.step(
                Controls {
                    throttle: 1.0,
                    steer: 1.0,
                    drift: true,
                    nitro: false,
                },
                1.0,
                1.0 / 120.0,
                &mut ev,
            );
        }
        assert!(
            car.drift_tier() >= 2,
            "tier {} charge {}",
            car.drift_tier(),
            car.drift_charge
        );
        car.step(
            Controls {
                throttle: 1.0,
                ..Default::default()
            },
            1.0,
            1.0 / 120.0,
            &mut ev,
        );
        assert!(
            ev.iter()
                .any(|e| matches!(e, CarEvent::Turbo(t) if *t >= 2))
        );
        assert!(car.boost_t > 0.5);
    }

    #[test]
    fn nitro_raises_top_speed() {
        let mut a = Car::new(v2(0.0, 0.0), 0.0, Stats::default());
        a.nitro = 1.0;
        let mut b = a.clone();
        run(
            &mut a,
            Controls {
                throttle: 1.0,
                ..Default::default()
            },
            2.4,
        );
        run(
            &mut b,
            Controls {
                throttle: 1.0,
                nitro: true,
                ..Default::default()
            },
            2.4,
        );
        assert!(
            b.speed() > a.speed() + 80.0,
            "{} vs {}",
            b.speed(),
            a.speed()
        );
    }

    #[test]
    fn full_lock_turn_radius_is_drivable() {
        let st = Stats::default();
        // at 300 px/s the car turns tighter than a 120px hairpin
        assert!(300.0 / st.yaw_max(300.0) < 120.0);
    }
}
