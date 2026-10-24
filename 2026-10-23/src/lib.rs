//! Night Signal: engine-free radio station management campaign.
use std::{fs, path::Path};
pub mod audio;
pub const W: usize = 1280;
pub const H: usize = 720;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Screen {
    Title,
    Playing,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Program {
    Bulletin,
    Music,
    Advice,
    Rumor,
}
#[derive(Clone, Debug)]
pub struct District {
    pub name: &'static str,
    pub need: u8,
    pub trust: i32,
    pub audience: i32,
    pub relay: bool,
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Debug)]
pub struct Game {
    pub screen: Screen,
    pub night: u8,
    pub power: i32,
    pub audience: i32,
    pub trust: i32,
    pub funds: i32,
    pub score: i32,
    pub streak: u8,
    pub program: Program,
    pub budget: [bool; 5],
    pub districts: Vec<District>,
    pub upgrades: u8,
    pub best: i32,
    pub events: u32,
    pub reason: String,
    pub seed: u64,
    pub muted: bool,
    pub tutorial: bool,
}
impl Game {
    pub fn new(seed: u64, best: i32) -> Self {
        let mut g = Self {
            screen: Screen::Title,
            night: 0,
            power: 12,
            audience: 120,
            trust: 50,
            funds: 8,
            score: 0,
            streak: 0,
            program: Program::Bulletin,
            budget: [true, true, false, false, false],
            districts: vec![],
            upgrades: 0,
            best,
            events: 0,
            reason: String::new(),
            seed,
            muted: false,
            tutorial: true,
        };
        g.make_map();
        g
    }
    fn make_map(&mut self) {
        let names = [
            "Old Quay",
            "Market Row",
            "Glass Hill",
            "Southbank",
            "Orchard End",
            "Ironworks",
            "North Loop",
            "Cedar Park",
        ];
        self.districts = names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let h = mix(self.seed + i as u64 * 971);
                District {
                    name: n,
                    need: ((h % 4) + 1) as u8,
                    trust: 38 + (h % 25) as i32,
                    audience: 40 + (h % 75) as i32,
                    relay: i < 2,
                    x: 170 + (i as i32 % 4) * 172,
                    y: 240 + (i as i32 / 4) * 170,
                }
            })
            .collect();
    }
    pub fn start(&mut self) {
        self.screen = Screen::Playing;
        self.night = 1;
        self.tutorial = true;
    }
    pub fn choose(&mut self, n: u8) {
        self.program = match n % 4 {
            0 => Program::Bulletin,
            1 => Program::Music,
            2 => Program::Advice,
            _ => Program::Rumor,
        };
    }
    pub fn toggle_relay(&mut self, i: usize) {
        if let Some(d) = self.districts.get_mut(i)
            && i >= 2
        {
            d.relay = !d.relay;
        }
    }
    pub fn toggle_power(&mut self, i: usize) {
        if i < 5 {
            self.budget[i] = !self.budget[i];
        }
    }
    pub fn upgrade(&mut self) {
        if self.funds >= 6 && self.upgrades < 3 {
            self.funds -= 6;
            self.upgrades += 1;
            self.power += 2;
        }
    }
    pub fn broadcast(&mut self) {
        if self.screen != Screen::Playing {
            return;
        }
        self.events += 1;
        let allocated = self.budget.iter().filter(|x| **x).count() as i32;
        if allocated > self.power {
            self.power = 0;
        } else {
            self.power -= allocated;
        }
        let (need, reach, trust, warm, label): (u8, i32, i32, i32, &str) = match self.program {
            Program::Bulletin => (1, 34, 5, 1, "Verified bulletin"),
            Program::Music => (2, 27, 2, 8, "Request hour"),
            Program::Advice => (3, 22, 8, 5, "Open lines"),
            Program::Rumor => (4, 58, -12, 0, "Unverified tip"),
        };
        let mut gain = 0;
        let mut resonant = 0;
        let tuned_reach = reach + if self.budget[0] { 8 } else { 0 } + self.upgrades as i32 * 4;
        for d in &mut self.districts {
            let powered = d.relay && self.budget[4];
            let match_need = d.need == need;
            let delta = if match_need {
                tuned_reach + (if powered { tuned_reach / 2 } else { 0 })
            } else {
                tuned_reach / 5 + (if powered { tuned_reach / 8 } else { 0 })
            };
            d.audience += delta;
            d.trust = (d.trust
                + trust
                + (if match_need { 3 } else { -2 })
                + (if self.budget[2] && trust > 0 { 2 } else { 0 }))
            .clamp(0, 100);
            gain += delta;
            if match_need && powered {
                resonant += 1;
            }
        }
        let cost = if self.budget[0] { 2 } else { 0 };
        self.power = (self.power - cost).max(0);
        let combo = if resonant > 0 {
            self.streak = self.streak.saturating_add(1);
            resonant * self.streak as i32 * 5
        } else {
            self.streak = 0;
            0
        };
        self.audience += gain / 2 + warm + if self.budget[1] { 5 } else { 0 };
        self.trust = (self.trust + trust / 3 + if self.budget[2] { 2 } else { 0 }).clamp(0, 100);
        self.score += gain + combo + if self.budget[3] { 12 } else { 0 };
        self.funds += (gain / 100).max(1)
            + if self.trust > 60 { 1 } else { 0 }
            + if self.budget[3] { 1 } else { 0 };
        for d in &mut self.districts {
            d.need = ((d.need as usize + self.night as usize + 1) % 4 + 1) as u8;
        }
        self.night += 1;
        self.power = (self.power + 10 + self.upgrades as i32 * 2).min(20);
        if self.night.is_multiple_of(3) {
            self.audience -= 25;
            self.trust -= 2;
            self.reason = "A rolling outage cuts the city's signal.".into();
        } else {
            self.reason = format!(
                "{label}: {} neighborhoods resonated. +{} audience",
                resonant, gain
            );
        }
        if self.night > 14 {
            self.screen = Screen::End;
            self.reason = if self.trust >= 48 && self.audience >= 500 {
                "The city answers the final appeal. You kept the signal alive.".into()
            } else {
                "The final appeal fades into static. The city needed more trust and listeners."
                    .into()
            };
        }
        if self.power <= 0 || self.trust <= 0 || self.audience <= 0 {
            self.screen = Screen::End;
            self.reason = if self.power <= 0 {
                "The transmitter ran out of power.".into()
            } else if self.trust <= 0 {
                "The city stopped believing the station.".into()
            } else {
                "The audience drifted away.".into()
            };
        }
        self.best = self.best.max(self.score);
    }
    pub fn selftest() -> bool {
        let mut win = Game::new(73, 0);
        win.start();
        for _ in 0..14 {
            win.choose(2);
            win.broadcast();
            if win.screen == Screen::End {
                break;
            }
        }
        let mut loss = Game::new(991, 0);
        loss.start();
        for _ in 0..14 {
            loss.choose(3);
            loss.broadcast();
            if loss.screen == Screen::End {
                break;
            }
        }
        win.screen == Screen::End
            && win.reason.contains("kept the signal alive")
            && win.trust >= 48
            && win.audience >= 500
            && win.best > 0
            && win.events >= 10
            && loss.screen == Screen::End
    }
    pub fn save(&self, path: &Path) {
        let _ = fs::write(
            path,
            format!("{}\n{}\n{}", self.best, self.score, self.upgrades),
        );
    }
}
fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

pub mod gfx {
    use super::*;
    use font8x8::{BASIC_FONTS, UnicodeFonts};
    pub struct Canvas {
        pub p: Vec<u32>,
    }
    impl Canvas {
        pub fn new() -> Self {
            Self { p: vec![0; W * H] }
        }
        fn put(&mut self, x: i32, y: i32, c: u32) {
            if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
                self.p[y as usize * W + x as usize] = c;
            }
        }
        fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
            for yy in y.max(0)..(y + h).min(H as i32) {
                for xx in x.max(0)..(x + w).min(W as i32) {
                    self.put(xx, yy, c)
                }
            }
        }
        fn circle(&mut self, x: i32, y: i32, r: i32, c: u32) {
            for yy in -r..=r {
                for xx in -r..=r {
                    if xx * xx + yy * yy <= r * r {
                        self.put(x + xx, y + yy, c)
                    }
                }
            }
        }
        fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
            let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
            for i in 0..=n {
                self.put(x0 + (x1 - x0) * i / n, y0 + (y1 - y0) * i / n, c)
            }
        }
        pub fn text(&mut self, x: i32, y: i32, s: &str, c: u32, scale: i32) {
            let mut xx = x;
            for ch in s.chars() {
                if let Some(g) = BASIC_FONTS.get(ch) {
                    for (row, b) in g.iter().enumerate() {
                        for col in 0..8 {
                            if b & (1 << col) != 0 {
                                self.rect(xx + col * scale, y + row as i32 * scale, scale, scale, c)
                            }
                        }
                    }
                }
                xx += 8 * scale;
            }
        }
    }
    impl Default for Canvas {
        fn default() -> Self {
            Self::new()
        }
    }
    pub fn draw(g: &Game) -> Vec<u32> {
        let mut c = Canvas::new();
        c.p.fill(0xff111a27);
        for y in 0..H as i32 {
            let t = y as u32 * 14 / H as u32;
            c.rect(
                0,
                y,
                W as i32,
                1,
                0xff000000 | ((27 + t) << 16) | ((35 + t) << 8) | (47 + t),
            );
        }
        // distant city skyline
        for i in 0..18 {
            let x = i * 78;
            let h = 70 + (mix(i as u64 + g.seed) % 110) as i32;
            c.rect(x, 200 - h, 54, h, 0xff202d3a);
            for wy in (210 - h..190).step_by(19) {
                c.rect(x + 9, wy, 5, 7, 0xffb87949);
                c.rect(x + 27, wy, 5, 7, 0xffd7a967);
            }
        }
        c.rect(0, 205, W as i32, 2, 0xffe3a45b);
        if g.screen == Screen::Title {
            c.text(230, 72, "NIGHT SIGNAL", 0xffffd58a, 5);
            c.text(
                285,
                125,
                "A CITYWIDE BLACKOUT RADIO CAMPAIGN",
                0xffc4d0d5,
                2,
            );
            c.rect(286, 220, 708, 350, 0xff1b2937);
            c.text(
                340,
                260,
                "KEEP THE CITY TOGETHER, ONE BROADCAST AT A TIME",
                0xffffd58a,
                2,
            );
            c.text(360, 325, "CLICK OR ENTER TO START", 0xfff1e6c9, 3);
            c.text(
                350,
                402,
                "1-4 PROGRAM   |   CLICK DISTRICTS TO POWER RELAYS",
                0xffadc1c7,
                2,
            );
            c.text(
                350,
                448,
                "SPACE: BROADCAST    U: STATION UPGRADE    M: MUTE",
                0xffadc1c7,
                2,
            );
            c.text(
                360,
                520,
                "MATCH NEED + POWERED RELAYS TO RESONATE; ROUTE POWER TO TUNE EFFECTS.",
                0xffe59462,
                2,
            );
            return c.p;
        }
        c.rect(24, 20, 1232, 104, 0xff202e3c);
        c.text(
            48,
            36,
            "NIGHT SIGNAL   /   EMERGENCY BROADCAST",
            0xffffd58a,
            2,
        );
        let hud = format!(
            "NIGHT {:02}/14   POWER {}   AUDIENCE {}   TRUST {}   FUNDS {}   UPGRADES {}",
            g.night.min(14),
            g.power,
            g.audience,
            g.trust,
            g.funds,
            g.upgrades
        );
        c.text(48, 78, &hud, 0xffe8eee9, 2);
        c.text(1000, 36, &format!("RESONANCE x{}", g.streak), 0xfff0bd78, 1);
        // map panel and radial routes
        c.rect(28, 145, 812, 510, 0xff253947);
        c.text(
            54,
            158,
            "THE CITY  /  CLICK A DISTRICT TO TOGGLE ITS RELAY",
            0xffeac18b,
            1,
        );
        for (i, d) in g.districts.iter().enumerate() {
            let sx = d.x + 15;
            let sy = d.y + 10;
            c.line(
                420,
                420,
                sx,
                sy,
                if d.relay { 0xffe5a758 } else { 0xff465866 },
            );
            c.circle(sx, sy, 42, if d.relay { 0xffbd7843 } else { 0xff304654 });
            c.circle(sx, sy, 34, if d.relay { 0xffe7ae63 } else { 0xff405c68 });
            c.text(sx - 31, sy - 9, d.name, 0xfff3e7ce, 1);
            c.circle(
                sx + 25,
                sy - 26,
                7,
                match d.need {
                    1 => 0xffef9d68,
                    2 => 0xff88bed1,
                    3 => 0xffbd93dc,
                    _ => 0xffedcf72,
                },
            );
            c.text(
                sx - 27,
                sy + 18,
                &format!("{}% {}", d.trust, if d.relay { "ON" } else { "OFF" }),
                0xffd7e0d8,
                1,
            );
            let _ = i;
        }
        // right console
        c.rect(862, 145, 390, 510, 0xff202e3c);
        c.text(890, 167, "PROGRAM DESK", 0xffffd58a, 2);
        for (i, (name, desc, col)) in [
            ("1  VERIFIED BULLETIN", "Need: NEWS / trust", 0xffe8a86d),
            ("2  REQUEST HOUR", "Need: MUSIC / warmth", 0xff87bfd1),
            ("3  OPEN LINES", "Need: ADVICE / care", 0xffc39be0),
            ("4  UNVERIFIED TIP", "Need: RUMOR / reach", 0xffe9d176),
        ]
        .iter()
        .enumerate()
        {
            let y = 212 + i as i32 * 65;
            c.rect(
                886,
                y,
                338,
                50,
                if i == g.program as usize {
                    0xff3e5360
                } else {
                    0xff293b48
                },
            );
            c.text(900, y + 9, name, *col, 1);
            c.text(900, y + 27, desc, 0xffc4d0cf, 1);
        }
        c.text(890, 498, "STATION POWER ROUTING", 0xffffd58a, 1);
        for i in 0..5 {
            let name = ["TX", "STUDIO", "VERIFIER", "ARCHIVE", "TOWER"][i];
            let on = g.budget[i];
            c.rect(
                892 + (i as i32 % 3) * 108,
                520 + (i as i32 / 3) * 38,
                96,
                28,
                if on { 0xff557258 } else { 0xff34424b },
            );
            c.text(
                902 + (i as i32 % 3) * 108,
                529 + (i as i32 / 3) * 38,
                &format!("{} {}", name, if on { "ON" } else { "OFF" }),
                0xffeef0dd,
                1,
            );
        }
        c.rect(888, 598, 332, 42, 0xffa95f3e);
        c.text(918, 611, "BROADCAST NOW  [SPACE]", 0xfffff0d1, 2);
        c.text(46, 677, &g.reason, 0xfff0ba77, 1);
        if g.screen == Screen::End {
            c.rect(0, 0, W as i32, H as i32, 0xbb08121b);
            c.rect(250, 205, 780, 310, 0xff263744);
            c.text(
                380,
                260,
                if g.trust >= 48 && g.audience >= 500 {
                    "THE CITY ANSWERED"
                } else {
                    "SIGNAL LOST"
                },
                0xffffd58a,
                4,
            );
            let ending = if g.reason.len() > 65 {
                format!("{}...", &g.reason[..65])
            } else {
                g.reason.clone()
            };
            c.text(320, 355, &ending, 0xfff5e7d0, 1);
            c.text(
                375,
                440,
                &format!("CAMPAIGN SCORE {}     BEST {}", g.score, g.best),
                0xffe8b77e,
                2,
            );
            c.text(400, 500, "R  PLAY AGAIN   ESC  EXIT", 0xffbfced0, 2);
        }
        c.p
    }
}
