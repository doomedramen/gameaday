//! Screen flow and input handling. Engine-free: `main.rs` feeds `Input` and
//! presents `Canvas`; the selftest drives the same `App` with synthetic input.

use crate::audio::{Engine, Mix, Music, Sound, Ui};
use crate::campaign::{self, Campaign, GARAGE_SLOTS, ROUNDS, Verdict};
use crate::canvas::{Align, Canvas, H, W, mix};
use crate::car::Controls;
use crate::font::Face;
use crate::race::{CHAMPION, Driver, Kind, Mode, Phase, ROSTER, Race};
use crate::render::{
    self, C_ACCENT, C_CYAN, C_DIM, C_GOLD, C_GREEN, C_PANEL, C_RED, C_TEXT, RenderOpts, View,
    fmt_time, ordinal,
};
use crate::rng::Rng;
use crate::save::{Save, medal};
use crate::track::{BIOMES, Biome, Track, TrackSpec};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Drift,
    Nitro,
    Enter,
    Back,
    Pause,
    Mute,
    Help,
    Restart,
}

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub down: Vec<Key>,
    pub pressed: Vec<Key>,
}

impl Input {
    pub fn held(&self, k: Key) -> bool {
        self.down.contains(&k)
    }
    pub fn hit(&self, k: Key) -> bool {
        self.pressed.contains(&k)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Help,
    Settings,
    Intro,
    PreRace,
    Racing,
    Results,
    Standings,
    Draft,
    Garage,
    Route,
    Victory,
    GameOver,
    TtSelect,
    TtResults,
}

pub const CLASSIC_SEEDS: [u64; 5] = [0xC0FFEE, 0xD00D5, 0x7EE5, 0xB1AC, 0x6A11];
pub const CLASSIC_TWIST: [f32; 5] = [0.3, 0.2, 0.55, 0.45, 0.4];

pub fn classic_spec(i: usize) -> TrackSpec {
    TrackSpec {
        seed: CLASSIC_SEEDS[i],
        biome: BIOMES[i],
        twisty: CLASSIC_TWIST[i],
        hazard: 0.3,
        length: 12500.0,
    }
}

pub struct App {
    pub screen: Screen,
    pub save: Save,
    save_path: Option<PathBuf>,
    pub campaign: Option<Campaign>,
    pub race: Option<Race>,
    pub view: Option<View>,
    demo: Option<(Race, View)>,
    pub sel: usize,
    pub screen_t: f32,
    pub time: f32,
    pub paused: bool,
    pub pause_sel: usize,
    settings_return: Screen,
    sounds: Vec<Sound>,
    pub muted: bool,
    pub quit: bool,
    rng: Rng,
    pub last_result: Option<campaign::RoundResult>,
    pub last_order: Vec<(Driver, f32, u32)>,
    pub last_stats: Vec<(String, String)>,
    pub tt_track: usize,
    pub tt_new_record: bool,
    pub tt_medal: u8,
    pub tt_best: Option<f32>,
    pub draft_pick_t: f32,
    pub done_t: f32,
    pub help_page: usize,
    hints: bool,
}

impl App {
    pub fn new(save_path: Option<PathBuf>, seed: u64) -> App {
        let save = save_path
            .as_ref()
            .map(|p| Save::load(p))
            .unwrap_or_default();
        let mut app = App {
            screen: Screen::Title,
            save,
            save_path,
            campaign: None,
            race: None,
            view: None,
            demo: None,
            sel: 0,
            screen_t: 0.0,
            time: 0.0,
            paused: false,
            pause_sel: 0,
            settings_return: Screen::Title,
            sounds: Vec::new(),
            muted: false,
            quit: false,
            rng: Rng::new(seed),
            last_result: None,
            last_order: Vec::new(),
            last_stats: Vec::new(),
            tt_track: 0,
            tt_new_record: false,
            tt_medal: 0,
            tt_best: None,
            draft_pick_t: 0.0,
            done_t: 0.0,
            help_page: 0,
            hints: false,
        };
        app.start_demo();
        app
    }

    fn start_demo(&mut self) {
        let spec = TrackSpec {
            seed: self.rng.next_u64(),
            biome: Biome::Greenvale,
            twisty: 0.35,
            hazard: 0.2,
            length: 12000.0,
        };
        let mut rivals = ROSTER.to_vec();
        rivals.push(CHAMPION);
        let mut race = Race::new(
            Track::generate(spec),
            Mode::Championship,
            99,
            campaign::build_stats(&Default::default(), &[]),
            &rivals,
            0.97,
            self.rng.next_u64(),
        );
        race.autopilot = true;
        race.countdown = 0.0;
        let view = View::new(&race);
        self.demo = Some((race, view));
    }

    pub fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }

    fn persist(&mut self) {
        if let Some(p) = &self.save_path {
            let _ = self.save.store(p);
        }
    }

    fn goto(&mut self, s: Screen) {
        self.screen = s;
        self.sel = 0;
        self.screen_t = 0.0;
    }

    /// What the mixer should be doing this frame.
    pub fn mix(&self) -> Mix {
        let s = &self.save.settings;
        let mut m = Mix {
            music_vol: if self.muted {
                0.0
            } else {
                s.music as f32 / 10.0
            },
            sfx_vol: if self.muted { 0.0 } else { s.sfx as f32 / 10.0 },
            ..Mix::default()
        };
        match (&self.race, self.screen) {
            (Some(race), Screen::Racing) => {
                let me = race.player();
                let theme = race.track.spec.biome.index() as u8 + 1;
                let mut intensity = 0.4;
                if me.place <= 3 {
                    intensity += 0.2;
                }
                if me.car.speed() > 480.0 {
                    intensity += 0.15;
                }
                if race.phase == Phase::Countdown {
                    intensity = 0.15;
                }
                let final_lap =
                    me.lap == race.laps && me.finish.is_none() && race.phase == Phase::Running;
                if final_lap {
                    intensity = 1.0;
                }
                m.music = Music {
                    theme,
                    intensity,
                    final_lap,
                };
                if self.paused {
                    m.music_vol *= 0.4;
                    return m;
                }
                let p = me.car.pos;
                m.engines.push(Engine {
                    rpm: me.car.rpm,
                    load: me.controls.throttle.max(0.0),
                    vol: 0.2,
                    pan: 0.0,
                    boost: me.car.boosting(),
                    bike: false,
                });
                let mut others: Vec<&crate::race::Racer> =
                    race.racers.iter().filter(|r| !r.is_player()).collect();
                others.sort_by(|a, b| a.car.pos.dist(p).partial_cmp(&b.car.pos.dist(p)).unwrap());
                for r in others.iter().take(3) {
                    let d = r.car.pos - p;
                    let v = (1.0 - d.len() / 800.0).clamp(0.0, 1.0) * 0.1;
                    m.engines.push(Engine {
                        rpm: r.car.rpm,
                        load: r.controls.throttle.max(0.0),
                        vol: v,
                        pan: (d.x / 600.0).clamp(-1.0, 1.0),
                        boost: r.car.boosting(),
                        bike: r.driver.kind.is_bike(),
                    });
                }
                let sk = if me.car.drifting || (me.car.slip > 0.2 && me.car.speed() > 150.0) {
                    1.0
                } else {
                    0.0
                };
                m.skid = sk;
                m.wind =
                    (me.car.draft * 2.0).min(1.0) * 0.6 + if me.car.nitro_on { 0.4 } else { 0.0 };
            }
            _ => {
                let theme = match self.screen {
                    Screen::Title
                    | Screen::Help
                    | Screen::Settings
                    | Screen::TtSelect
                    | Screen::Intro => 0,
                    _ => 6,
                };
                m.music = Music {
                    theme,
                    intensity: 0.3,
                    final_lap: false,
                };
            }
        }
        m
    }

    // ---- update -------------------------------------------------------------

    pub fn update(&mut self, dt: f32, input: &Input) {
        self.time += dt;
        self.screen_t += dt;
        if input.hit(Key::Mute) {
            self.muted = !self.muted;
        }
        if let Some((race, view)) = &mut self.demo {
            if matches!(
                self.screen,
                Screen::Title | Screen::Help | Screen::Settings | Screen::TtSelect | Screen::Intro
            ) && self.race.is_none()
            {
                race.update(dt, Controls::default());
                race.sounds.clear();
                race.popups.clear();
                view.update(race, dt);
            }
        }
        match self.screen {
            Screen::Title => self.update_title(input),
            Screen::Help => {
                if input.hit(Key::Left) || input.hit(Key::Right) {
                    self.help_page = 1 - self.help_page;
                    self.sounds.push(Sound::Ui(Ui::Move));
                }
                if input.hit(Key::Back) || input.hit(Key::Enter) || input.hit(Key::Help) {
                    self.sounds.push(Sound::Ui(Ui::Back));
                    self.goto(Screen::Title);
                }
            }
            Screen::Settings => self.update_settings(input),
            Screen::Intro => {
                if input.hit(Key::Enter) {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    self.enter_prerace();
                } else if input.hit(Key::Back) {
                    self.sounds.push(Sound::Ui(Ui::Back));
                    self.campaign = None;
                    self.goto(Screen::Title);
                }
            }
            Screen::PreRace => {
                if input.hit(Key::Enter) {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    self.start_race();
                } else if input.hit(Key::Back) {
                    self.sounds.push(Sound::Ui(Ui::Back));
                    self.race = None;
                    self.view = None;
                    self.campaign = None;
                    self.goto(Screen::Title);
                }
            }
            Screen::Racing => self.update_race(dt, input),
            Screen::Results => {
                if input.hit(Key::Enter) && self.screen_t > 0.4 {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    self.after_results();
                }
            }
            Screen::Standings => {
                if input.hit(Key::Enter) && self.screen_t > 0.3 {
                    self.sounds.push(Sound::Ui(Ui::Reveal));
                    if let Some(c) = &mut self.campaign {
                        c.roll_draft();
                    }
                    self.goto(Screen::Draft);
                    self.sel = 1;
                }
            }
            Screen::Draft => self.update_draft(input),
            Screen::Garage => self.update_garage(input),
            Screen::Route => {
                self.menu_nav(input, 2, true);
                if input.hit(Key::Enter) {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    let sel = self.sel;
                    if let Some(c) = &mut self.campaign {
                        c.choose_route(sel);
                    }
                    self.enter_prerace();
                }
            }
            Screen::Victory | Screen::GameOver => {
                if input.hit(Key::Enter) && self.screen_t > 1.0 {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    self.campaign = None;
                    self.race = None;
                    self.view = None;
                    self.goto(Screen::Title);
                }
            }
            Screen::TtSelect => {
                self.menu_nav(input, 5, false);
                if input.hit(Key::Enter) {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    self.tt_track = self.sel;
                    self.start_tt();
                } else if input.hit(Key::Back) {
                    self.sounds.push(Sound::Ui(Ui::Back));
                    self.goto(Screen::Title);
                    self.sel = 1;
                }
            }
            Screen::TtResults => {
                if input.hit(Key::Enter) || input.hit(Key::Restart) {
                    self.sounds.push(Sound::Ui(Ui::Select));
                    self.start_tt();
                } else if input.hit(Key::Back) {
                    self.sounds.push(Sound::Ui(Ui::Back));
                    self.race = None;
                    self.view = None;
                    let t = self.tt_track;
                    self.goto(Screen::TtSelect);
                    self.sel = t;
                }
            }
        }
    }

    fn menu_nav(&mut self, input: &Input, n: usize, horizontal: bool) {
        let (prev, next) = if horizontal {
            (Key::Left, Key::Right)
        } else {
            (Key::Up, Key::Down)
        };
        if input.hit(prev) && n > 0 {
            self.sel = (self.sel + n - 1) % n;
            self.sounds.push(Sound::Ui(Ui::Move));
        }
        if input.hit(next) && n > 0 {
            self.sel = (self.sel + 1) % n;
            self.sounds.push(Sound::Ui(Ui::Move));
        }
    }

    fn update_title(&mut self, input: &Input) {
        self.menu_nav(input, 5, false);
        if input.hit(Key::Help) {
            self.goto(Screen::Help);
            return;
        }
        if input.hit(Key::Enter) {
            self.sounds.push(Sound::Ui(Ui::Select));
            match self.sel {
                0 => {
                    let seed = self.rng.next_u64();
                    self.campaign = Some(Campaign::new(seed));
                    self.save.cups_played += 1;
                    self.persist();
                    self.goto(Screen::Intro);
                }
                1 => {
                    self.goto(Screen::TtSelect);
                    self.compute_pars();
                }
                2 => {
                    self.help_page = 0;
                    self.goto(Screen::Help);
                }
                3 => {
                    self.settings_return = Screen::Title;
                    self.goto(Screen::Settings);
                }
                _ => self.quit = true,
            }
        }
    }

    fn update_settings(&mut self, input: &Input) {
        self.menu_nav(input, 7, false);
        let d: i32 = if input.hit(Key::Left) {
            -1
        } else if input.hit(Key::Right) || input.hit(Key::Enter) {
            1
        } else {
            0
        };
        if d != 0 {
            let s = &mut self.save.settings;
            match self.sel {
                0 => s.steer_assist = !s.steer_assist,
                1 => s.auto_throttle = !s.auto_throttle,
                2 => {
                    let opts = [70, 85, 100];
                    let i = opts.iter().position(|x| *x == s.game_speed).unwrap_or(2) as i32;
                    s.game_speed = opts[(i + d).rem_euclid(3) as usize];
                }
                3 => s.shake = !s.shake,
                4 => s.music = (s.music as i32 + d).clamp(0, 10) as u32,
                5 => s.sfx = (s.sfx as i32 + d).clamp(0, 10) as u32,
                _ => {
                    if input.hit(Key::Enter) {
                        self.persist();
                        self.sounds.push(Sound::Ui(Ui::Back));
                        let r = self.settings_return;
                        self.goto(r);
                        return;
                    }
                }
            }
            if self.sel < 6 {
                self.sounds.push(Sound::Ui(Ui::Move));
                self.persist();
                if let Some(r) = &mut self.race {
                    r.assists.steer = self.save.settings.steer_assist;
                    r.assists.auto_throttle = self.save.settings.auto_throttle;
                }
            }
        }
        if input.hit(Key::Back) {
            self.persist();
            self.sounds.push(Sound::Ui(Ui::Back));
            let r = self.settings_return;
            self.goto(r);
        }
    }

    fn enter_prerace(&mut self) {
        let Some(c) = &self.campaign else { return };
        let track = Track::generate(c.venue.spec);
        let seed = self.rng.next_u64();
        let mut race = Race::new(
            track,
            Mode::Championship,
            c.venue.laps,
            c.stats(),
            &c.rivals(),
            campaign::skill(c.round),
            seed,
        );
        race.assists.steer = self.save.settings.steer_assist;
        race.assists.auto_throttle = self.save.settings.auto_throttle;
        self.hints = c.round == 0 && !self.save.tutorial_done;
        let mut view = View::new(&race);
        view.shake_on = self.save.settings.shake;
        self.race = Some(race);
        self.view = Some(view);
        self.goto(Screen::PreRace);
    }

    fn start_race(&mut self) {
        self.paused = false;
        self.done_t = 0.0;
        if let (Some(race), Some(view)) = (&mut self.race, &mut self.view) {
            view.snap(race);
        }
        self.goto(Screen::Racing);
    }

    fn start_tt(&mut self) {
        let i = self.tt_track;
        let track = Track::generate(classic_spec(i));
        let mut race = Race::new(
            track,
            Mode::TimeTrial,
            3,
            campaign::build_stats(&Default::default(), &[]),
            &[],
            1.0,
            1,
        );
        race.assists.steer = self.save.settings.steer_assist;
        race.assists.auto_throttle = self.save.settings.auto_throttle;
        let rec = &self.save.records[i];
        if !rec.ghost.is_empty() {
            race.ghost = Some(rec.ghost.clone());
        }
        let mut view = View::new(&race);
        view.shake_on = self.save.settings.shake;
        view.snap(&race);
        self.race = Some(race);
        self.view = Some(view);
        self.hints = false;
        self.paused = false;
        self.done_t = 0.0;
        self.goto(Screen::Racing);
    }

    /// Calibrate medal times: the autopilot's best lap on each classic.
    pub fn compute_pars(&mut self) {
        for i in 0..5 {
            if self.save.records[i].par.is_none() {
                self.save.records[i].par = Some(autopilot_lap(classic_spec(i)));
            }
        }
        self.persist();
    }

    fn update_race(&mut self, dt: f32, input: &Input) {
        if input.hit(Key::Pause) || input.hit(Key::Back) {
            if !self.paused {
                self.paused = true;
                self.pause_sel = 0;
                self.sounds.push(Sound::Ui(Ui::Back));
                return;
            } else if input.hit(Key::Back) || input.hit(Key::Pause) {
                self.paused = false;
                return;
            }
        }
        if self.paused {
            let n = 4;
            if input.hit(Key::Up) {
                self.pause_sel = (self.pause_sel + n - 1) % n;
                self.sounds.push(Sound::Ui(Ui::Move));
            }
            if input.hit(Key::Down) {
                self.pause_sel = (self.pause_sel + 1) % n;
                self.sounds.push(Sound::Ui(Ui::Move));
            }
            if input.hit(Key::Enter) {
                self.sounds.push(Sound::Ui(Ui::Select));
                match self.pause_sel {
                    0 => self.paused = false,
                    1 => {
                        // restart: free in Time Trial; in the Cup it restarts
                        // the round from the grid (same track)
                        if self
                            .race
                            .as_ref()
                            .is_some_and(|r| r.mode == Mode::TimeTrial)
                        {
                            self.start_tt();
                        } else {
                            self.restart_round();
                        }
                    }
                    2 => {
                        self.settings_return = Screen::Racing;
                        self.goto(Screen::Settings);
                    }
                    _ => {
                        self.race = None;
                        self.view = None;
                        self.campaign = None;
                        self.paused = false;
                        self.goto(Screen::Title);
                    }
                }
            }
            return;
        }
        let Some(race) = &mut self.race else { return };
        let throttle = if input.held(Key::Up) {
            1.0
        } else if input.held(Key::Down) {
            -1.0
        } else {
            0.0
        };
        let steer = (input.held(Key::Right) as i32 - input.held(Key::Left) as i32) as f32;
        let c = Controls {
            throttle,
            steer,
            drift: input.held(Key::Drift),
            nitro: input.held(Key::Nitro),
        };
        let was_nitro = race.player().car.nitro_on;
        let speed = self.save.settings.game_speed as f32 / 100.0;
        race.update(dt * speed, c);
        if !was_nitro && race.player().car.nitro_on {
            self.sounds
                .push(Sound::Race(crate::race::Sfx::NitroStart, 1.0, 0.0));
        }
        for s in race.sounds.drain(..) {
            self.sounds.push(Sound::Race(s.sfx, s.vol, s.pan));
        }
        if let Some(v) = &mut self.view {
            v.update(race, dt);
        }
        if race.mode == Mode::TimeTrial && input.hit(Key::Restart) {
            self.start_tt();
            return;
        }
        if race.phase == Phase::Done {
            self.done_t += dt;
            if self.done_t > 1.2 || input.hit(Key::Enter) {
                self.finish_race();
            }
        } else if race.phase == Phase::Cooldown && input.hit(Key::Enter) {
            // skip the cool-down lap
            race.autopilot = true;
            for _ in 0..2400 {
                race.update(1.0 / 60.0, Controls::default());
                race.sounds.clear();
                race.skids.clear();
                if race.phase == Phase::Done {
                    break;
                }
            }
            race.particles.clear();
        }
        if let Some(race) = &mut self.race {
            if race.phase == Phase::Cooldown {
                race.autopilot = true;
            }
        }
    }

    fn restart_round(&mut self) {
        let Some(c) = &self.campaign else { return };
        let spec = c.venue.spec;
        let track = Track::generate(spec);
        let seed = self.rng.next_u64();
        let mut race = Race::new(
            track,
            Mode::Championship,
            c.venue.laps,
            c.stats(),
            &c.rivals(),
            campaign::skill(c.round),
            seed,
        );
        race.assists.steer = self.save.settings.steer_assist;
        race.assists.auto_throttle = self.save.settings.auto_throttle;
        let mut view = View::new(&race);
        view.shake_on = self.save.settings.shake;
        self.race = Some(race);
        self.view = Some(view);
        self.start_race();
    }

    fn finish_race(&mut self) {
        let Some(race) = &self.race else { return };
        if race.mode == Mode::TimeTrial {
            let best = race.best_lap;
            let i = self.tt_track;
            let rec = &mut self.save.records[i];
            self.tt_new_record = false;
            if let Some(b) = best {
                if rec.best.is_none_or(|x| b < x) {
                    rec.best = Some(b);
                    rec.ghost = race.best_ghost.clone().unwrap_or_default();
                    self.tt_new_record = true;
                }
            }
            self.tt_best = best;
            self.tt_medal = match (best, rec.par) {
                (Some(b), Some(p)) => medal(b, p),
                _ => 0,
            };
            if self.tt_medal > 0 {
                self.sounds.push(Sound::Ui(Ui::Medal));
            }
            self.persist();
            self.goto(Screen::TtResults);
            return;
        }
        let order: Vec<(Driver, f32, u32)> = race
            .classification()
            .iter()
            .map(|&i| {
                (
                    race.racers[i].driver,
                    race.racers[i].finish.unwrap_or(0.0),
                    0,
                )
            })
            .collect();
        let style = race.style.total;
        let st = &race.style;
        self.last_stats = vec![
            (
                "Best lap".into(),
                race.best_lap.map(fmt_time).unwrap_or("-".into()),
            ),
            (
                "Drifts / slingshots".into(),
                format!("{} / {}", st.drifts, st.slingshots),
            ),
            (
                "Overtakes / close calls".into(),
                format!("{} / {}", st.overtakes, st.near_misses),
            ),
            ("Longest chain".into(), format!("{}", st.best_chain)),
        ];
        let drivers: Vec<Driver> = order.iter().map(|o| o.0).collect();
        let Some(c) = &mut self.campaign else { return };
        let res = c.finish_round(&drivers, style);
        self.last_order = order
            .iter()
            .enumerate()
            .map(|(k, o)| (o.0, o.1, campaign::POINTS[k.min(7)]))
            .collect();
        if c.round == 0 {
            self.save.tutorial_done = true;
        }
        self.save.best_round = self.save.best_round.max(c.round as u32 + 1);
        match res.verdict {
            Verdict::Champion => {
                self.save.cups_won += 1;
                self.sounds.push(Sound::Ui(Ui::Fanfare));
            }
            Verdict::Eliminated => self.sounds.push(Sound::Ui(Ui::Defeat)),
            Verdict::Qualified => self.sounds.push(Sound::Ui(Ui::Medal)),
            Verdict::SecondChance => self.sounds.push(Sound::Ui(Ui::Error)),
        }
        if matches!(res.verdict, Verdict::Champion | Verdict::Eliminated) {
            let score = c.score();
            self.save.best_score = self.save.best_score.max(score);
        }
        self.persist();
        self.last_result = Some(res);
        self.goto(Screen::Results);
    }

    fn after_results(&mut self) {
        let Some(res) = self.last_result.clone() else {
            return;
        };
        match res.verdict {
            Verdict::Champion => self.goto(Screen::Victory),
            Verdict::Eliminated => self.goto(Screen::GameOver),
            Verdict::SecondChance => self.enter_prerace(),
            Verdict::Qualified => self.goto(Screen::Standings),
        }
    }

    fn update_draft(&mut self, input: &Input) {
        let n = self.campaign.as_ref().map(|c| c.draft.len()).unwrap_or(0);
        if n == 0 {
            self.goto(Screen::Garage);
            return;
        }
        if self.draft_pick_t > 0.0 {
            self.draft_pick_t += 0.016;
            if self.draft_pick_t > 0.5 {
                self.draft_pick_t = 0.0;
                let sel = self.sel;
                if let Some(c) = &mut self.campaign {
                    c.take_part(sel);
                }
                self.goto(Screen::Garage);
            }
            return;
        }
        self.menu_nav(input, n, true);
        if input.hit(Key::Enter) && self.screen_t > 0.5 {
            self.sounds.push(Sound::Ui(Ui::Buy));
            self.draft_pick_t = 0.01;
        }
    }

    fn update_garage(&mut self, input: &Input) {
        // rows: 4 upgrades, undo, continue
        self.menu_nav(input, 6, false);
        if input.hit(Key::Enter) {
            let sel = self.sel;
            let Some(c) = &mut self.campaign else { return };
            if sel < 4 {
                if c.buy(sel) {
                    self.sounds.push(Sound::Ui(Ui::Buy));
                } else {
                    self.sounds.push(Sound::Ui(Ui::Error));
                }
            } else if sel == 4 {
                if c.undo() {
                    self.sounds.push(Sound::Ui(Ui::Back));
                } else {
                    self.sounds.push(Sound::Ui(Ui::Error));
                }
            } else {
                self.sounds.push(Sound::Ui(Ui::Select));
                c.next_round();
                if c.is_final() {
                    self.enter_prerace();
                } else {
                    self.goto(Screen::Route);
                }
            }
        } else if input.hit(Key::Back) {
            let undone = self.campaign.as_mut().is_some_and(|c| c.undo());
            self.sounds
                .push(Sound::Ui(if undone { Ui::Back } else { Ui::Error }));
        }
    }

    // ---- draw ---------------------------------------------------------------

    pub fn draw(&mut self, cv: &mut Canvas) {
        match self.screen {
            Screen::Racing => {
                if let (Some(race), Some(view)) = (&self.race, &mut self.view) {
                    render::draw_race(
                        cv,
                        race,
                        view,
                        &RenderOpts {
                            hints: self.hints,
                            paused: self.paused,
                        },
                    );
                }
                if self.paused {
                    self.draw_pause(cv);
                }
            }
            Screen::Settings if self.settings_return == Screen::Racing => {
                if let (Some(race), Some(view)) = (&self.race, &mut self.view) {
                    render::draw_race(
                        cv,
                        race,
                        view,
                        &RenderOpts {
                            hints: false,
                            paused: true,
                        },
                    );
                }
                cv.darken(0.55);
                self.draw_settings(cv);
            }
            _ => {
                self.draw_backdrop(cv);
                match self.screen {
                    Screen::Title => self.draw_title(cv),
                    Screen::Help => self.draw_help(cv),
                    Screen::Settings => self.draw_settings(cv),
                    Screen::Intro => self.draw_intro(cv),
                    Screen::PreRace => self.draw_prerace(cv),
                    Screen::Results => self.draw_results(cv),
                    Screen::Standings => self.draw_standings(cv),
                    Screen::Draft => self.draw_draft(cv),
                    Screen::Garage => self.draw_garage(cv),
                    Screen::Route => self.draw_route(cv),
                    Screen::Victory => self.draw_end(cv, true),
                    Screen::GameOver => self.draw_end(cv, false),
                    Screen::TtSelect => self.draw_tt_select(cv),
                    Screen::TtResults => self.draw_tt_results(cv),
                    Screen::Racing => {}
                }
            }
        }
    }

    fn draw_backdrop(&mut self, cv: &mut Canvas) {
        let use_race = !matches!(
            self.screen,
            Screen::Title | Screen::Help | Screen::Settings | Screen::TtSelect | Screen::Intro
        );
        if use_race {
            if let (Some(race), Some(view)) = (&self.race, &mut self.view) {
                render::draw_scene_only(cv, race, view);
                cv.darken(0.55);
                return;
            }
        }
        if let Some((race, view)) = &mut self.demo {
            render::draw_scene_only(cv, race, view);
            cv.darken(0.45);
        } else {
            cv.clear(0x1B2233);
        }
    }

    fn header(&self, cv: &mut Canvas, title: &str, sub: &str) {
        cv.text_outline(
            title,
            W as f32 / 2.0,
            34.0,
            46.0,
            Face::Bold,
            C_TEXT,
            0x141925,
            1.0,
            Align::Center,
        );
        if !sub.is_empty() {
            cv.text_sh(
                sub,
                W as f32 / 2.0,
                92.0,
                20.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Center,
            );
        }
    }

    fn footer(&self, cv: &mut Canvas, txt: &str) {
        cv.text_sh(
            txt,
            W as f32 / 2.0,
            H as f32 - 40.0,
            18.0,
            Face::Body,
            C_DIM,
            1.0,
            Align::Center,
        );
    }

    fn draw_title(&self, cv: &mut Canvas) {
        let t = self.time;
        // logo
        let y = 92.0 + (t * 1.6).sin() * 4.0;
        cv.text_outline(
            "SLIPSTREAM",
            W as f32 / 2.0,
            y,
            92.0,
            Face::Bold,
            C_TEXT,
            0x141925,
            1.0,
            Align::Center,
        );
        cv.text_outline(
            "CUP",
            W as f32 / 2.0,
            y + 92.0,
            64.0,
            Face::Bold,
            C_ACCENT,
            0x141925,
            1.0,
            Align::Center,
        );
        let items = [
            "Championship",
            "Time Trial",
            "How to Play",
            "Settings",
            "Quit",
        ];
        let (x, y0) = (W as f32 / 2.0, 330.0);
        for (i, it) in items.iter().enumerate() {
            let y = y0 + i as f32 * 54.0;
            let sel = i == self.sel;
            let w = 360.0;
            cv.round_rect(
                x - w / 2.0,
                y,
                w,
                44.0,
                14.0,
                if sel { C_ACCENT } else { C_PANEL },
                if sel { 0.95 } else { 0.78 },
            );
            cv.text(
                it,
                x,
                y + 9.0,
                24.0,
                Face::Bold,
                if sel { 0x1B1F2A } else { C_TEXT },
                1.0,
                Align::Center,
            );
        }
        let s = &self.save;
        let line = format!(
            "Best Cup score {}    Cups won {}    Best round reached {}/{}",
            s.best_score, s.cups_won, s.best_round, ROUNDS
        );
        cv.text_sh(
            &line,
            W as f32 / 2.0,
            H as f32 - 64.0,
            17.0,
            Face::Body,
            C_DIM,
            1.0,
            Align::Center,
        );
        self.footer(
            cv,
            "UP/DOWN choose   ENTER select   H help   M mute   F fullscreen",
        );
    }

    fn draw_help(&self, cv: &mut Canvas) {
        self.header(
            cv,
            "HOW TO PLAY",
            if self.help_page == 0 {
                "Driving  (LEFT/RIGHT for page 2)"
            } else {
                "The Cup  (LEFT/RIGHT for page 1)"
            },
        );
        let x = 200.0;
        let w = W as f32 - 400.0;
        cv.round_rect(x, 130.0, w, 500.0, 18.0, C_PANEL, 0.88);
        let lines: &[(&str, &str)] = if self.help_page == 0 {
            &[
                ("UP / W", "Accelerate"),
                ("DOWN / S", "Brake, then reverse"),
                ("LEFT RIGHT / A D", "Steer"),
                (
                    "SPACE (hold)",
                    "Drift. Steer into a bend and hold: sparks go blue, orange, purple. Release for a mini-turbo.",
                ),
                ("SHIFT (hold)", "Fire NITRO from the meter"),
                (
                    "SLIPSTREAM",
                    "Sit right behind any car: it tows you and fills NITRO. Pull out and pass for a SLINGSHOT.",
                ),
                (
                    "ROCKET START",
                    "Press UP just as the last light goes out. Too early and you stall.",
                ),
                (
                    "PACE NOTES",
                    "The top panel calls the next bend: green fast, yellow tight, red hairpin.",
                ),
                (
                    "PICKUPS",
                    "Blue cans: nitro. Green wrenches: repair. Yellow arrows: boost pads.",
                ),
                (
                    "HAZARDS",
                    "Oil spins you, mud and sand drag you. Off the road is slow.",
                ),
                (
                    "ESC / P",
                    "Pause. Assists (steering help, auto-throttle, game speed) are in Settings.",
                ),
            ]
        } else {
            &[
                (
                    "FIVE ROUNDS",
                    "Four venues you choose between, then the Grand Prix against the Champion.",
                ),
                (
                    "QUALIFY",
                    "Finish in the target place (top 5, 4, 3, 3) to go on. Win the Grand Prix to take the Cup.",
                ),
                (
                    "SECOND CHANCES",
                    "Two per Cup. Missing a target spends one and you retry the round.",
                ),
                (
                    "STYLE",
                    "Drifts, slingshots, close calls and cone smashes chain into a multiplier. Style pays cash.",
                ),
                (
                    "PARTS",
                    "After each round pick one of three car parts that change how you drive.",
                ),
                (
                    "GARAGE",
                    "Spend prize money on Engine, Tyres, Armour, Nitro. Changed your mind? Undo before you leave.",
                ),
                (
                    "RIVALS",
                    "Rocket, Drifter, Bully (flashes red then rams), Blocker, Wasp, Ace - and the Champion.",
                ),
                (
                    "DAMAGE",
                    "Hits cost top speed. It lasts the whole race, so drive clean or grab wrenches.",
                ),
                (
                    "TIME TRIAL",
                    "Five classic circuits. Your best lap races you as a ghost. Medals vs the AI's lap.",
                ),
                ("R", "Restart a Time Trial instantly."),
            ]
        };
        for (i, (k, v)) in lines.iter().enumerate() {
            let y = 150.0 + i as f32 * 43.0;
            cv.text(k, x + 24.0, y, 18.0, Face::Bold, C_ACCENT, 1.0, Align::Left);
            cv.text(
                v,
                x + 250.0,
                y + 1.0,
                17.0,
                Face::Body,
                C_TEXT,
                1.0,
                Align::Left,
            );
        }
        self.footer(cv, "ENTER or ESC to go back");
    }

    fn draw_settings(&self, cv: &mut Canvas) {
        self.header(
            cv,
            "SETTINGS",
            "Assists make the game easier without changing the rules",
        );
        let s = &self.save.settings;
        let onoff = |b: bool| if b { "ON" } else { "OFF" };
        let rows = [
            (
                "Steering assist",
                onoff(s.steer_assist).to_string(),
                "Gently steers you toward the racing line",
            ),
            (
                "Auto-throttle",
                onoff(s.auto_throttle).to_string(),
                "Always accelerating unless you brake",
            ),
            (
                "Game speed",
                format!("{}%", s.game_speed),
                "Slow the whole race down",
            ),
            ("Screen shake", onoff(s.shake).to_string(), ""),
            ("Music volume", format!("{}", s.music), ""),
            ("Effects volume", format!("{}", s.sfx), ""),
            ("Done", String::new(), ""),
        ];
        let x = W as f32 / 2.0 - 330.0;
        for (i, (name, val, desc)) in rows.iter().enumerate() {
            let y = 140.0 + i as f32 * 66.0;
            let sel = i == self.sel;
            cv.round_rect(
                x,
                y,
                660.0,
                56.0,
                14.0,
                if sel { C_ACCENT } else { C_PANEL },
                if sel { 0.95 } else { 0.82 },
            );
            let fg = if sel { 0x1B1F2A } else { C_TEXT };
            cv.text(
                name,
                x + 24.0,
                y + 8.0,
                22.0,
                Face::Bold,
                fg,
                1.0,
                Align::Left,
            );
            if !desc.is_empty() {
                cv.text(
                    desc,
                    x + 24.0,
                    y + 34.0,
                    14.0,
                    Face::Body,
                    if sel { 0x3A2A1A } else { C_DIM },
                    1.0,
                    Align::Left,
                );
            }
            if !val.is_empty() {
                cv.text(
                    &format!("< {val} >"),
                    x + 636.0,
                    y + 14.0,
                    22.0,
                    Face::Bold,
                    fg,
                    1.0,
                    Align::Right,
                );
            }
        }
        self.footer(cv, "UP/DOWN choose   LEFT/RIGHT change   ESC back");
    }

    fn draw_intro(&self, cv: &mut Canvas) {
        self.header(
            cv,
            "THE FIELD",
            "Six rivals stand between you and the Grand Prix. The Champion waits there.",
        );
        let mut drivers: Vec<Driver> = ROSTER.to_vec();
        drivers.push(CHAMPION);
        for (i, d) in drivers.iter().enumerate() {
            let col = i % 4;
            let row = i / 4;
            let x = 70.0 + col as f32 * 290.0 + if row == 1 { 145.0 } else { 0.0 };
            let y = 140.0 + row as f32 * 250.0;
            let champ = d.kind == Kind::Champion;
            cv.round_rect(x, y, 270.0, 230.0, 18.0, C_PANEL, 0.9);
            if champ {
                cv.round_rect_line(x, y, 270.0, 230.0, 18.0, 3.0, C_GOLD, 0.9);
            }
            let bob = (self.time * 2.0 + i as f32).sin() * 3.0;
            render::car_portrait(cv, d.sprite, x + 60.0, y + 95.0 + bob, 0.9, 0.0);
            cv.text(
                d.name,
                x + 120.0,
                y + 26.0,
                28.0,
                Face::Bold,
                if champ { C_GOLD } else { C_TEXT },
                1.0,
                Align::Left,
            );
            cv.round_rect(x + 120.0, y + 66.0, 120.0, 24.0, 12.0, d.color, 0.9);
            cv.text(
                d.kind.label(),
                x + 180.0,
                y + 69.0,
                16.0,
                Face::Bold,
                0x141925,
                1.0,
                Align::Center,
            );
            wrap(cv, d.kind.blurb(), x + 18.0, y + 162.0, 236.0, 16.0, C_DIM);
        }
        self.footer(cv, "ENTER to the first round   ESC back");
    }

    fn draw_prerace(&self, cv: &mut Canvas) {
        let (Some(c), Some(race)) = (&self.campaign, &self.race) else {
            return;
        };
        let v = &c.venue;
        let round = c.round + 1;
        let title = if c.is_final() {
            "THE GRAND PRIX".to_string()
        } else {
            format!("ROUND {round} OF {ROUNDS}")
        };
        self.header(
            cv,
            &title,
            &format!("{} - {}", v.spec.biome.name(), v.label),
        );
        // track preview
        let (px, py, pw, ph) = (80.0, 140.0, 520.0, 420.0);
        cv.round_rect(px, py, pw, ph, 18.0, C_PANEL, 0.9);
        draw_track_preview(cv, &race.track, px + 20.0, py + 20.0, pw - 40.0, ph - 40.0);
        // info
        let x = 640.0;
        cv.round_rect(x, py, 560.0, ph, 18.0, C_PANEL, 0.9);
        let need = campaign::qualify_place(c.round);
        let goal = if c.is_final() {
            "WIN to take the Cup".to_string()
        } else {
            format!("Finish top {need} to qualify")
        };
        cv.text(
            &goal,
            x + 28.0,
            py + 24.0,
            28.0,
            Face::Bold,
            C_ACCENT,
            1.0,
            Align::Left,
        );
        wrap(
            cv,
            v.spec.biome.blurb(),
            x + 28.0,
            py + 74.0,
            500.0,
            18.0,
            C_TEXT,
        );
        cv.text(
            &v.traits(),
            x + 28.0,
            py + 128.0,
            18.0,
            Face::Body,
            C_DIM,
            1.0,
            Align::Left,
        );
        cv.text(
            &format!(
                "Prize x{:.2}    Second chances {}    Cash ${}",
                v.prize_mult, c.chances, c.cash
            ),
            x + 28.0,
            py + 158.0,
            18.0,
            Face::Body,
            C_DIM,
            1.0,
            Align::Left,
        );
        cv.text(
            "YOUR CAR",
            x + 28.0,
            py + 204.0,
            18.0,
            Face::Bold,
            C_TEXT,
            1.0,
            Align::Left,
        );
        for (k, (name, _)) in GARAGE_SLOTS.iter().enumerate() {
            let lvl = c.garage.levels[k];
            let yy = py + 234.0 + k as f32 * 26.0;
            cv.text(
                name,
                x + 28.0,
                yy,
                16.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Left,
            );
            for p in 0..3 {
                cv.round_rect(
                    x + 140.0 + p as f32 * 26.0,
                    yy + 2.0,
                    20.0,
                    14.0,
                    4.0,
                    if p < lvl { C_CYAN } else { 0x3A4458 },
                    1.0,
                );
            }
        }
        let mut yy = py + 234.0;
        cv.text(
            "PARTS",
            x + 280.0,
            py + 204.0,
            18.0,
            Face::Bold,
            C_TEXT,
            1.0,
            Align::Left,
        );
        if c.parts.is_empty() {
            cv.text(
                "none yet",
                x + 280.0,
                yy,
                16.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Left,
            );
        }
        for p in &c.parts {
            cv.circle(x + 288.0, yy + 9.0, 5.0, p.color(), 1.0);
            cv.text(
                p.name(),
                x + 300.0,
                yy,
                16.0,
                Face::Body,
                C_TEXT,
                1.0,
                Align::Left,
            );
            yy += 24.0;
        }
        if c.is_final() {
            cv.round_rect(x + 28.0, py + 350.0, 500.0, 50.0, 12.0, 0x3A2F12, 0.9);
            cv.text(
                "Sable, the reigning Champion, joins the grid.",
                x + 44.0,
                py + 364.0,
                18.0,
                Face::Bold,
                C_GOLD,
                1.0,
                Align::Left,
            );
        }
        self.footer(cv, "ENTER to the grid   ESC quit the Cup");
    }

    fn draw_results(&self, cv: &mut Canvas) {
        let Some(res) = &self.last_result else { return };
        let Some(c) = &self.campaign else { return };
        self.header(cv, "RESULTS", res.biome.name());
        let x = 160.0;
        cv.round_rect(x, 130.0, 560.0, 470.0, 18.0, C_PANEL, 0.9);
        let lead = self.last_order.first().map(|o| o.1).unwrap_or(0.0);
        for (k, (d, t, pts)) in self.last_order.iter().enumerate() {
            let y = 150.0 + k as f32 * 52.0;
            let me = d.kind == Kind::Player;
            let reveal = ((self.screen_t - k as f32 * 0.08) * 4.0).clamp(0.0, 1.0);
            if me {
                cv.round_rect(
                    x + 10.0,
                    y - 4.0,
                    540.0,
                    46.0,
                    10.0,
                    C_ACCENT,
                    0.35 * reveal,
                );
            }
            cv.text(
                ordinal(k + 1),
                x + 24.0,
                y + 4.0,
                26.0,
                Face::Bold,
                if k == 0 { C_GOLD } else { C_TEXT },
                reveal,
                Align::Left,
            );
            render::car_portrait(
                cv,
                d.sprite,
                x + 120.0,
                y + 18.0,
                0.32,
                std::f32::consts::FRAC_PI_2,
            );
            cv.text(
                d.name,
                x + 160.0,
                y + 6.0,
                22.0,
                Face::Bold,
                C_TEXT,
                reveal,
                Align::Left,
            );
            let gap = if k == 0 {
                fmt_time(*t)
            } else {
                format!("+{:.2}", t - lead)
            };
            cv.text(
                &gap,
                x + 440.0,
                y + 8.0,
                18.0,
                Face::Body,
                C_DIM,
                reveal,
                Align::Right,
            );
            cv.text(
                &format!("+{pts}"),
                x + 530.0,
                y + 8.0,
                18.0,
                Face::Bold,
                C_CYAN,
                reveal,
                Align::Right,
            );
        }
        // verdict
        let vx = 750.0;
        cv.round_rect(vx, 130.0, 380.0, 470.0, 18.0, C_PANEL, 0.9);
        let need = campaign::qualify_place(c.round);
        let (title, col) = match res.verdict {
            Verdict::Qualified => ("QUALIFIED", C_GREEN),
            Verdict::Champion => ("CHAMPION!", C_GOLD),
            Verdict::SecondChance => ("MISSED IT", C_GOLD),
            Verdict::Eliminated => ("ELIMINATED", C_RED),
        };
        cv.text(
            title,
            vx + 190.0,
            154.0,
            36.0,
            Face::Bold,
            col,
            1.0,
            Align::Center,
        );
        let why = if c.is_final() {
            format!("Needed 1st, finished {}", ordinal(res.place))
        } else {
            format!("Needed top {need}, finished {}", ordinal(res.place))
        };
        cv.text(
            &why,
            vx + 190.0,
            206.0,
            20.0,
            Face::Body,
            C_TEXT,
            1.0,
            Align::Center,
        );
        if res.verdict == Verdict::SecondChance {
            wrap(
                cv,
                &format!(
                    "Second chance used ({} left). You will rerun this round.",
                    c.chances
                ),
                vx + 30.0,
                240.0,
                320.0,
                17.0,
                C_DIM,
            );
        }
        if res.verdict == Verdict::Eliminated {
            wrap(
                cv,
                "No second chances left. The Cup ends here.",
                vx + 30.0,
                240.0,
                320.0,
                17.0,
                C_DIM,
            );
        }
        let rows = [
            ("Prize", format!("${}", res.prize)),
            ("Style", format!("{}", res.style)),
            ("Cash", format!("${}", c.cash)),
        ];
        for (k, (a, b)) in rows.iter().enumerate() {
            let y = 300.0 + k as f32 * 38.0;
            cv.text(a, vx + 40.0, y, 22.0, Face::Body, C_DIM, 1.0, Align::Left);
            cv.text(
                b,
                vx + 340.0,
                y,
                22.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Right,
            );
        }
        cv.line(vx + 40.0, 420.0, vx + 340.0, 420.0, 1.5, 0x3A4458, 1.0);
        for (k, (a, b)) in self.last_stats.iter().enumerate() {
            let y = 436.0 + k as f32 * 34.0;
            cv.text(a, vx + 40.0, y, 17.0, Face::Body, C_DIM, 1.0, Align::Left);
            cv.text(
                b,
                vx + 340.0,
                y,
                17.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Right,
            );
        }
        self.footer(cv, "ENTER continue");
    }

    fn draw_standings(&self, cv: &mut Canvas) {
        let Some(c) = &self.campaign else { return };
        self.header(
            cv,
            "CUP STANDINGS",
            &format!("After round {} of {ROUNDS}", c.round + 1),
        );
        let x = W as f32 / 2.0 - 300.0;
        let st = c.standings();
        cv.round_rect(
            x,
            130.0,
            600.0,
            30.0 + st.len() as f32 * 56.0,
            18.0,
            C_PANEL,
            0.9,
        );
        let max = st.first().map(|s| s.1).unwrap_or(1).max(1);
        for (k, (d, pts)) in st.iter().enumerate() {
            let y = 148.0 + k as f32 * 56.0;
            let me = d.kind == Kind::Player;
            if me {
                cv.round_rect(x + 10.0, y - 6.0, 580.0, 50.0, 10.0, C_ACCENT, 0.35);
            }
            cv.text(
                &format!("{}", k + 1),
                x + 40.0,
                y + 4.0,
                24.0,
                Face::Bold,
                C_DIM,
                1.0,
                Align::Right,
            );
            cv.circle(x + 66.0, y + 18.0, 8.0, d.color, 1.0);
            cv.text(
                d.name,
                x + 86.0,
                y + 4.0,
                24.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Left,
            );
            let grow = (self.screen_t * 2.0).min(1.0);
            let bw = 240.0 * (*pts as f32 / max as f32) * grow;
            cv.round_rect(x + 250.0, y + 8.0, bw.max(4.0), 20.0, 6.0, d.color, 0.9);
            cv.text(
                &format!("{pts}"),
                x + 570.0,
                y + 4.0,
                24.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Right,
            );
        }
        self.footer(cv, "ENTER to the parts draft");
    }

    fn draw_draft(&self, cv: &mut Canvas) {
        let Some(c) = &self.campaign else { return };
        self.header(
            cv,
            "PICK A PART",
            "One of three. Parts stay on the car for the rest of the Cup.",
        );
        for (i, p) in c.draft.iter().enumerate() {
            let x = 130.0 + i as f32 * 350.0;
            let flip = ((self.screen_t - i as f32 * 0.15) * 3.0).clamp(0.0, 1.0);
            let sel = i == self.sel;
            let lift = if sel { -14.0 } else { 0.0 }
                + if self.draft_pick_t > 0.0 && sel {
                    -40.0 * self.draft_pick_t
                } else {
                    0.0
                };
            let w = 320.0 * flip;
            let cx = x + 160.0;
            let y = 170.0 + lift;
            if w < 4.0 {
                continue;
            }
            cv.round_rect(cx - w / 2.0, y, w, 380.0, 22.0, C_PANEL, 0.95);
            cv.round_rect_line(
                cx - w / 2.0,
                y,
                w,
                380.0,
                22.0,
                if sel { 4.0 } else { 2.0 },
                if sel { p.color() } else { 0x3A4458 },
                1.0,
            );
            if flip < 0.9 {
                continue;
            }
            cv.glow(
                cx,
                y + 110.0,
                80.0,
                p.color(),
                0.35 + if sel {
                    0.15 * (self.time * 4.0).sin()
                } else {
                    0.0
                },
            );
            cv.circle(cx, y + 110.0, 52.0, p.color(), 1.0);
            cv.circle(cx, y + 110.0, 40.0, 0x1B2233, 0.6);
            let initials: String = p
                .name()
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .collect();
            cv.text(
                &initials,
                cx,
                y + 92.0,
                34.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Center,
            );
            cv.text(
                p.name(),
                cx,
                y + 190.0,
                26.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Center,
            );
            wrap_center(cv, p.desc(), cx, y + 240.0, 270.0, 19.0, C_DIM);
        }
        self.footer(cv, "LEFT/RIGHT choose   ENTER take it");
    }

    fn draw_garage(&self, cv: &mut Canvas) {
        let Some(c) = &self.campaign else { return };
        self.header(cv, "GARAGE", &format!("Cash ${}", c.cash));
        let x = 140.0;
        cv.round_rect(820.0, 140.0, 330.0, 486.0, 22.0, C_PANEL, 0.92);
        cv.glow(985.0, 360.0, 170.0, C_ACCENT, 0.18);
        render::car_portrait(
            cv,
            "car_red_1",
            985.0,
            350.0,
            2.0,
            (self.time * 0.6).sin() * 0.25,
        );
        let st = c.stats();
        let bars = [
            ("Top speed", st.top / 760.0),
            ("Grip", st.grip / 1650.0),
            ("Toughness", 0.55 / st.fragility.max(0.2)),
            ("Nitro", st.nitro_secs / 4.4),
        ];
        for (k, (name, v)) in bars.iter().enumerate() {
            let y = 520.0 + k as f32 * 24.0;
            cv.text(name, 846.0, y, 15.0, Face::Body, C_DIM, 1.0, Align::Left);
            cv.round_rect(960.0, y + 3.0, 160.0, 10.0, 5.0, 0x0D1320, 1.0);
            cv.round_rect(
                960.0,
                y + 3.0,
                160.0 * v.clamp(0.05, 1.0),
                10.0,
                5.0,
                C_CYAN,
                1.0,
            );
        }
        for (i, (name, desc)) in GARAGE_SLOTS.iter().enumerate() {
            let y = 140.0 + i as f32 * 92.0;
            let sel = i == self.sel;
            cv.round_rect(
                x,
                y,
                640.0,
                80.0,
                16.0,
                if sel { 0x2C3650 } else { C_PANEL },
                0.92,
            );
            if sel {
                cv.round_rect_line(x, y, 640.0, 80.0, 16.0, 3.0, C_ACCENT, 1.0);
            }
            cv.text(
                name,
                x + 24.0,
                y + 12.0,
                26.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Left,
            );
            cv.text(
                desc,
                x + 24.0,
                y + 48.0,
                16.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Left,
            );
            let lvl = c.garage.levels[i];
            for p in 0..3 {
                cv.round_rect(
                    x + 330.0 + p as f32 * 34.0,
                    y + 18.0,
                    28.0,
                    18.0,
                    5.0,
                    if p < lvl { C_CYAN } else { 0x3A4458 },
                    1.0,
                );
            }
            let price = match c.level_cost(i) {
                Some(cost) => format!("${cost}"),
                None => "MAX".into(),
            };
            let afford = c.level_cost(i).is_some_and(|p| p <= c.cash);
            cv.text(
                &price,
                x + 620.0,
                y + 16.0,
                24.0,
                Face::Bold,
                if afford { C_GREEN } else { C_DIM },
                1.0,
                Align::Right,
            );
        }
        let rows = [("Undo last purchase", 4usize), ("Continue", 5)];
        for (k, (label, idx)) in rows.iter().enumerate() {
            let y = 520.0 + k as f32 * 58.0;
            let sel = self.sel == *idx;
            let enabled = *idx != 4 || !c.purchases.is_empty();
            cv.round_rect(
                x,
                y,
                640.0,
                48.0,
                14.0,
                if sel { C_ACCENT } else { C_PANEL },
                if sel { 0.95 } else { 0.8 },
            );
            cv.text(
                label,
                x + 320.0,
                y + 10.0,
                24.0,
                Face::Bold,
                if sel {
                    0x1B1F2A
                } else if enabled {
                    C_TEXT
                } else {
                    C_DIM
                },
                1.0,
                Align::Center,
            );
        }
        self.footer(cv, "UP/DOWN choose   ENTER buy   ESC undo");
    }

    fn draw_route(&self, cv: &mut Canvas) {
        let Some(c) = &self.campaign else { return };
        self.header(
            cv,
            "CHOOSE THE NEXT VENUE",
            &format!(
                "Round {} of {ROUNDS}: finish top {} to qualify",
                c.round + 1,
                campaign::qualify_place(c.round)
            ),
        );
        for (i, v) in c.options.iter().enumerate() {
            let x = 120.0 + i as f32 * 540.0;
            let sel = i == self.sel;
            let y = 150.0 + if sel { -10.0 } else { 0.0 };
            cv.round_rect(x, y, 500.0, 430.0, 22.0, C_PANEL, 0.94);
            cv.round_rect_line(
                x,
                y,
                500.0,
                430.0,
                22.0,
                if sel { 4.0 } else { 2.0 },
                if sel { C_ACCENT } else { 0x3A4458 },
                1.0,
            );
            // ground swatch with a road stripe
            let a = crate::assets::get();
            tile_rect(
                cv,
                a.s(v.spec.biome.ground()),
                x + 20.0,
                y + 20.0,
                460.0,
                190.0,
            );
            let road = v.spec.biome.road();
            cv.fill_rect(x + 20.0, y + 90.0, 460.0, 50.0, road, 1.0);
            let (k1, k2) = v.spec.biome.kerb();
            for s in 0..23 {
                cv.fill_rect(
                    x + 20.0 + s as f32 * 20.0,
                    y + 86.0,
                    20.0,
                    5.0,
                    if s % 2 == 0 { k1 } else { k2 },
                    1.0,
                );
                cv.fill_rect(
                    x + 20.0 + s as f32 * 20.0,
                    y + 139.0,
                    20.0,
                    5.0,
                    if s % 2 == 0 { k1 } else { k2 },
                    1.0,
                );
            }
            if v.spec.biome.night() {
                cv.fill_rect(x + 20.0, y + 20.0, 460.0, 190.0, 0x0A1030, 0.55);
            }
            cv.text(
                v.spec.biome.name(),
                x + 30.0,
                y + 228.0,
                32.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Left,
            );
            cv.round_rect(x + 30.0, y + 276.0, 150.0, 26.0, 13.0, C_ACCENT, 0.9);
            cv.text(
                v.label,
                x + 105.0,
                y + 280.0,
                16.0,
                Face::Bold,
                0x141925,
                1.0,
                Align::Center,
            );
            wrap(
                cv,
                v.spec.biome.blurb(),
                x + 30.0,
                y + 318.0,
                440.0,
                17.0,
                C_TEXT,
            );
            cv.text(
                &v.traits(),
                x + 30.0,
                y + 370.0,
                17.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Left,
            );
            cv.text(
                &format!("Prize x{:.2}", v.prize_mult),
                x + 470.0,
                y + 370.0,
                18.0,
                Face::Bold,
                C_GOLD,
                1.0,
                Align::Right,
            );
        }
        self.footer(cv, "LEFT/RIGHT choose   ENTER race there");
    }

    fn draw_end(&self, cv: &mut Canvas, won: bool) {
        let Some(c) = &self.campaign else { return };
        let t = self.screen_t;
        if won {
            // falling confetti
            for k in 0..120 {
                let fx = (k as f32 * 97.13) % W as f32;
                let fy = ((k as f32 * 53.7 + t * (90.0 + (k % 7) as f32 * 20.0))
                    % (H as f32 + 40.0))
                    - 20.0;
                let cols = [C_RED, C_GOLD, C_CYAN, C_GREEN, 0xFFFFFF];
                let w = 5.0 * (t * 4.0 + k as f32).sin().abs().max(0.2);
                cv.fill_rect(fx - w, fy, w * 2.0, 8.0, cols[k % 5], 0.9);
            }
            draw_trophy(cv, W as f32 / 2.0, 250.0, t);
            cv.text_outline(
                "CUP CHAMPION",
                W as f32 / 2.0,
                380.0,
                64.0,
                Face::Bold,
                C_GOLD,
                0x141925,
                1.0,
                Align::Center,
            );
            cv.text_sh(
                "You beat Sable at the Grand Prix.",
                W as f32 / 2.0,
                456.0,
                22.0,
                Face::Body,
                C_TEXT,
                1.0,
                Align::Center,
            );
        } else {
            let res = c.results.last();
            cv.text_outline(
                "CUP OVER",
                W as f32 / 2.0,
                150.0,
                72.0,
                Face::Bold,
                C_RED,
                0x141925,
                1.0,
                Align::Center,
            );
            if let Some(r) = res {
                let need = campaign::qualify_place(c.round);
                let why = if c.is_final() {
                    format!(
                        "The Grand Prix needed a win. You finished {}.",
                        ordinal(r.place)
                    )
                } else {
                    format!(
                        "Round {} at {} needed top {need}. You finished {}.",
                        c.round + 1,
                        r.biome.name(),
                        ordinal(r.place)
                    )
                };
                cv.text_sh(
                    &why,
                    W as f32 / 2.0,
                    250.0,
                    24.0,
                    Face::Body,
                    C_TEXT,
                    1.0,
                    Align::Center,
                );
            }
            cv.text_sh(
                "Upgrade the garage early and use the slipstream to fill your nitro.",
                W as f32 / 2.0,
                290.0,
                18.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Center,
            );
        }
        let s = c.score();
        let best = self.save.best_score;
        cv.round_rect(
            W as f32 / 2.0 - 260.0,
            500.0,
            520.0,
            120.0,
            18.0,
            C_PANEL,
            0.9,
        );
        cv.text(
            &format!("CUP SCORE {s}"),
            W as f32 / 2.0,
            516.0,
            34.0,
            Face::Bold,
            C_TEXT,
            1.0,
            Align::Center,
        );
        let pts = c
            .points
            .iter()
            .find(|p| p.0.kind == Kind::Player)
            .map(|p| p.1)
            .unwrap_or(0);
        cv.text(
            &format!(
                "{pts} points   style {}   best {best}{}",
                c.style_total,
                if s >= best && s > 0 {
                    "  NEW BEST!"
                } else {
                    ""
                }
            ),
            W as f32 / 2.0,
            568.0,
            19.0,
            Face::Body,
            C_DIM,
            1.0,
            Align::Center,
        );
        self.footer(cv, "ENTER to the title screen");
    }

    fn draw_tt_select(&self, cv: &mut Canvas) {
        self.header(
            cv,
            "TIME TRIAL",
            "Three laps alone. Your best lap becomes the ghost to beat.",
        );
        for i in 0..5 {
            let y = 140.0 + i as f32 * 96.0;
            let x = 170.0;
            let sel = i == self.sel;
            cv.round_rect(
                x,
                y,
                940.0,
                84.0,
                16.0,
                if sel { 0x2C3650 } else { C_PANEL },
                0.92,
            );
            if sel {
                cv.round_rect_line(x, y, 940.0, 84.0, 16.0, 3.0, C_ACCENT, 1.0);
            }
            tile_rect(
                cv,
                crate::assets::get().s(BIOMES[i].ground()),
                x + 16.0,
                y + 12.0,
                72.0,
                60.0,
            );
            cv.fill_rect(x + 16.0, y + 34.0, 72.0, 16.0, BIOMES[i].road(), 1.0);
            if BIOMES[i].night() {
                cv.fill_rect(x + 16.0, y + 12.0, 72.0, 60.0, 0x0A1030, 0.5);
            }
            cv.text(
                &format!("Classic {}", BIOMES[i].name()),
                x + 110.0,
                y + 14.0,
                26.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Left,
            );
            let rec = &self.save.records[i];
            let par = rec
                .par
                .map(|p| format!("Gold {}", fmt_time(p * 1.04)))
                .unwrap_or_default();
            cv.text(
                &par,
                x + 110.0,
                y + 50.0,
                16.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Left,
            );
            let best = rec.best.map(fmt_time).unwrap_or("--:--.--".into());
            cv.text(
                &best,
                x + 900.0,
                y + 18.0,
                28.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Right,
            );
            let m = match (rec.best, rec.par) {
                (Some(b), Some(p)) => medal(b, p),
                _ => 0,
            };
            draw_medal(cv, x + 640.0, y + 42.0, m);
            if !rec.ghost.is_empty() {
                cv.text(
                    "GHOST",
                    x + 900.0,
                    y + 54.0,
                    14.0,
                    Face::Bold,
                    0x9FD8FF,
                    1.0,
                    Align::Right,
                );
            }
        }
        self.footer(cv, "UP/DOWN choose   ENTER drive   ESC back");
    }

    fn draw_tt_results(&self, cv: &mut Canvas) {
        let i = self.tt_track;
        self.header(cv, "TIME TRIAL", &format!("Classic {}", BIOMES[i].name()));
        let x = W as f32 / 2.0 - 320.0;
        cv.round_rect(x, 140.0, 640.0, 420.0, 20.0, C_PANEL, 0.92);
        if let Some(race) = &self.race {
            for (k, l) in race.player().lap_times.iter().enumerate() {
                let y = 170.0 + k as f32 * 44.0;
                let best = race.best_lap.is_some_and(|b| (b - l).abs() < 1e-4);
                cv.text(
                    &format!("Lap {}", k + 1),
                    x + 40.0,
                    y,
                    24.0,
                    Face::Body,
                    C_DIM,
                    1.0,
                    Align::Left,
                );
                cv.text(
                    &fmt_time(*l),
                    x + 600.0,
                    y,
                    24.0,
                    Face::Bold,
                    if best { C_GREEN } else { C_TEXT },
                    1.0,
                    Align::Right,
                );
            }
        }
        draw_medal(cv, W as f32 / 2.0, 360.0, self.tt_medal);
        let name = ["No medal", "Bronze", "Silver", "Gold"][self.tt_medal as usize];
        cv.text(
            name,
            W as f32 / 2.0,
            410.0,
            28.0,
            Face::Bold,
            C_TEXT,
            1.0,
            Align::Center,
        );
        if self.tt_new_record {
            let a = 0.6 + 0.4 * (self.time * 6.0).sin();
            cv.text_outline(
                "NEW RECORD - ghost saved",
                W as f32 / 2.0,
                460.0,
                26.0,
                Face::Bold,
                C_GOLD,
                0x141925,
                a,
                Align::Center,
            );
        }
        if let Some(p) = self.save.records[i].par {
            cv.text(
                &format!(
                    "Gold {}   Silver {}   Bronze {}",
                    fmt_time(p * 1.04),
                    fmt_time(p * 1.12),
                    fmt_time(p * 1.22)
                ),
                W as f32 / 2.0,
                510.0,
                17.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Center,
            );
        }
        self.footer(cv, "ENTER or R race again   ESC circuits");
    }

    fn draw_pause(&self, cv: &mut Canvas) {
        cv.darken(0.5);
        cv.text_outline(
            "PAUSED",
            W as f32 / 2.0,
            170.0,
            64.0,
            Face::Bold,
            C_TEXT,
            0x141925,
            1.0,
            Align::Center,
        );
        let tt = self
            .race
            .as_ref()
            .is_some_and(|r| r.mode == Mode::TimeTrial);
        let items = [
            "Resume",
            if tt { "Restart" } else { "Restart round" },
            "Settings",
            "Quit to title",
        ];
        for (i, it) in items.iter().enumerate() {
            let y = 290.0 + i as f32 * 58.0;
            let sel = i == self.pause_sel;
            cv.round_rect(
                W as f32 / 2.0 - 180.0,
                y,
                360.0,
                48.0,
                14.0,
                if sel { C_ACCENT } else { C_PANEL },
                0.92,
            );
            cv.text(
                it,
                W as f32 / 2.0,
                y + 11.0,
                24.0,
                Face::Bold,
                if sel { 0x1B1F2A } else { C_TEXT },
                1.0,
                Align::Center,
            );
        }
        if !tt {
            cv.text_sh(
                "Restarting the round re-rolls the grid on the same track.",
                W as f32 / 2.0,
                540.0,
                16.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Center,
            );
        }
    }
}

/// Run the autopilot alone for two laps and return its best lap.
pub fn autopilot_lap(spec: TrackSpec) -> f32 {
    let track = Track::generate(spec);
    let mut race = Race::new(
        track,
        Mode::TimeTrial,
        2,
        campaign::build_stats(&Default::default(), &[]),
        &[],
        1.0,
        1,
    );
    race.autopilot = true;
    race.countdown = 0.0;
    for _ in 0..(60 * 120) {
        race.update(1.0 / 60.0, Controls::default());
        race.sounds.clear();
        race.skids.clear();
        if race.phase != Phase::Running && race.phase != Phase::Countdown {
            break;
        }
    }
    race.best_lap.unwrap_or(60.0)
}

pub fn wrap(cv: &mut Canvas, text: &str, x: f32, y: f32, w: f32, size: f32, c: u32) {
    let mut line = String::new();
    let mut yy = y;
    for word in text.split_whitespace() {
        let cand = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if cv.text_width(&cand, size, Face::Body) > w && !line.is_empty() {
            cv.text(&line, x, yy, size, Face::Body, c, 1.0, Align::Left);
            yy += size * 1.35;
            line = word.to_string();
        } else {
            line = cand;
        }
    }
    if !line.is_empty() {
        cv.text(&line, x, yy, size, Face::Body, c, 1.0, Align::Left);
    }
}

fn wrap_center(cv: &mut Canvas, text: &str, cx: f32, y: f32, w: f32, size: f32, c: u32) {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let cand = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if cv.text_width(&cand, size, Face::Body) > w && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = cand;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    for (k, l) in lines.iter().enumerate() {
        cv.text(
            l,
            cx,
            y + k as f32 * size * 1.35,
            size,
            Face::Body,
            c,
            1.0,
            Align::Center,
        );
    }
}

pub fn draw_track_preview(cv: &mut Canvas, t: &Track, x: f32, y: f32, w: f32, h: f32) {
    let mut min = t.pts[0];
    let mut max = t.pts[0];
    for p in &t.pts {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    let k = ((w - 40.0) / (max.x - min.x)).min((h - 40.0) / (max.y - min.y));
    let off = crate::math::v2(
        x + (w - (max.x - min.x) * k) / 2.0,
        y + (h - (max.y - min.y) * k) / 2.0,
    );
    let tp = |p: crate::math::V2| (p - min) * k + off;
    let n = t.pts.len();
    let road = t.spec.biome.road();
    for pass in 0..2 {
        for i in (0..n).step_by(3) {
            let a = tp(t.pts[i]);
            let b = tp(t.pts[(i + 3) % n]);
            if pass == 0 {
                cv.line(a.x, a.y, b.x, b.y, 16.0, 0x0D1320, 1.0);
            } else {
                cv.line(a.x, a.y, b.x, b.y, 10.0, road, 1.0);
            }
        }
    }
    for c in &t.corners {
        let p = tp(t.point_at(c.apex_s, 0.0));
        let col = [C_GREEN, C_GREEN, C_GOLD, C_RED][c.severity as usize];
        cv.circle(p.x, p.y, 4.0, col, 1.0);
    }
    let s = tp(t.pts[0]);
    cv.circle(s.x, s.y, 8.0, C_TEXT, 1.0);
    let d = t.tan[0];
    cv.line(
        s.x,
        s.y,
        s.x + d.x * 22.0,
        s.y + d.y * 22.0,
        4.0,
        C_ACCENT,
        1.0,
    );
    cv.text(
        "START",
        s.x,
        s.y + 12.0,
        13.0,
        Face::Bold,
        C_TEXT,
        1.0,
        Align::Center,
    );
}

pub fn draw_medal(cv: &mut Canvas, x: f32, y: f32, m: u8) {
    let col = [0x3A4458, 0xC98A4B, 0xC9D3DE, C_GOLD][m as usize];
    cv.circle(x + 2.0, y + 3.0, 22.0, 0x000000, 0.3);
    cv.circle(x, y, 22.0, col, 1.0);
    cv.circle(x, y, 15.0, mix(col, 0xFFFFFF, 0.3), 1.0);
    if m > 0 {
        cv.text(
            &format!("{}", 4 - m),
            x,
            y - 11.0,
            20.0,
            Face::Bold,
            0x2A2F3A,
            1.0,
            Align::Center,
        );
    }
}

fn draw_trophy(cv: &mut Canvas, x: f32, y: f32, t: f32) {
    cv.glow(x, y - 20.0, 160.0, C_GOLD, 0.35 + 0.1 * (t * 3.0).sin());
    cv.round_rect(x - 70.0, y + 70.0, 140.0, 26.0, 6.0, 0x6B4A1A, 1.0);
    cv.round_rect(x - 18.0, y + 30.0, 36.0, 44.0, 4.0, 0xD9A92E, 1.0);
    cv.circle(x, y - 30.0, 70.0, 0xF2C33A, 1.0);
    cv.fill_rect(x - 80.0, y - 110.0, 160.0, 80.0, 0x000000, 0.0);
    cv.ring(x - 74.0, y - 40.0, 26.0, 10.0, 0xF2C33A, 1.0);
    cv.ring(x + 74.0, y - 40.0, 26.0, 10.0, 0xF2C33A, 1.0);
    cv.round_rect(x - 70.0, y - 100.0, 140.0, 70.0, 10.0, 0xF2C33A, 1.0);
    cv.circle(x - 30.0, y - 60.0, 12.0, 0xFFF1B8, 0.6);
    let star: Vec<(f32, f32)> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let r = if k % 2 == 0 { 26.0 } else { 11.0 };
            (x + a.cos() * r, y - 52.0 + a.sin() * r)
        })
        .collect();
    for k in 0..10 {
        cv.tri([(x, y - 52.0), star[k], star[(k + 1) % 10]], 0xC98F1E, 1.0);
    }
}

/// Fill a rectangle with a repeating texture (clipped).
pub fn tile_rect(cv: &mut Canvas, tile: &crate::canvas::Sprite, x: f32, y: f32, w: f32, h: f32) {
    let (x0, y0) = (x.max(0.0) as usize, y.max(0.0) as usize);
    let (x1, y1) = (((x + w) as usize).min(W), ((y + h) as usize).min(H));
    for yy in y0..y1 {
        for xx in x0..x1 {
            let p = tile.px[((yy - y0) % tile.h) * tile.w + (xx - x0) % tile.w];
            cv.px[yy * W + xx] = p & 0xFFFFFF;
        }
    }
}
