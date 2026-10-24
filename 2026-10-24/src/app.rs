//! The application: screens, menus, input, the stage loop and the run flow.
//! Everything is engine-free; `main.rs` feeds it input and drains its sound.

use std::path::PathBuf;
use std::sync::Arc;

use crate::audio::{MusicState, Sfx};
use crate::boss::BState;
use crate::game::Game;
use crate::gfx::canvas::{Canvas, H, W, lerp_color};
use crate::gfx::fx::Fx;
use crate::gfx::scene::{Camera, SceneCache, draw_scene};
use crate::gfx::sprites::{WrenPose, draw_wren};
use crate::gfx::ui::*;
use crate::level::*;
use crate::run::*;
use crate::save::Save;
use crate::sim::*;
use crate::stagegen::generate;

pub const STEP: f32 = 1.0 / 60.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Jump,
    Hook,
    Rewind,
    Pause,
    Mute,
    Help,
    Enter,
    Tab,
    Num1,
    Num2,
    Num3,
    Back,
}

#[derive(Clone, Default, Debug)]
pub struct Input {
    pub mouse: (f32, f32),
    pub mouse_down: bool,
    pub mouse_pressed: bool,
    pub rmouse_down: bool,
    pub rmouse_pressed: bool,
    pub down: Vec<Key>,
    pub pressed: Vec<Key>,
}

impl Input {
    pub fn is_down(&self, k: Key) -> bool {
        self.down.contains(&k)
    }

    pub fn was_pressed(&self, k: Key) -> bool {
        self.pressed.contains(&k)
    }

    pub fn any_pressed(&self) -> bool {
        !self.pressed.is_empty() || self.mouse_pressed
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Workshop,
    Help,
    Intro,
    Playing,
    Paused,
    Clear,
    Draft,
    Route,
    Victory,
    Defeat,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    Play,
    Training,
    Workshop,
    Help,
    Quit,
    Resume,
    Restart,
    ToTitle,
    Continue,
    Pick(usize),
    PickRoute(Route),
    Buy(usize),
    Back,
    GiveUp,
    ToggleAssist,
    CycleSpeed,
    ToggleExplorer,
    ToggleMute,
    HeatUp,
    HeatDown,
}

pub struct App {
    pub screen: Screen,
    pub save: Save,
    save_path: Option<PathBuf>,
    pub run: Option<Run>,
    pub game: Option<Game>,
    pub cam: Camera,
    pub fx: Fx,
    cache: Option<SceneCache>,
    pub time: f32,
    anim: f32,
    acc: f32,
    sfx: Vec<Sfx>,
    pub quit: bool,
    pub muted: bool,
    hot: Vec<(Rect, Act, bool)>,
    menu_idx: usize,
    intro_t: f32,
    clear: Option<StageResult>,
    clear_t: f32,
    draft: Vec<Boon>,
    picks_total: u32,
    freeze: f32,
    squash: f32,
    step_t: f32,
    jump_edge: bool,
    hook_edge: bool,
    hook_by_mouse: bool,
    mouse_idle: f32,
    last_mouse: (f32, f32),
    hint_t: f32,
    rewind_t: f32,
    was_rewinding: bool,
    seed: u64,
    help_back: Screen,
    new_best_score: bool,
    end_note: String,
    title_fx: Fx,
    pub last_cleared_stage: Option<usize>,
    toast: Option<(String, f32)>,
    hint: Option<(String, f32)>,
    pub stats_seen: SeenStats,
    next_route: Route,
    boss_stun_seen: bool,
    tutorial_active: bool,
}

/// What the app has shown the player (for playtests).
#[derive(Clone, Default, Debug)]
pub struct SeenStats {
    pub stages_started: u32,
    pub stages_cleared: u32,
    pub rewinds: u32,
    pub drafts: u32,
    pub routes: u32,
}

impl App {
    pub fn new(save_path: Option<PathBuf>, seed: u64) -> App {
        let save = save_path
            .as_ref()
            .map(|p| Save::load(p))
            .unwrap_or_default();
        let muted = save.muted;
        App {
            screen: Screen::Title,
            save,
            save_path,
            run: None,
            game: None,
            cam: Camera::new(),
            fx: Fx::new(),
            cache: None,
            time: 0.0,
            anim: 0.0,
            acc: 0.0,
            sfx: vec![],
            quit: false,
            muted,
            hot: vec![],
            menu_idx: 0,
            intro_t: 0.0,
            clear: None,
            clear_t: 0.0,
            draft: vec![],
            picks_total: 1,
            freeze: 0.0,
            squash: 0.0,
            step_t: 0.0,
            jump_edge: false,
            hook_edge: false,
            hook_by_mouse: false,
            mouse_idle: 99.0,
            last_mouse: (0.0, 0.0),
            hint_t: 0.0,
            rewind_t: 0.0,
            was_rewinding: false,
            seed,
            help_back: Screen::Title,
            new_best_score: false,
            end_note: String::new(),
            title_fx: Fx::new(),
            last_cleared_stage: None,
            toast: None,
            hint: None,
            stats_seen: SeenStats::default(),
            next_route: Route::Steady,
            boss_stun_seen: false,
            tutorial_active: false,
        }
    }

    /// Test helper: (re)build the current stage and skip the intro card.
    pub fn start_stage_for_test(&mut self) {
        self.begin_stage();
        self.screen = Screen::Playing;
        self.acc = 0.0;
    }

    /// Should the OS cursor be hidden (we draw a crosshair while playing)?
    pub fn cursor_hidden(&self) -> bool {
        self.screen == Screen::Playing
    }

    pub fn take_sfx(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sfx)
    }

    fn play(&mut self, s: Sfx) {
        self.sfx.push(s);
    }

    fn persist(&mut self) {
        self.save.muted = self.muted;
        if let Some(p) = &self.save_path {
            let _ = self.save.store(p);
        }
    }

    // ----------------------------------------------------------- music

    pub fn music(&self) -> MusicState {
        let mut intensity = 0.22;
        let mut theme = 0u8;
        let mut rewinding = false;
        match self.screen {
            Screen::Playing | Screen::Paused | Screen::Intro => {
                if let (Some(g), Some(run)) = (&self.game, &self.run) {
                    let w = &g.world;
                    theme = if w.level.boss {
                        5
                    } else {
                        w.level.biome.index() as u8
                    };
                    let _ = run;
                    let sp = (w.p.speed / 650.0).clamp(0.0, 1.0);
                    let threats = w
                        .ents
                        .iter()
                        .filter(|e| {
                            e.alive
                                && e.kind.is_threat()
                                && (e.x - w.p.x).abs() < 380.0
                                && (e.y - w.p.y).abs() < 300.0
                        })
                        .count()
                        .min(4) as f32;
                    intensity = 0.2 + 0.32 * sp + 0.09 * threats;
                    if w.p.rope.is_some() {
                        intensity += 0.1;
                    }
                    if w.stats.chain >= 4 {
                        intensity += 0.1;
                    }
                    if let Some(b) = &w.boss {
                        intensity = match b.state {
                            BState::Intro => 0.45,
                            BState::Stun => 0.95,
                            BState::Dead => 0.3,
                            _ => 0.75 + 0.05 * b.phase() as f32,
                        };
                    }
                    if self.screen == Screen::Paused || self.screen == Screen::Intro {
                        intensity = 0.15;
                    }
                    rewinding = g.rewinding;
                }
            }
            Screen::Clear | Screen::Draft | Screen::Route => {
                intensity = 0.3;
                theme = self
                    .run
                    .as_ref()
                    .map(|r| (r.stage.min(4)) as u8)
                    .unwrap_or(0);
            }
            Screen::Victory => {
                intensity = 0.85;
                theme = 5;
            }
            Screen::Defeat => {
                intensity = 0.1;
                theme = 1;
            }
            _ => {}
        }
        MusicState {
            intensity: intensity.clamp(0.0, 1.0),
            theme,
            rewinding,
        }
    }

    // ------------------------------------------------------ run control

    fn new_seed(&mut self) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.seed ^ (self.time * 1000.0) as u64
    }

    pub fn start_run(&mut self, training: bool) {
        let seed = self.new_seed();
        let run = Run::new(seed, &self.save, training);
        self.tutorial_active = training;
        self.run = Some(run);
        self.begin_stage();
    }

    fn begin_stage(&mut self) {
        let Some(run) = &self.run else { return };
        let spec = run.spec();
        let level = Arc::new(generate(&spec));
        let biome = level.biome;
        let game = run.make_game(level);
        self.cam = Camera::new();
        self.cam.snap(&game.world);
        self.game = Some(game);
        if self.cache.as_ref().is_none_or(|c| c.biome != biome) {
            self.cache = Some(SceneCache::new(biome));
        }
        self.fx.clear();
        self.screen = Screen::Intro;
        self.intro_t = 0.0;
        self.hint_t = 14.0;
        self.acc = 0.0;
        self.boss_stun_seen = false;
        self.stats_seen.stages_started += 1;
        self.play(Sfx::Banner);
    }

    fn on_cleared(&mut self) {
        let (Some(run), Some(game)) = (self.run.as_mut(), self.game.as_ref()) else {
            return;
        };
        let was_training = run.training;
        let res = run.finish_stage(game, &mut self.save);
        self.last_cleared_stage = Some(res.stage);
        self.stats_seen.stages_cleared += 1;
        if was_training {
            self.save.tutorial_done = true;
        }
        self.clear = Some(res);
        self.clear_t = 0.0;
        self.screen = Screen::Clear;
        self.persist();
        self.play(Sfx::Cleared);
    }

    fn on_defeat(&mut self) {
        let (Some(run), Some(game)) = (self.run.as_ref(), self.game.as_ref()) else {
            return;
        };
        let cause = game.world.last_cause;
        self.end_note = if cause.is_empty() {
            "You ran out of hearts.".to_string()
        } else {
            format!("Brought down by {cause}.")
        };
        if !run.training && !run.explorer {
            let stage = run.stage as u32 + 1;
            let score = run.score + game.world.stats.score;
            let glim = run.crystals + game.world.stats.crystals;
            self.new_best_score = self.save.record_run(score, stage, false, glim);
        } else {
            self.new_best_score = false;
        }
        self.screen = Screen::Defeat;
        self.persist();
        self.play(Sfx::Defeat);
    }

    fn after_clear(&mut self) {
        let Some(run) = self.run.as_mut() else { return };
        let daring = run.route == Route::Daring && run.stage > 0 && run.stage < 5;
        let relics = self.clear.as_ref().map(|c| c.chests).unwrap_or(0).min(2);
        let was_training = run.training;
        let boss = run.stage >= 5;
        let more = run.advance();
        if !more {
            // victory
            if !run.explorer {
                let score = run.final_score();
                let glim = run.glimmer_earned();
                self.new_best_score = self.save.record_run(score, 7, true, glim);
            } else {
                self.new_best_score = false;
            }
            self.screen = Screen::Victory;
            self.persist();
            self.play(Sfx::Victory);
            return;
        }
        let _ = boss;
        if was_training {
            // straight into the real run
            self.tutorial_active = false;
            self.begin_stage();
            return;
        }
        run.pending_picks = 1 + daring as u32 + relics;
        self.picks_total = run.pending_picks;
        self.draft = run.draft_options();
        if self.draft.is_empty() {
            run.pending_picks = 0;
            self.after_draft();
            return;
        }
        self.screen = Screen::Draft;
        self.stats_seen.drafts += 1;
        self.menu_idx = 0;
    }

    fn after_draft(&mut self) {
        let Some(run) = self.run.as_mut() else { return };
        if run.stage < 5 {
            self.screen = Screen::Route;
            self.stats_seen.routes += 1;
            self.menu_idx = 0;
        } else {
            run.route = Route::Steady;
            self.begin_stage();
        }
    }

    fn open_help(&mut self) {
        self.help_back = self.screen;
        self.screen = Screen::Help;
    }

    fn activate(&mut self, a: Act) {
        match a {
            Act::Play => {
                let training = !self.save.tutorial_done;
                self.start_run(training);
            }
            Act::Training => self.start_run(true),
            Act::Workshop => {
                self.screen = Screen::Workshop;
                self.menu_idx = 0;
            }
            Act::Help => self.open_help(),
            Act::Quit => self.quit = true,
            Act::Resume => self.screen = Screen::Playing,
            Act::Restart => {
                let training = self.run.as_ref().is_some_and(|r| r.training);
                self.start_run(training);
            }
            Act::ToTitle => {
                self.screen = Screen::Title;
                self.menu_idx = 0;
                self.game = None;
            }
            Act::Continue => match self.screen {
                Screen::Clear => self.after_clear(),
                Screen::Victory | Screen::Defeat => {
                    self.screen = Screen::Title;
                    self.menu_idx = 0;
                    self.game = None;
                }
                _ => {}
            },
            Act::Pick(i) => {
                let Some(b) = self.draft.get(i).copied() else {
                    return;
                };
                let Some(run) = self.run.as_mut() else { return };
                run.pick(b);
                let mut next_draft = None;
                if run.pending_picks > 0 {
                    let d = run.draft_options();
                    if d.is_empty() {
                        run.pending_picks = 0;
                    } else {
                        next_draft = Some(d);
                    }
                }
                let done = run.pending_picks == 0;
                self.play(Sfx::Draft);
                if let Some(d) = next_draft {
                    self.draft = d;
                }
                if done {
                    self.after_draft();
                }
            }
            Act::PickRoute(r) => {
                if let Some(run) = self.run.as_mut() {
                    run.set_route(r);
                    self.next_route = r;
                }
                self.play(Sfx::Select);
                self.begin_stage();
            }
            Act::Buy(i) => self.buy_perk(i),
            Act::Back => {
                self.screen = if self.screen == Screen::Help {
                    self.help_back
                } else {
                    Screen::Title
                };
                self.menu_idx = 0;
            }
            Act::GiveUp => {
                if let Some(g) = self.game.as_mut() {
                    g.world.outcome = Outcome::Dead;
                    g.echo = 0.0;
                }
                self.screen = Screen::Playing;
                self.on_defeat();
            }
            Act::ToggleAssist => {
                self.save.aim_assist = !self.save.aim_assist;
                if let Some(g) = self.game.as_mut() {
                    g.world.mods.assist_aim = self.save.aim_assist;
                }
                if let Some(r) = self.run.as_mut() {
                    r.mods.assist_aim = self.save.aim_assist;
                }
                self.persist();
            }
            Act::CycleSpeed => {
                self.save.speed = (self.save.speed + 1) % 3;
                self.persist();
            }
            Act::ToggleExplorer => {
                self.save.explorer = !self.save.explorer;
                if let Some(g) = self.game.as_mut() {
                    g.world.mods.invincible = self.save.explorer;
                }
                if let Some(r) = self.run.as_mut() {
                    r.mods.invincible = self.save.explorer;
                    r.explorer = self.save.explorer || r.explorer;
                }
                self.persist();
            }
            Act::ToggleMute => {
                self.muted = !self.muted;
                self.persist();
            }
            Act::HeatUp => {
                self.save.heat = (self.save.heat + 1).min(self.save.heat_unlocked);
                self.persist();
            }
            Act::HeatDown => {
                self.save.heat = self.save.heat.saturating_sub(1);
                self.persist();
            }
        }
        if !matches!(
            a,
            Act::Pick(_) | Act::Play | Act::Training | Act::PickRoute(_)
        ) {
            self.play(Sfx::Click);
        }
    }

    fn buy_perk(&mut self, i: usize) {
        if i >= PERKS.len() {
            return;
        }
        let lvl = self.save.perks[i] as usize;
        let def = &PERKS[i];
        if lvl >= def.costs.len() {
            self.play(Sfx::Error);
            return;
        }
        let cost = def.costs[lvl];
        if self.save.glimmer >= cost {
            self.save.glimmer -= cost;
            self.save.perks[i] += 1;
            self.persist();
            self.play(Sfx::Draft);
        } else {
            self.play(Sfx::Error);
        }
    }

    // ---------------------------------------------------------- update

    pub fn update(&mut self, dt: f32, input: &Input) {
        let dt = dt.min(0.05);
        self.time += dt;
        self.anim += dt;
        self.fx.update(dt);
        self.title_fx.update(dt);
        if let Some((_, t)) = &mut self.toast {
            *t -= dt;
            if *t <= 0.0 {
                self.toast = None;
            }
        }
        if let Some((_, t)) = &mut self.hint {
            *t -= dt;
            if *t <= 0.0 {
                self.hint = None;
            }
        }
        if input.mouse != self.last_mouse {
            self.mouse_idle = 0.0;
            self.last_mouse = input.mouse;
        } else {
            self.mouse_idle += dt;
        }
        if input.was_pressed(Key::Mute) {
            self.activate(Act::ToggleMute);
        }
        match self.screen {
            Screen::Playing => self.update_playing(dt, input),
            Screen::Intro => {
                self.intro_t += dt;
                self.fx
                    .update_weather(self.biome(), self.cam.x, self.cam.y, 0.0, dt);
                if self.intro_t > 0.5 && input.any_pressed() || self.intro_t > 3.2 {
                    self.screen = Screen::Playing;
                    self.acc = 0.0;
                }
            }
            Screen::Paused => {
                if input.was_pressed(Key::Pause) || input.was_pressed(Key::Back) {
                    self.screen = Screen::Playing;
                } else {
                    self.menu_input(input);
                }
                if input.was_pressed(Key::Help) {
                    self.open_help();
                }
            }
            Screen::Clear => {
                self.clear_t += dt;
                if input.was_pressed(Key::Enter) || input.was_pressed(Key::Jump) {
                    if self.clear_t > 0.8 {
                        self.activate(Act::Continue);
                    }
                } else {
                    self.menu_input(input);
                }
            }
            Screen::Draft => {
                for (k, i) in [(Key::Num1, 0), (Key::Num2, 1), (Key::Num3, 2)] {
                    if input.was_pressed(k) && i < self.draft.len() {
                        self.activate(Act::Pick(i));
                        return;
                    }
                }
                self.menu_input(input);
            }
            Screen::Route => {
                if input.was_pressed(Key::Num1) {
                    self.activate(Act::PickRoute(Route::Steady));
                } else if input.was_pressed(Key::Num2) {
                    self.activate(Act::PickRoute(Route::Daring));
                } else {
                    self.menu_input(input);
                }
            }
            Screen::Help => {
                if input.was_pressed(Key::Back)
                    || input.was_pressed(Key::Help)
                    || input.was_pressed(Key::Enter)
                    || input.mouse_pressed
                {
                    self.activate(Act::Back);
                }
            }
            Screen::Workshop => {
                if input.was_pressed(Key::Back) {
                    self.activate(Act::Back);
                } else {
                    self.menu_input(input);
                }
            }
            Screen::Title | Screen::Victory | Screen::Defeat => {
                if self.screen == Screen::Title && input.was_pressed(Key::Help) {
                    self.open_help();
                } else if self.screen == Screen::Title && input.was_pressed(Key::Tab) {
                    self.activate(Act::Training);
                } else {
                    self.menu_input(input);
                }
            }
        }
    }

    fn biome(&self) -> Biome {
        self.game
            .as_ref()
            .map(|g| g.world.level.biome)
            .unwrap_or(Biome::Mossbluff)
    }

    /// Keyboard and mouse navigation over the buttons registered at the last draw.
    fn menu_input(&mut self, input: &Input) {
        if self.hot.is_empty() {
            return;
        }
        let n = self.hot.len();
        if self.menu_idx >= n {
            self.menu_idx = 0;
        }
        if input.was_pressed(Key::Down)
            || input.was_pressed(Key::Right) && self.screen != Screen::Draft
        {
            self.menu_idx = (self.menu_idx + 1) % n;
            self.play(Sfx::Click);
        }
        if input.was_pressed(Key::Up)
            || input.was_pressed(Key::Left) && self.screen != Screen::Draft
        {
            self.menu_idx = (self.menu_idx + n - 1) % n;
            self.play(Sfx::Click);
        }
        if (self.screen == Screen::Draft || self.screen == Screen::Route)
            && input.was_pressed(Key::Right)
        {
            self.menu_idx = (self.menu_idx + 1) % n;
        }
        if (self.screen == Screen::Draft || self.screen == Screen::Route)
            && input.was_pressed(Key::Left)
        {
            self.menu_idx = (self.menu_idx + n - 1) % n;
        }
        for (i, (r, _, en)) in self.hot.iter().enumerate() {
            if r.contains(input.mouse) && *en && input.mouse != self.last_mouse_for_menu() {
                self.menu_idx = i;
            }
        }
        let mut fire = None;
        if input.mouse_pressed {
            for (r, a, en) in &self.hot {
                if r.contains(input.mouse) && *en {
                    fire = Some(*a);
                }
            }
        }
        if fire.is_none() && (input.was_pressed(Key::Enter) || input.was_pressed(Key::Jump)) {
            let (_, a, en) = self.hot[self.menu_idx.min(n - 1)];
            if en {
                fire = Some(a);
            } else {
                self.play(Sfx::Error);
            }
        }
        if let Some(a) = fire {
            self.activate(a);
        }
    }

    fn last_mouse_for_menu(&self) -> (f32, f32) {
        (-1.0, -1.0)
    }

    fn build_controls(&mut self, input: &Input) -> (Controls, bool) {
        let left = input.is_down(Key::Left);
        let right = input.is_down(Key::Right);
        let dir = (right as i8) - (left as i8);
        if input.was_pressed(Key::Jump) {
            self.jump_edge = true;
        }
        let key_hook = input.is_down(Key::Hook);
        if input.mouse_pressed {
            self.hook_edge = true;
            self.hook_by_mouse = true;
        }
        if input.was_pressed(Key::Hook) {
            self.hook_edge = true;
            self.hook_by_mouse = false;
        }
        let hook_held = input.mouse_down || key_hook;
        if !hook_held {
            self.hook_edge = false;
        }
        let aim = if self.hook_by_mouse || self.mouse_idle < 2.5 {
            Some((input.mouse.0 + self.cam.x, input.mouse.1 + self.cam.y))
        } else {
            None
        };
        let c = Controls {
            dir,
            up: input.is_down(Key::Up),
            down: input.is_down(Key::Down),
            jump_pressed: self.jump_edge,
            jump_held: input.is_down(Key::Jump),
            hook_pressed: self.hook_edge,
            hook_held,
            aim: if self.hook_by_mouse || input.mouse_down {
                aim
            } else {
                aim.filter(|_| !key_hook)
            },
        };
        self.jump_edge = false;
        self.hook_edge = false;
        (c, input.is_down(Key::Rewind) || input.rmouse_down)
    }

    fn update_playing(&mut self, dt: f32, input: &Input) {
        if input.was_pressed(Key::Pause) || input.was_pressed(Key::Back) {
            self.screen = Screen::Paused;
            self.menu_idx = 0;
            self.play(Sfx::Click);
            return;
        }
        if input.was_pressed(Key::Help) {
            self.open_help();
            return;
        }
        if self.tutorial_active && input.was_pressed(Key::Tab) {
            // skip training
            if let Some(run) = self.run.as_mut() {
                run.training = false;
                run.stage = 0;
            }
            self.save.tutorial_done = true;
            self.persist();
            self.tutorial_active = false;
            self.begin_stage();
            return;
        }
        let (controls, rewind) = self.build_controls(input);
        let Some(mut game) = self.game.take() else {
            return;
        };
        self.hint_t = (self.hint_t - dt).max(0.0);
        self.squash = (self.squash - dt * 6.0).max(0.0);
        self.freeze = (self.freeze - dt).max(0.0);
        if self.freeze <= 0.0 {
            let speed = [1.0, 0.85, 0.7][self.save.speed as usize % 3];
            self.acc = (self.acc + dt * speed).min(0.12);
            while self.acc >= STEP {
                self.acc -= STEP;
                let events = game.tick(&controls, rewind);
                for e in &events {
                    self.handle_event(e, &game);
                }
                self.footsteps(&game, STEP);
                if self.freeze > 0.0 {
                    break;
                }
            }
        }
        // rewind feedback
        if game.rewinding {
            if !self.was_rewinding {
                self.play(Sfx::RewindStart);
                self.stats_seen.rewinds += 1;
            }
            self.rewind_t += dt;
            if self.rewind_t > 0.11 {
                self.rewind_t = 0.0;
                self.play(Sfx::RewindTick);
            }
        } else if self.was_rewinding {
            self.play(Sfx::RewindEnd);
        }
        self.was_rewinding = game.rewinding;
        // camera, scarf, trail
        let w = &game.world;
        self.cam.follow(w, dt);
        let neck = (w.p.x - w.p.facing as f32 * 1.0, w.p.y - 6.0);
        self.fx
            .update_scarf(neck, w.p.vx, w.p.vy, w.p.facing as f32, dt);
        if w.p.charged && !game.rewinding {
            self.fx.push_trail(w.p.x, w.p.y);
        }
        self.fx
            .update_weather(w.level.biome, self.cam.x, self.cam.y, w.gust, dt);
        let new_hint = self.pick_hint(&game);
        let outcome = game.world.outcome;
        let dead_for = game.dead_for;
        let can_rewind = game.can_rewind();
        self.game = Some(game);
        if let Some((bit, text)) = new_hint {
            self.save.hints |= 1 << bit;
            self.hint = Some((text.to_string(), 5.0));
            self.play(Sfx::Banner);
            self.persist();
        }
        match outcome {
            Outcome::Cleared => self.on_cleared(),
            Outcome::Dead => {
                let give_up = input.was_pressed(Key::Enter);
                if give_up || !can_rewind && dead_for > 1.3 || dead_for > 4.5 {
                    self.on_defeat();
                }
            }
            Outcome::Playing => {}
        }
    }

    /// The next one-time hint worth showing, as (bit, text).
    fn pick_hint(&self, g: &Game) -> Option<(u32, &'static str)> {
        if self.hint.is_some() || self.tutorial_active {
            return None;
        }
        let w = &g.world;
        let seen = |b: u32| self.save.hints & (1 << b) != 0;
        let near = |k: EntKind, dx: f32| {
            w.ents.iter().any(|e| {
                e.alive && e.kind == k && (e.x - w.p.x).abs() < dx && (e.y - w.p.y).abs() < 360.0
            })
        };
        let table: [(u32, EntKind, f32, &'static str); 11] = [
            (
                0,
                EntKind::Skitter,
                420.0,
                "SKITTERS: land on their heads to stomp them.",
            ),
            (
                1,
                EntKind::Spitter,
                520.0,
                "SPITTERS aim along a red line: step off it, or stomp them.",
            ),
            (
                2,
                EntKind::Bulwark,
                420.0,
                "BULWARKS are armoured: ram them at speed, or stomp twice.",
            ),
            (
                3,
                EntKind::Saw,
                420.0,
                "SAWBLADES ride their rails: time your jump past them.",
            ),
            (
                4,
                EntKind::Geyser,
                420.0,
                "GEYSERS rumble before they erupt. Wait for the lull.",
            ),
            (
                5,
                EntKind::Wisp,
                420.0,
                "WISPS drift toward you: stomp or ram them.",
            ),
            (
                6,
                EntKind::Strike,
                500.0,
                "LIGHTNING lands where the ring flickers. Move!",
            ),
            (
                7,
                EntKind::Bloom,
                420.0,
                "SPRINGBLOOMS launch you high: look for treasure above.",
            ),
            (
                8,
                EntKind::Wind,
                420.0,
                "GUSTS shove you sideways: pump the rope against them.",
            ),
            (
                12,
                EntKind::Chest,
                500.0,
                "RELIC CHESTS grant an extra boon after the ascent.",
            ),
            (
                13,
                EntKind::EchoOrb,
                500.0,
                "ECHO ORBS refill your rewind meter.",
            ),
        ];
        for (bit, kind, dx, text) in table {
            if !seen(bit) && near(kind, dx) {
                return Some((bit, text));
            }
        }
        if !seen(9)
            && w.level.anchors.iter().enumerate().any(|(i, a)| {
                a.kind == AnchorKind::Brittle
                    && dist(a.x, a.y, w.p.x, w.p.y) < 420.0
                    && w.anchors[i].alive
            })
        {
            return Some((9, "CRACKED ANCHORS give way after a second: keep moving."));
        }
        if !seen(10)
            && w.level
                .anchors
                .iter()
                .any(|a| a.kind == AnchorKind::Rail && dist(a.x, a.y, w.p.x, w.p.y) < 420.0)
        {
            return Some((10, "RAIL ANCHORS slide back and forth: time your hook."));
        }
        if !seen(15)
            && w.level.block_tiles.iter().enumerate().any(|(i, &(x, y))| {
                w.level.tile(x, y) == GLASS
                    && w.blocks[i].solid
                    && dist(x as f32 * T, y as f32 * T, w.p.x, w.p.y) < 520.0
            })
        {
            return Some((
                15,
                "GLASS DOORS only break at speed: swing low, let go, and fly through.",
            ));
        }
        if !seen(11) && w.stats.damage > 0 {
            return Some((
                11,
                "OUCH! Hold R (or right click) to REWIND time and undo it.",
            ));
        }
        if !seen(14)
            && w.level
                .tile((w.p.x / T) as i32, ((w.p.y + HH + 2.0) / T) as i32)
                == CRUMBLE
        {
            return Some((14, "CRUMBLING BRIDGES fall away under you: keep moving."));
        }
        None
    }

    fn footsteps(&mut self, g: &Game, dt: f32) {
        let p = &g.world.p;
        if p.on_ground && p.vx.abs() > 90.0 && !g.rewinding {
            self.step_t += dt * (p.vx.abs() / 235.0);
            if self.step_t > 0.27 {
                self.step_t = 0.0;
                self.play(Sfx::Step);
            }
        } else {
            self.step_t = 0.2;
        }
    }

    fn handle_event(&mut self, e: &Event, g: &Game) {
        let w = &g.world;
        let near = dist(e.x, e.y, w.p.x, w.p.y) < 1000.0;
        self.fx.on_event(e, w.level.biome, &w.p);
        if !near && !matches!(e.kind, Ev::BossHit | Ev::BossDown | Ev::Cleared) {
            return;
        }
        let s = match e.kind {
            Ev::Jump => Some(Sfx::Jump),
            Ev::WallJump => Some(Sfx::WallJump),
            Ev::AirJump => Some(Sfx::AirJump),
            Ev::Land => {
                self.squash = (e.v / 700.0).clamp(0.2, 1.0);
                Some(Sfx::Land(if e.v < 320.0 {
                    0
                } else if e.v < 620.0 {
                    1
                } else {
                    2
                }))
            }
            Ev::HookHit => Some(Sfx::HookHit),
            Ev::HookMiss => Some(Sfx::HookMiss),
            Ev::Release => (e.v > 250.0).then_some(Sfx::Release),
            Ev::RopeSnap => Some(Sfx::RopeSnap),
            Ev::Brittle => Some(Sfx::Brittle),
            Ev::Crystal => Some(Sfx::Crystal(w.stats.chain)),
            Ev::EchoOrb => Some(Sfx::EchoOrb),
            Ev::HeartShard => Some(Sfx::Heart),
            Ev::Chest => Some(Sfx::Chest),
            Ev::Hurt => {
                self.freeze = self.freeze.max(0.07);
                Some(Sfx::Hurt)
            }
            Ev::Stomp => {
                self.freeze = self.freeze.max(0.03);
                Some(Sfx::Stomp)
            }
            Ev::Ram => {
                self.freeze = self.freeze.max(0.06);
                Some(Sfx::Ram)
            }
            Ev::Smash => {
                self.freeze = self.freeze.max(0.05);
                Some(Sfx::Smash)
            }
            Ev::Pit => Some(Sfx::Pit),
            Ev::Checkpoint => Some(Sfx::Checkpoint),
            Ev::SpitterCharge => Some(Sfx::SpitterCharge),
            Ev::SpitterFire => Some(Sfx::SpitterFire),
            Ev::GeyserWarn => Some(Sfx::GeyserWarn),
            Ev::GeyserBurst => Some(Sfx::GeyserBurst),
            Ev::StrikeWarn => Some(Sfx::StrikeWarn),
            Ev::StrikeHit => Some(Sfx::StrikeHit),
            Ev::Bounce => Some(Sfx::Bounce),
            Ev::Crumble => Some(Sfx::Crumble),
            Ev::Cleared => None,
            Ev::Shield => Some(Sfx::Shield),
            Ev::BossHit => {
                self.freeze = self.freeze.max(0.16);
                Some(Sfx::BossHit)
            }
            Ev::BossStun => {
                self.boss_stun_seen = true;
                Some(Sfx::BossStun)
            }
            Ev::BossRoar => Some(Sfx::BossRoar),
            Ev::BossAttack => Some(Sfx::BossAttack),
            Ev::BossDown => Some(Sfx::BossDown),
            Ev::Dying => Some(Sfx::Dying),
            Ev::Revive => Some(Sfx::Revive),
            Ev::ChainBreak => None,
        };
        if let Some(s) = s {
            self.play(s);
        }
    }

    // ------------------------------------------------------------ draw

    fn add_btn(
        &mut self,
        c: &mut Canvas,
        r: Rect,
        label: &str,
        act: Act,
        enabled: bool,
        accent: u32,
    ) {
        let idx = self.hot.len();
        let sel = idx == self.menu_idx;
        button(c, r, label, sel, enabled, accent);
        self.hot.push((r, act, enabled));
    }

    pub fn draw(&mut self, c: &mut Canvas) {
        self.hot.clear();
        c.ox = 0.0;
        c.oy = 0.0;
        match self.screen {
            Screen::Title => self.draw_title(c),
            Screen::Workshop => self.draw_workshop(c),
            Screen::Help => self.draw_help(c),
            Screen::Intro
            | Screen::Playing
            | Screen::Paused
            | Screen::Clear
            | Screen::Draft
            | Screen::Route => {
                self.draw_world(c);
                match self.screen {
                    Screen::Intro => self.draw_intro(c),
                    Screen::Paused => self.draw_pause(c),
                    Screen::Clear => self.draw_clear(c),
                    Screen::Draft => self.draw_draft(c),
                    Screen::Route => self.draw_route(c),
                    _ => {}
                }
            }
            Screen::Victory => {
                self.draw_world(c);
                self.draw_victory(c);
            }
            Screen::Defeat => {
                self.draw_world(c);
                self.draw_defeat(c);
            }
        }
        // toast
        if let Some((t, life)) = &self.hint {
            let a = (*life).min(1.0);
            let wd = (Canvas::text_width(t, 1.8) + 60.0).max(480.0);
            let r = Rect::new(W as f32 / 2.0 - wd / 2.0, 108.0, wd, 44.0);
            panel(c, r, GOLD, 0.88 * a);
            c.text_center(W as f32 / 2.0, 122.0, t, 1.8, TEXT, a);
        }
        if let Some((t, life)) = &self.toast {
            c.text_center(W as f32 / 2.0, 120.0, t, 2.4, GOLD, (*life).min(1.0));
        }
        if self.muted {
            c.text_shadow(
                W as f32 - 100.0,
                H as f32 - 22.0,
                "MUTED [M]",
                1.0,
                MUTED,
                0.8,
            );
        }
        // white flash overlay
        if self.fx.flash > 0.01 {
            c.fill_rect(
                0.0,
                0.0,
                W as f32,
                H as f32,
                self.fx.flash_col,
                self.fx.flash * 0.6,
            );
        }
    }

    fn draw_world(&mut self, c: &mut Canvas) {
        let (Some(game), Some(cache)) = (self.game.as_ref(), self.cache.as_mut()) else {
            c.clear(0x100c20);
            return;
        };
        let w = &game.world;
        let aim = if self.mouse_idle < 3.0 {
            Some((
                self.last_mouse.0 + self.cam.x,
                self.last_mouse.1 + self.cam.y,
            ))
        } else {
            None
        };
        let t = self.anim;
        let info = draw_scene(c, cache, game, &self.cam, &self.fx, t, aim, self.squash);
        let info_target = info.targeted_anchor;
        let _ = w;
        if matches!(
            self.screen,
            Screen::Playing | Screen::Paused | Screen::Intro | Screen::Clear
        ) {
            let run = self.run.as_ref();
            let stage_label = run.map(|r| r.stage_label()).unwrap_or_default();
            let boons: Vec<Boon> = run.map(|r| r.boons.clone()).unwrap_or_default();
            let base = run.map(|r| r.score).unwrap_or(0);
            let score = base + game.world.stats.score;
            draw_hud(
                c,
                game,
                &stage_label,
                game.world.level.biome.name(),
                &boons,
                score,
                self.hint_t,
                t,
            );
            // tutorial prompt
            if self.tutorial_active && self.screen == Screen::Playing {
                if let Some(p) = game
                    .world
                    .level
                    .prompts
                    .iter()
                    .find(|p| game.world.p.x >= p.x0 && game.world.p.x <= p.x1)
                {
                    hint_box(c, p.text, 1.0);
                }
                c.text_shadow(
                    W as f32 - 250.0,
                    H as f32 - 30.0,
                    "TAB: skip training",
                    1.0,
                    MUTED,
                    0.9,
                );
            }
        }
        if self.screen == Screen::Playing {
            let g = self.game.as_ref().unwrap();
            if self.mouse_idle < 6.0 {
                let (mx, my) = self.last_mouse;
                let hot = info_target.is_some();
                let col = if hot { 0xffe066 } else { 0xffffff };
                let r = if hot { 12.0 } else { 9.0 };
                c.ring(mx, my, r, r, 1.6, col, 0.9);
                c.circle(mx, my, 1.6, col, 1.0);
                for k in 0..4 {
                    let a = k as f32 * 1.5708;
                    c.line(
                        mx + a.cos() * (r + 2.0),
                        my + a.sin() * (r + 2.0),
                        mx + a.cos() * (r + 6.0),
                        my + a.sin() * (r + 6.0),
                        1.4,
                        col,
                        0.9,
                    );
                }
            }
            if g.world.outcome == Outcome::Dead {
                c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x300008, 0.45);
                let cx = W as f32 / 2.0;
                c.text_center(cx, 250.0, "YOU HAVE FALLEN", 4.0, 0xff8f9f, 1.0);
                if g.can_rewind() && g.dead_for < 4.4 {
                    c.text_center(
                        cx,
                        320.0,
                        "HOLD [R] OR RIGHT CLICK TO REWIND TIME",
                        2.2,
                        0xc8d8ff,
                        0.7 + 0.3 * (self.anim * 8.0).sin().abs(),
                    );
                    c.text_center(cx, 352.0, "ENTER: give up", 1.6, MUTED, 0.9);
                    let k = (1.0 - g.dead_for / 4.5).clamp(0.0, 1.0);
                    c.round_rect(cx - 150.0, 384.0, 300.0 * k, 8.0, 4.0, 0x8aa8ff, 0.9);
                }
            }
        }
    }

    fn draw_intro(&mut self, c: &mut Canvas) {
        let Some(run) = self.run.as_ref() else { return };
        let Some(g) = self.game.as_ref() else { return };
        let l = &g.world.level;
        let k =
            (self.intro_t / 0.5).min(1.0) * (1.0 - ((self.intro_t - 2.7) / 0.5).clamp(0.0, 1.0));
        c.fill_rect(0.0, 150.0, W as f32, 240.0, 0x080616, 0.78 * k);
        let cx = W as f32 / 2.0;
        let accent = crate::gfx::palette::pal(l.biome).accent;
        let label = if run.training {
            "TRAINING".to_string()
        } else {
            run.stage_label().to_uppercase()
        };
        c.text_center(cx, 176.0, &label, 2.4, lerp_color(MUTED, accent, 0.5), k);
        c.text_center(cx, 214.0, &l.biome.name().to_uppercase(), 6.0, 0xffffff, k);
        c.text_center(cx, 276.0, l.biome.blurb(), 2.0, TEXT, k);
        let news = match (run.training, run.stage) {
            (true, _) => "Learn the ropes. Press TAB to skip.",
            (_, 0) => "Skitters, spikes, crumbling bridges, springblooms",
            (_, 1) => "Spitters, wisps, Bulwarks, brittle anchors - light your way",
            (_, 2) => "Slippery ice, crosswinds, sawblades",
            (_, 3) => "Geysers, rail anchors, molten pits",
            (_, 4) => "Lightning and updrafts - everything at once",
            _ => "Stun the Stormheart, then smash it at speed",
        };
        c.text_center(cx, 316.0, &format!("NEW: {news}"), 1.8, accent, k);
        let mut tags = format!("GOLD PAR {}", mmss(l.par));
        if run.route == Route::Daring && run.stage > 0 && run.stage < 5 {
            tags.push_str("   DARING ROUTE: crystals x1.5, extra boon");
        }
        if run.heat > 0 {
            tags.push_str(&format!("   HEAT {}", run.heat));
        }
        c.text_center(cx, 352.0, &tags, 1.6, MUTED, k);
        c.text_center(
            cx,
            376.0 - 4.0,
            "click or press any key",
            1.2,
            MUTED,
            k * (0.5 + 0.5 * (self.anim * 4.0).sin().abs()),
        );
    }

    fn toggles(&mut self, c: &mut Canvas, x: f32, y: f32, with_heat: bool) {
        let mut yy = y;
        let w = 290.0;
        let h = 34.0;
        let assist = if self.save.aim_assist {
            "AIM ASSIST: ON"
        } else {
            "AIM ASSIST: OFF"
        };
        self.add_btn(
            c,
            Rect::new(x, yy, w, h),
            assist,
            Act::ToggleAssist,
            true,
            EDGE,
        );
        yy += h + 8.0;
        let spd = ["GAME SPEED: 100%", "GAME SPEED: 85%", "GAME SPEED: 70%"]
            [self.save.speed as usize % 3];
        self.add_btn(c, Rect::new(x, yy, w, h), spd, Act::CycleSpeed, true, EDGE);
        yy += h + 8.0;
        let ex = if self.save.explorer {
            "EXPLORER MODE: ON"
        } else {
            "EXPLORER MODE: OFF"
        };
        self.add_btn(
            c,
            Rect::new(x, yy, w, h),
            ex,
            Act::ToggleExplorer,
            true,
            if self.save.explorer { 0x9be07a } else { EDGE },
        );
        yy += h + 8.0;
        let mt = if self.muted {
            "SOUND: OFF"
        } else {
            "SOUND: ON"
        };
        self.add_btn(c, Rect::new(x, yy, w, h), mt, Act::ToggleMute, true, EDGE);
        if with_heat {
            yy += h + 8.0;
            let lbl = if self.save.heat_unlocked == 0 {
                "HEAT: WIN TO UNLOCK".to_string()
            } else {
                format!("HEAT {} < >", self.save.heat)
            };
            let act = if self.save.heat >= self.save.heat_unlocked {
                Act::HeatDown
            } else {
                Act::HeatUp
            };
            self.add_btn(
                c,
                Rect::new(x, yy, w, h),
                &lbl,
                act,
                self.save.heat_unlocked > 0,
                0xff9a52,
            );
        }
    }

    fn draw_pause(&mut self, c: &mut Canvas) {
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.72);
        let cx = W as f32 / 2.0;
        c.text_center(cx, 90.0, "PAUSED", 6.0, TEXT, 1.0);
        let mut y = 170.0;
        for (label, act) in [
            ("RESUME", Act::Resume),
            ("HELP & CONTROLS", Act::Help),
            ("RESTART RUN", Act::Restart),
            ("QUIT TO TITLE", Act::ToTitle),
        ] {
            self.add_btn(
                c,
                Rect::new(cx - 160.0, y, 320.0, 46.0),
                label,
                act,
                true,
                GOLD,
            );
            y += 58.0;
        }
        c.text_center(cx, y + 14.0, "ASSISTS", 2.0, MUTED, 1.0);
        self.toggles(c, cx - 145.0, y + 40.0, false);
    }

    fn draw_clear(&mut self, c: &mut Canvas) {
        let Some(res) = self.clear.clone() else {
            return;
        };
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.6);
        let cx = W as f32 / 2.0;
        let r = Rect::new(cx - 330.0, 90.0, 660.0, 480.0);
        panel(c, r, GOLD, 0.94);
        let training = self.run.as_ref().is_some_and(|r| r.training) && self.tutorial_active;
        c.text_center(
            cx,
            114.0,
            if training {
                "TRAINING COMPLETE"
            } else {
                "ASCENT CLEARED"
            },
            4.0,
            GOLD,
            1.0,
        );
        // medal drops in
        let k = (self.clear_t / 0.6).clamp(0.0, 1.0);
        let my = 200.0 - (1.0 - k) * 60.0;
        draw_medal(c, cx, my, res.medal, 2.2 * (0.5 + 0.5 * k));
        c.text_center(
            cx,
            262.0,
            &format!("{} MEDAL", res.medal.name().to_uppercase()),
            2.4,
            medal_color(res.medal),
            k,
        );
        let lines = [
            format!("TIME      {}   (PAR {})", mmss(res.time), mmss(res.par)),
            format!("CRYSTALS  {}", res.crystals),
            format!("HITS      {}      FALLS {}", res.damage, res.falls),
            format!("SMASHES   {}      RAMS  {}", res.smashes, res.rams),
            format!(
                "BONUS     +{}{}",
                res.bonus,
                if res.damage == 0 {
                    "  (FLAWLESS +200)"
                } else {
                    ""
                }
            ),
            format!("STAGE     {}", res.stage_score),
        ];
        for (i, l) in lines.iter().enumerate() {
            let a = ((self.clear_t - 0.4 - i as f32 * 0.12) * 4.0).clamp(0.0, 1.0);
            c.text_shadow(
                cx - 220.0,
                300.0 + i as f32 * 30.0,
                l,
                2.0,
                if i == 5 { GOLD } else { TEXT },
                a,
            );
        }
        if res.new_best_time {
            c.text_center(
                cx,
                490.0,
                "NEW BEST TIME!",
                2.4,
                0x9be07a,
                0.6 + 0.4 * (self.anim * 6.0).sin().abs(),
            );
        }
        self.add_btn(
            c,
            Rect::new(cx - 130.0, 520.0, 260.0, 44.0),
            "CONTINUE",
            Act::Continue,
            self.clear_t > 0.8,
            GOLD,
        );
    }

    fn draw_draft(&mut self, c: &mut Canvas) {
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.72);
        let cx = W as f32 / 2.0;
        let taken = self.picks_total - self.run.as_ref().map(|r| r.pending_picks).unwrap_or(0);
        let title = if self.picks_total > 1 {
            format!("CHOOSE A BOON ({} OF {})", taken + 1, self.picks_total)
        } else {
            "CHOOSE A BOON".to_string()
        };
        c.text_center(cx, 70.0, &title, 4.0, GOLD, 1.0);
        c.text_center(cx, 118.0, "Boons last for the whole run", 1.8, MUTED, 1.0);
        let n = self.draft.len();
        let cw = 300.0;
        let gap = 34.0;
        let total = n as f32 * cw + (n as f32 - 1.0) * gap;
        let x0 = cx - total / 2.0;
        for i in 0..n {
            let b = self.draft[i];
            let r = Rect::new(x0 + i as f32 * (cw + gap), 160.0, cw, 360.0);
            let sel = self.menu_idx == i;
            let idx = self.hot.len();
            self.hot.push((r, Act::Pick(i), true));
            let _ = idx;
            c.glow(
                r.x + cw / 2.0,
                r.y + 100.0,
                150.0,
                b.color(),
                if sel { 0.5 } else { 0.18 },
            );
            panel(c, r, if sel { b.color() } else { EDGE }, 0.95);
            boon_icon(c, r.x + cw / 2.0, r.y + 90.0, b, 44.0);
            c.text_center(
                r.x + cw / 2.0,
                r.y + 170.0,
                b.name(),
                2.4,
                if sel { 0xffffff } else { TEXT },
                1.0,
            );
            let _ = c.text_wrap(
                r.x + 24.0,
                r.y + 214.0,
                cw - 48.0,
                b.desc(),
                1.8,
                MUTED,
                1.0,
            );
            c.text_center(
                r.x + cw / 2.0,
                r.y + 320.0,
                &format!("[{}]", i + 1),
                2.4,
                GOLD,
                1.0,
            );
        }
        if let Some(run) = &self.run {
            c.text_center(cx, 548.0, "OWNED", 1.4, MUTED, 1.0);
            for (i, b) in run.boons.iter().enumerate() {
                boon_icon(
                    c,
                    cx - (run.boons.len() as f32 - 1.0) * 17.0 + i as f32 * 34.0,
                    580.0,
                    *b,
                    13.0,
                );
            }
        }
    }

    fn draw_route(&mut self, c: &mut Canvas) {
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.72);
        let cx = W as f32 / 2.0;
        let Some(run) = self.run.as_ref() else { return };
        let next = Biome::RUN[run.stage.min(4)];
        c.text_center(cx, 60.0, "CHOOSE YOUR ROUTE", 4.0, GOLD, 1.0);
        c.text_center(
            cx,
            108.0,
            &format!(
                "Next: ASCENT {} - {}",
                run.stage + 1,
                next.name().to_uppercase()
            ),
            2.2,
            TEXT,
            1.0,
        );
        c.text_center(cx, 136.0, next.blurb(), 1.8, MUTED, 1.0);
        let heals = run.hp < run.mods.max_hp;
        let opts = [
            (
                Route::Steady,
                "STEADY ROUTE",
                0x7bd0ff,
                if heals {
                    "A gentler climb.|Fewer hazards.|Recover 1 heart before you go."
                } else {
                    "A gentler climb.|Fewer hazards.|(You are at full health.)"
                },
            ),
            (
                Route::Daring,
                "DARING ROUTE",
                0xff8a50,
                "A harder climb.|More hazards.|Crystals worth x1.5.|Choose an EXTRA boon after.",
            ),
        ];
        let cw = 360.0;
        let gap = 50.0;
        let x0 = cx - cw - gap / 2.0;
        for (i, (route, name, col, desc)) in opts.iter().enumerate() {
            let r = Rect::new(x0 + i as f32 * (cw + gap), 190.0, cw, 330.0);
            let sel = self.menu_idx == i;
            self.hot.push((r, Act::PickRoute(*route), true));
            c.glow(
                r.x + cw / 2.0,
                r.y + 80.0,
                160.0,
                *col,
                if sel { 0.45 } else { 0.15 },
            );
            panel(c, r, if sel { *col } else { EDGE }, 0.95);
            c.text_center(
                r.x + cw / 2.0,
                r.y + 30.0,
                name,
                3.0,
                if sel { 0xffffff } else { *col },
                1.0,
            );
            // difficulty pips
            let pips = if *route == Route::Steady { 2 } else { 4 };
            for k in 0..5 {
                let px = r.x + cw / 2.0 - 52.0 + k as f32 * 26.0;
                c.circle(
                    px,
                    r.y + 84.0,
                    8.0,
                    if k < pips { *col } else { 0x2a2444 },
                    1.0,
                );
            }
            for (k, line) in desc.split('|').enumerate() {
                c.text_center(
                    r.x + cw / 2.0,
                    r.y + 128.0 + k as f32 * 34.0,
                    line,
                    1.8,
                    TEXT,
                    1.0,
                );
            }
            c.text_center(
                r.x + cw / 2.0,
                r.y + 292.0,
                &format!("[{}]", i + 1),
                2.4,
                GOLD,
                1.0,
            );
        }
    }

    fn draw_victory(&mut self, c: &mut Canvas) {
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.7);
        let cx = W as f32 / 2.0;
        let Some(run) = self.run.as_ref() else { return };
        let r = Rect::new(cx - 340.0, 70.0, 680.0, 540.0);
        panel(c, r, GOLD, 0.95);
        c.glow(cx, 140.0, 300.0, 0xffe9a0, 0.3);
        c.text_center(cx, 100.0, "THE BEACON BURNS", 5.0, GOLD, 1.0);
        c.text_center(
            cx,
            150.0,
            "The Stormheart is shattered. Light returns to the Cragspire.",
            1.6,
            TEXT,
            1.0,
        );
        let lines = [
            format!("FINAL SCORE   {}", run.final_score()),
            format!("TOTAL TIME    {}", mmss(run.total_time)),
            format!("CRYSTALS      {}", run.crystals),
            format!("GLIMMER BANKED +{}", run.glimmer_earned()),
        ];
        for (i, l) in lines.iter().enumerate() {
            c.text_shadow(
                cx - 230.0,
                200.0 + i as f32 * 34.0,
                l,
                2.2,
                if i == 0 { GOLD } else { TEXT },
                1.0,
            );
        }
        // medals
        for (i, res) in run.results.iter().enumerate() {
            draw_medal(
                c,
                cx - (run.results.len() as f32 - 1.0) * 34.0 + i as f32 * 68.0,
                380.0,
                res.medal,
                1.0,
            );
        }
        for (i, b) in run.boons.iter().enumerate() {
            boon_icon(
                c,
                cx - (run.boons.len() as f32 - 1.0) * 17.0 + i as f32 * 34.0,
                450.0,
                *b,
                13.0,
            );
        }
        if self.new_best_score {
            c.text_center(
                cx,
                486.0,
                "NEW BEST SCORE!",
                2.4,
                0x9be07a,
                0.6 + 0.4 * (self.anim * 6.0).sin().abs(),
            );
        }
        if run.explorer {
            c.text_center(
                cx,
                486.0,
                "EXPLORER MODE - RESULT NOT RECORDED",
                1.6,
                MUTED,
                1.0,
            );
        }
        if self.save.heat_unlocked > 0 {
            c.text_center(
                cx,
                512.0,
                &format!(
                    "HEAT {} UNLOCKED - HARDER ASCENTS AWAIT",
                    self.save.heat_unlocked
                ),
                1.6,
                0xff9a52,
                1.0,
            );
        }
        self.add_btn(
            c,
            Rect::new(cx - 130.0, 540.0, 260.0, 46.0),
            "CONTINUE",
            Act::Continue,
            true,
            GOLD,
        );
    }

    fn draw_defeat(&mut self, c: &mut Canvas) {
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x180410, 0.8);
        let cx = W as f32 / 2.0;
        let Some(run) = self.run.as_ref() else { return };
        let r = Rect::new(cx - 330.0, 110.0, 660.0, 430.0);
        panel(c, r, 0xff6f7f, 0.95);
        c.text_center(cx, 140.0, "THE CLIMB ENDS HERE", 4.0, 0xff8f9f, 1.0);
        c.text_center(cx, 196.0, &self.end_note, 2.2, TEXT, 1.0);
        let reached = if run.training {
            "Training".to_string()
        } else if run.stage >= 5 {
            "The Beacon".to_string()
        } else {
            format!(
                "Ascent {} - {}",
                run.stage + 1,
                Biome::RUN[run.stage].name()
            )
        };
        let score = run.score + self.game.as_ref().map(|g| g.world.stats.score).unwrap_or(0);
        let lines = [
            format!("REACHED     {reached}"),
            format!("SCORE       {score}"),
            format!("BEST SCORE  {}", self.save.best_score),
            format!(
                "GLIMMER     +{} banked",
                run.crystals
                    + self
                        .game
                        .as_ref()
                        .map(|g| g.world.stats.crystals)
                        .unwrap_or(0)
            ),
        ];
        for (i, l) in lines.iter().enumerate() {
            c.text_shadow(cx - 250.0, 250.0 + i as f32 * 34.0, l, 2.0, TEXT, 1.0);
        }
        if self.new_best_score {
            c.text_center(cx, 400.0, "NEW BEST SCORE!", 2.4, 0x9be07a, 1.0);
        }
        c.text_center(
            cx,
            432.0,
            "Spend glimmer in the Workshop for permanent perks.",
            1.6,
            MUTED,
            1.0,
        );
        self.add_btn(
            c,
            Rect::new(cx - 130.0, 470.0, 260.0, 46.0),
            "CONTINUE",
            Act::Continue,
            true,
            0xff6f7f,
        );
    }

    fn draw_help(&mut self, c: &mut Canvas) {
        let t = self.anim;
        if let Some(g) = &self.game {
            let _ = g;
        }
        crate::gfx::bg::draw_background(
            c,
            Biome::Mossbluff,
            &crate::gfx::palette::pal(Biome::Mossbluff),
            t * 40.0,
            0.0,
            t,
        );
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.7);
        let cx = W as f32 / 2.0;
        panel(c, Rect::new(cx - 520.0, 40.0, 1040.0, 640.0), GOLD, 0.95);
        c.text_center(cx, 62.0, "HOW TO CLIMB", 4.0, GOLD, 1.0);
        let left = [
            ("A / D  or  ARROWS", "run"),
            ("SPACE / K / Z", "jump (hold for height)"),
            ("PRESS INTO WALL + JUMP", "wall slide and wall jump"),
            (
                "HOLD LEFT CLICK",
                "hook the glowing anchor under your cursor",
            ),
            ("HOLD SHIFT / X / J", "hook without a mouse (auto-aim)"),
            ("A / D while hooked", "pump the swing for speed"),
            ("W / S while hooked", "reel the rope in or out"),
            ("RELEASE or JUMP", "let go and fly"),
            ("HOLD R / RIGHT CLICK", "REWIND time (uses Echo)"),
            ("ESC / P", "pause and assists"),
            ("M", "mute"),
            ("FULL SCREEN", "green window button or Ctrl+Cmd+F"),
        ];
        for (i, (k, d)) in left.iter().enumerate() {
            c.text_shadow(cx - 490.0, 120.0 + i as f32 * 30.0, k, 1.8, GOLD, 1.0);
            c.text_shadow(cx - 170.0, 120.0 + i as f32 * 30.0, d, 1.8, TEXT, 1.0);
        }
        c.text_shadow(cx - 490.0, 470.0, "TIPS", 2.4, 0x9df4ff, 1.0);
        let tips = [
            "MOMENTUM IS A WEAPON. Fast enough, you shatter glass, ram armoured foes and crack the Stormheart.",
            "Land on a foe's head to stomp it. Bulwarks only fall to a fast ram.",
            "Swing low and let go at the bottom of the arc: that is where you are fastest.",
            "Rewind rescues you from a bad hit, a fall, even a fatal blow. Checkpoints and orbs refill Echo.",
            "Combos grow with every crystal, hook and kill. Do not touch the ground to keep them hot.",
        ];
        for (i, t) in tips.iter().enumerate() {
            let _ = c.text_wrap(
                cx - 490.0,
                504.0 + i as f32 * 34.0,
                980.0,
                t,
                1.5,
                TEXT,
                1.0,
            );
        }
        c.text_center(
            cx,
            658.0,
            "press any key to return",
            1.4,
            MUTED,
            0.6 + 0.4 * (t * 4.0).sin().abs(),
        );
    }

    fn draw_workshop(&mut self, c: &mut Canvas) {
        let t = self.anim;
        crate::gfx::bg::draw_background(
            c,
            Biome::Glimmerdeep,
            &crate::gfx::palette::pal(Biome::Glimmerdeep),
            t * 30.0,
            0.0,
            t,
        );
        c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x04030c, 0.5);
        let cx = W as f32 / 2.0;
        c.text_center(cx, 40.0, "THE WORKSHOP", 5.0, GOLD, 1.0);
        c.text_center(
            cx,
            90.0,
            "Spend glimmer on perks that last forever",
            1.8,
            MUTED,
            1.0,
        );
        c.text_shadow(
            W as f32 - 280.0,
            44.0,
            &format!("GLIMMER {}", self.save.glimmer),
            2.4,
            0x9df4ff,
            1.0,
        );
        for i in 0..PERKS.len() {
            let def = &PERKS[i];
            let y = 130.0 + i as f32 * 88.0;
            let r = Rect::new(cx - 450.0, y, 900.0, 76.0);
            panel(c, r, EDGE, 0.92);
            c.text_shadow(r.x + 20.0, y + 14.0, def.name, 2.4, TEXT, 1.0);
            c.text_shadow(r.x + 20.0, y + 46.0, def.desc, 1.6, MUTED, 1.0);
            let lvl = self.save.perks[i] as usize;
            for k in 0..def.costs.len() {
                c.circle(
                    r.x + 520.0 + k as f32 * 26.0,
                    y + 38.0,
                    9.0,
                    if k < lvl { GOLD } else { 0x2a2444 },
                    1.0,
                );
            }
            let maxed = lvl >= def.costs.len();
            let cost = if maxed { 0 } else { def.costs[lvl] };
            let label = if maxed {
                "MAXED".to_string()
            } else {
                format!("BUY {cost}")
            };
            self.add_btn(
                c,
                Rect::new(r.x + r.w - 190.0, y + 16.0, 170.0, 44.0),
                &label,
                Act::Buy(i),
                !maxed && self.save.glimmer >= cost,
                GOLD,
            );
        }
        self.add_btn(
            c,
            Rect::new(cx - 110.0, 590.0, 220.0, 46.0),
            "BACK",
            Act::Back,
            true,
            EDGE,
        );
    }

    fn draw_title(&mut self, c: &mut Canvas) {
        let t = self.anim;
        let pal = crate::gfx::palette::pal(Biome::Mossbluff);
        crate::gfx::bg::draw_background(c, Biome::Mossbluff, &pal, t * 60.0, 120.0, t);
        // foreground cliffs
        for i in 0..3 {
            let x = 40.0 + i as f32 * 20.0;
            c.poly(
                &[
                    (0.0, H as f32),
                    (0.0, 520.0 - i as f32 * 30.0),
                    (x + 160.0, 560.0 - i as f32 * 30.0),
                    (x + 300.0, H as f32),
                ],
                lerp_color(0x2a4a3a, 0x16281e, i as f32 / 2.0),
                1.0,
            );
        }
        // swinging Wren on a rope from an anchor above the logo
        let ax = 930.0;
        let ay = 40.0;
        let len = 360.0;
        let ang = (t * 1.25).sin() * 0.95;
        let wx = ax + ang.sin() * len;
        let wy = ay + ang.cos() * len;
        let vx = (t * 1.25).cos() * 1.25 * 0.95 * len * ang.cos();
        let vy = -(ang.sin()) * (t * 1.25).cos() * 1.25 * 0.95 * len;
        c.glow(ax, ay, 60.0, 0xf0b85a, 0.7);
        c.ring(ax, ay, 12.0, 12.0, 3.4, 0xf0b85a, 1.0);
        c.line(ax, ay, wx, wy - 4.0, 3.0, 0x3a2a1a, 1.0);
        c.line(ax, ay, wx, wy - 4.0, 1.4, 0xe8d0a0, 1.0);
        let facing = if vx >= 0.0 { 1.0 } else { -1.0 };
        self.title_fx
            .update_scarf((wx - facing, wy - 6.0), vx, vy, facing, 1.0 / 60.0);
        let pose = WrenPose {
            x: wx,
            y: wy,
            facing,
            vx,
            vy,
            on_ground: false,
            wall: 0,
            run_t: t * 10.0,
            anchor: Some((ax, ay)),
            invuln: 0.0,
            hurt_t: 0.0,
            charged: vx.abs() > 440.0,
            dying: 0.0,
            time: t,
            shield: false,
            landed_squash: 0.0,
        };
        let scarf = self.title_fx.scarf;
        draw_wren(c, &pose, &scarf);
        self.fx
            .update_weather(Biome::Mossbluff, 0.0, 0.0, 0.0, 1.0 / 60.0);
        self.fx.draw_weather(c);
        // logo
        let lx = 70.0;
        c.text_shadow(lx + 4.0, 92.0, "CRAGSWING", 11.0, 0x1a2a4a, 0.5);
        for (i, ch) in "CRAGSWING".chars().enumerate() {
            let x = lx + i as f32 * 7.0 * 11.0;
            let col = lerp_color(0xffe9a0, 0xff9a52, i as f32 / 8.0);
            c.text(x + 3.0, 94.0, &ch.to_string(), 11.0, 0x000000, 0.45);
            c.text(
                x,
                90.0 + ((t * 2.0 + i as f32 * 0.5).sin() * 2.0),
                &ch.to_string(),
                11.0,
                col,
                1.0,
            );
        }
        c.text_shadow(
            lx + 6.0,
            190.0,
            "a rope-swinging climb to relight the Beacon",
            2.0,
            0xffffff,
            1.0,
        );
        // menu
        let mx = lx;
        let mut y = 250.0;
        let first = !self.save.tutorial_done;
        self.add_btn(
            c,
            Rect::new(mx, y, 330.0, 50.0),
            if first { "BEGIN (TRAINING)" } else { "PLAY" },
            Act::Play,
            true,
            GOLD,
        );
        y += 62.0;
        if !first {
            self.add_btn(
                c,
                Rect::new(mx, y, 330.0, 44.0),
                "TRAINING [TAB]",
                Act::Training,
                true,
                EDGE,
            );
            y += 54.0;
        }
        self.add_btn(
            c,
            Rect::new(mx, y, 330.0, 44.0),
            "WORKSHOP",
            Act::Workshop,
            true,
            EDGE,
        );
        y += 54.0;
        self.add_btn(
            c,
            Rect::new(mx, y, 330.0, 44.0),
            "HOW TO PLAY [H]",
            Act::Help,
            true,
            EDGE,
        );
        y += 54.0;
        self.add_btn(
            c,
            Rect::new(mx, y, 330.0, 44.0),
            "QUIT",
            Act::Quit,
            true,
            EDGE,
        );
        // options column
        c.text_shadow(mx + 380.0, 252.0, "ASSISTS", 1.6, MUTED, 1.0);
        self.toggles(c, mx + 380.0, 276.0, true);
        // records
        let s = &self.save;
        let rec = format!(
            "BEST {}   WINS {}   RUNS {}   GLIMMER {}",
            s.best_score, s.wins, s.runs, s.glimmer
        );
        c.text_shadow(lx + 6.0, H as f32 - 48.0, &rec, 1.6, TEXT, 0.95);
        let best: Vec<String> = s
            .best_ms
            .iter()
            .take(5)
            .map(|m| {
                if *m == 0 {
                    "--".to_string()
                } else {
                    mmss(*m as f32 / 1000.0)
                }
            })
            .collect();
        c.text_shadow(
            lx + 6.0,
            H as f32 - 26.0,
            &format!("BEST TIMES  {}", best.join("  ")),
            1.3,
            MUTED,
            0.95,
        );
    }
}
