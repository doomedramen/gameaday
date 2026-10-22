//! The application: screens, input handling, tutorial, run flow and music
//! state. Engine-free - `main.rs` feeds it input and presents its canvas.

use std::path::PathBuf;

use crate::ai;
use crate::audio::{MusicState, Sfx};
use crate::battle::{Battle, Phase};
use crate::gfx::canvas::{Canvas, H, W};
use crate::gfx::iso;
use crate::gfx::palette::{Palette, palette};
use crate::gfx::scene::{self, Overlay};
use crate::gfx::sky::Sky;
use crate::gfx::sprites;
use crate::gfx::ui;
use crate::gfx::view::View;
use crate::map::{Biome, Map, Pos, Terrain};
use crate::run::{Run, Stage};
use crate::save::Save;
use crate::units::{Ability, Kind, Target, Team};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Num1,
    Num2,
    Num3,
    Space,
    Enter,
    Escape,
    Tab,
    U,
    D,
    H,
    M,
}

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub mouse: (f32, f32),
    pub click: bool,
    pub right_click: bool,
    pub keys: Vec<Key>,
}

impl Input {
    pub fn key(&self, k: Key) -> bool {
        self.keys.contains(&k)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Game,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Move,
    Ability(Ability),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Unit(u32),
    Ability(usize),
    EndTurn,
    Undo,
    Danger,
    Help,
    Draft(usize),
    Route(usize),
    NewRun,
    HowTo,
    Quit,
    Continue,
}

pub const TUTORIAL: [&str; 6] = [
    "Welcome, lantern-bearer. Click a squad member (or press TAB) to select them.",
    "Blue tiles show where they can move. Click one to move there. Misclicked? Press U to undo.",
    "Now act: press 1-3 (or click an ability below), then click a highlighted target.",
    "Foes standing in DARKNESS are SHROUDED - they take half damage. Lanterns, braziers and flares light the way.",
    "Shove foes into the void or water to destroy them outright. Press SPACE (or End Turn) when you're done.",
    "The dark answers. Watch where it strikes - hover any foe to see its reach, or press D for all danger.",
];

pub struct App {
    pub screen: Screen,
    pub run: Option<Run>,
    /// The finished battle, kept as a backdrop for draft/route screens.
    pub backdrop: Option<Battle>,
    pub view: View,
    pub save: Save,
    pub save_path: Option<PathBuf>,
    pub selected: Option<u32>,
    pub mode: Mode,
    pub mouse: (f32, f32),
    pub hover_tile: Option<Pos>,
    pub hover_unit: Option<u32>,
    pub hover_button: Option<Button>,
    pub show_danger: bool,
    pub help: bool,
    pub time: f32,
    pub outcome_timer: f32,
    pub pal: Palette,
    sky: Sky,
    sky_key: (Biome, u64),
    vig: Vec<f32>,
    pub title_map: Map,
    pub title_glow: Vec<f32>,
    pub tutorial: Option<usize>,
    tutorial_timer: f32,
    pub new_best: bool,
    pub quit: bool,
    pub sfx: Vec<Sfx>,
    pub muted: bool,
    seed_counter: u64,
    pub last_error: Option<(String, f32)>,
}

impl App {
    pub fn new(save_path: Option<PathBuf>, seed: u64) -> App {
        let save = save_path
            .as_ref()
            .map(|p| Save::load(p))
            .unwrap_or_default();
        let title_map = Map::generate(seed ^ 0x7171, Biome::Crystal, 4);
        let mut title_glow = vec![0.0; title_map.tiles.len()];
        // Light the title island from its braziers.
        for p in Map::all_pos() {
            let mut g: f32 = 0.0;
            for b in title_map.braziers() {
                let d = ((b.x - p.x).pow(2) + (b.y - p.y).pow(2)) as f32;
                g = g.max(1.0 - d.sqrt() / 4.5);
            }
            title_glow[Map::idx(p)] = g.max(0.0);
        }
        let mut tm = title_map.clone();
        let bs: Vec<Pos> = tm.braziers().collect();
        for b in bs {
            tm.tile_mut(b).lit = true;
        }
        let pal = palette(Biome::Crystal);
        App {
            screen: Screen::Title,
            run: None,
            backdrop: None,
            view: View::new(),
            save,
            save_path,
            selected: None,
            mode: Mode::Move,
            mouse: (0.0, 0.0),
            hover_tile: None,
            hover_unit: None,
            hover_button: None,
            show_danger: false,
            help: false,
            time: 0.0,
            outcome_timer: 0.0,
            pal,
            sky: Sky::new(&pal, 1),
            sky_key: (Biome::Crystal, 1),
            vig: scene::vignette(W, H),
            title_map: tm,
            title_glow,
            tutorial: None,
            tutorial_timer: 0.0,
            new_best: false,
            quit: false,
            sfx: vec![],
            muted: false,
            seed_counter: seed,
            last_error: None,
        }
    }

    pub fn battle(&self) -> Option<&Battle> {
        self.run.as_ref().and_then(|r| r.battle.as_ref())
    }

    fn battle_mut(&mut self) -> Option<&mut Battle> {
        self.run.as_mut().and_then(|r| r.battle.as_mut())
    }

    pub fn take_sfx(&mut self) -> Vec<Sfx> {
        let mut out = std::mem::take(&mut self.sfx);
        out.extend(self.view.take_sfx());
        out
    }

    pub fn music(&self) -> MusicState {
        match self.screen {
            Screen::Title => MusicState {
                intensity: 0.15,
                boss: false,
            },
            Screen::End => MusicState {
                intensity: 0.1,
                boss: false,
            },
            Screen::Game => match self.battle() {
                Some(b) => {
                    let boss = b.cfg.boss && b.outcome.is_none();
                    let base = if b.phase == Phase::Enemy { 0.8 } else { 0.55 };
                    MusicState {
                        intensity: if boss { 1.0 } else { base },
                        boss,
                    }
                }
                None => MusicState {
                    intensity: 0.3,
                    boss: false,
                },
            },
        }
    }

    pub fn start_run(&mut self) {
        self.seed_counter = self
            .seed_counter
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407 ^ (self.time * 1000.0) as u64);
        let run = Run::new(self.seed_counter);
        self.run = Some(run);
        self.backdrop = None;
        self.screen = Screen::Game;
        self.new_best = false;
        self.tutorial = if self.save.tutorial_done {
            None
        } else {
            Some(0)
        };
        self.tutorial_timer = 0.0;
        self.enter_battle();
        self.sfx.push(Sfx::Click);
    }

    fn enter_battle(&mut self) {
        self.selected = None;
        self.mode = Mode::Move;
        self.outcome_timer = 0.0;
        self.show_danger = false;
        let Some(b) = self.battle() else {
            return;
        };
        let biome = b.cfg.biome;
        let seed = b.cfg.seed;
        let island = b.cfg.island;
        let boss = b.cfg.boss;
        let elite = b.cfg.elite;
        self.pal = palette(biome);
        if self.sky_key != (biome, seed) {
            self.sky = Sky::new(&self.pal, seed);
            self.sky_key = (biome, seed);
        }
        let b = self
            .run
            .as_ref()
            .and_then(|r| r.battle.as_ref())
            .expect("battle");
        self.view.reset(b);
        let sub = if boss {
            "Strip the King's shroud with flares and braziers".to_string()
        } else if elite {
            format!("Island {island} of {} - Elite Hunt", crate::run::ISLANDS)
        } else {
            format!("Island {island} of {}", crate::run::ISLANDS)
        };
        self.view.banner(
            biome.name(),
            &sub,
            if boss { 0xff5060 } else { 0xffe6a0 },
            2.6,
        );
        self.sfx.push(Sfx::Banner);
        self.select_next();
    }

    fn select_next(&mut self) {
        let Some(b) = self.battle() else {
            return;
        };
        if b.phase != Phase::Player {
            self.selected = None;
            return;
        }
        let ready: Vec<u32> = b
            .alive(Team::Squad)
            .filter(|u| !(u.moved && u.acted))
            .map(|u| u.id)
            .collect();
        if ready.is_empty() {
            self.selected = None;
            return;
        }
        let next = match self.selected {
            Some(cur) => ready
                .iter()
                .copied()
                .find(|id| *id > cur)
                .unwrap_or(ready[0]),
            None => ready[0],
        };
        self.selected = Some(next);
        self.mode = Mode::Move;
    }

    fn select(&mut self, id: u32) {
        if self.selected != Some(id) {
            self.sfx.push(Sfx::Select);
        }
        self.selected = Some(id);
        self.mode = Mode::Move;
        if self.tutorial == Some(0) {
            self.advance_tutorial(1);
        }
    }

    fn advance_tutorial(&mut self, to: usize) {
        if let Some(t) = self.tutorial
            && to > t
        {
            self.tutorial = Some(to);
            self.tutorial_timer = 0.0;
        }
    }

    fn finish_tutorial(&mut self) {
        if self.tutorial.is_some() {
            self.tutorial = None;
            self.save.tutorial_done = true;
            self.persist();
        }
    }

    fn persist(&self) {
        if let Some(p) = &self.save_path {
            let _ = self.save.store(p);
        }
    }

    fn error(&mut self, msg: &str) {
        self.last_error = Some((msg.to_string(), 1.5));
        self.sfx.push(Sfx::Error);
    }

    pub fn player_can_act(&self) -> bool {
        self.screen == Screen::Game
            && !self.help
            && self
                .battle()
                .is_some_and(|b| b.phase == Phase::Player && b.outcome.is_none())
            && !self.view.busy()
    }

    // ----------------------------------------------------------------- update

    pub fn update(&mut self, dt: f32, input: &Input) {
        self.time += dt;
        self.mouse = input.mouse;
        if let Some((_, t)) = self.last_error.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.last_error = None;
            }
        }
        if input.key(Key::M) {
            self.muted = !self.muted;
        }
        let buttons = ui::buttons(self);
        self.hover_button = buttons
            .iter()
            .find(|(_, r)| r.contains(input.mouse))
            .map(|(b, _)| *b);
        if self.help {
            if input.click || input.key(Key::Escape) || input.key(Key::H) || input.key(Key::Enter) {
                self.help = false;
                self.sfx.push(Sfx::Click);
            }
            self.tick_world(dt);
            return;
        }
        match self.screen {
            Screen::Title => self.update_title(input),
            Screen::Game => self.update_game(dt, input),
            Screen::End => {
                if input.key(Key::Enter)
                    || input.key(Key::Space)
                    || (input.click && self.hover_button == Some(Button::Continue))
                {
                    self.screen = Screen::Title;
                    self.run = None;
                    self.backdrop = None;
                    self.sfx.push(Sfx::Click);
                }
            }
        }
        self.tick_world(dt);
    }

    fn tick_world(&mut self, dt: f32) {
        let pal = self.pal;
        if let Some(b) = self.run.as_ref().and_then(|r| r.battle.as_ref()) {
            self.view.update(dt, b, &pal);
        } else if let Some(b) = self.backdrop.as_ref() {
            self.view.update(dt, b, &pal);
        } else {
            self.view.time += dt;
        }
    }

    fn update_title(&mut self, input: &Input) {
        if input.key(Key::Enter)
            || input.key(Key::Space)
            || (input.click && self.hover_button == Some(Button::NewRun))
        {
            self.start_run();
        } else if input.key(Key::H) || (input.click && self.hover_button == Some(Button::HowTo)) {
            self.help = true;
            self.sfx.push(Sfx::Click);
        } else if input.key(Key::Escape) || (input.click && self.hover_button == Some(Button::Quit))
        {
            self.quit = true;
        }
    }

    fn update_game(&mut self, dt: f32, input: &Input) {
        let Some(run) = self.run.as_ref() else {
            self.screen = Screen::Title;
            return;
        };
        match run.stage.clone() {
            Stage::Battle => self.update_battle(dt, input),
            Stage::Draft { options, .. } => {
                let mut pick = None;
                for (i, k) in [Key::Num1, Key::Num2, Key::Num3].iter().enumerate() {
                    if input.key(*k) && i < options.len() {
                        pick = Some(i);
                    }
                }
                if input.click
                    && let Some(Button::Draft(i)) = self.hover_button
                {
                    pick = Some(i);
                }
                if let Some(i) = pick {
                    self.choose_upgrade(i);
                }
            }
            Stage::Route { options } => {
                let mut pick = None;
                for (i, k) in [Key::Num1, Key::Num2].iter().enumerate() {
                    if input.key(*k) && i < options.len() {
                        pick = Some(i);
                    }
                }
                if input.click
                    && let Some(Button::Route(i)) = self.hover_button
                {
                    pick = Some(i);
                }
                if let Some(i) = pick {
                    self.choose_route(i);
                }
            }
            Stage::Victory | Stage::Defeat { .. } => self.end_run(),
        }
        if input.key(Key::H) {
            self.help = true;
        }
    }

    pub fn choose_upgrade(&mut self, i: usize) {
        if let Some(run) = self.run.as_mut()
            && run.choose_upgrade(i)
        {
            self.sfx.push(Sfx::Draft);
            self.after_stage_change();
        }
    }

    pub fn choose_route(&mut self, i: usize) {
        if let Some(run) = self.run.as_mut()
            && run.choose_route(i)
        {
            self.sfx.push(Sfx::Click);
            self.after_stage_change();
        }
    }

    fn after_stage_change(&mut self) {
        let Some(run) = self.run.as_ref() else {
            return;
        };
        if run.stage == Stage::Battle {
            self.backdrop = None;
            self.enter_battle();
        }
    }

    fn end_run(&mut self) {
        let Some(run) = self.run.as_ref() else {
            return;
        };
        let won = run.won();
        let island = if won {
            run.island
        } else {
            run.island.saturating_sub(1)
        };
        let score = run.score;
        self.new_best = self.save.record_run(score, island, won);
        self.persist();
        self.screen = Screen::End;
    }

    fn update_battle(&mut self, dt: f32, input: &Input) {
        // Enemy phase: feed one enemy at a time to the view.
        let busy = self.view.busy();
        let mut fed = vec![];
        let mut became_player = false;
        if let Some(b) = self.battle_mut()
            && b.phase == Phase::Enemy
            && !busy
            && b.outcome.is_none()
        {
            fed = b.enemy_step();
            became_player = b.phase == Phase::Player;
        }
        if !fed.is_empty() {
            self.view.feed(fed);
        }
        if became_player {
            self.selected = None;
            self.select_next();
            if self.tutorial == Some(5) {
                self.finish_tutorial();
            }
        }
        // Outcome: let the banner breathe, then move the run on.
        if self.battle().is_some_and(|b| b.outcome.is_some()) && !self.view.busy() {
            self.outcome_timer += dt;
            if self.outcome_timer > 1.6 {
                self.outcome_timer = 0.0;
                self.finish_tutorial();
                let snapshot = self.battle().cloned();
                if let Some(run) = self.run.as_mut() {
                    run.finish_battle();
                }
                if let Some(run) = self.run.as_ref() {
                    match run.stage {
                        Stage::Draft { .. } => {
                            self.backdrop = snapshot;
                            self.sfx.push(Sfx::Draft);
                        }
                        Stage::Victory => {
                            self.backdrop = snapshot;
                            self.end_run();
                        }
                        Stage::Defeat { .. } => self.end_run(),
                        _ => {}
                    }
                }
            }
            return;
        }
        self.update_hover();
        if !self.player_can_act() {
            return;
        }
        if self.tutorial.is_some() {
            self.tutorial_timer += dt;
        }
        // Keyboard
        if input.key(Key::Tab) {
            self.select_next();
            self.sfx.push(Sfx::Select);
            if self.tutorial == Some(0) {
                self.advance_tutorial(1);
            }
        }
        for (i, k) in [Key::Num1, Key::Num2, Key::Num3].iter().enumerate() {
            if input.key(*k) {
                self.pick_ability(i);
            }
        }
        if input.key(Key::U) {
            self.undo();
        }
        if input.key(Key::D) {
            self.show_danger = !self.show_danger;
            self.sfx.push(Sfx::Click);
        }
        if input.key(Key::Space) || input.key(Key::Enter) {
            self.end_turn();
            return;
        }
        if input.key(Key::Escape) || input.right_click {
            if matches!(self.mode, Mode::Ability(_)) {
                self.mode = Mode::Move;
            } else {
                self.selected = None;
            }
            self.sfx.push(Sfx::Click);
        }
        if input.click {
            if let Some(btn) = self.hover_button {
                self.press(btn);
            } else {
                self.click_board();
            }
        }
    }

    fn press(&mut self, btn: Button) {
        match btn {
            Button::Unit(id) => {
                if self.battle().is_some_and(|b| b.unit(id).alive) {
                    self.select(id);
                }
            }
            Button::Ability(i) => self.pick_ability(i),
            Button::EndTurn => self.end_turn(),
            Button::Undo => self.undo(),
            Button::Danger => {
                self.show_danger = !self.show_danger;
                self.sfx.push(Sfx::Click);
            }
            Button::Help => {
                self.help = true;
                self.sfx.push(Sfx::Click);
            }
            _ => {}
        }
    }

    pub fn pick_ability(&mut self, i: usize) {
        let Some(id) = self.selected else {
            self.error("Select a squad member first");
            return;
        };
        let Some(b) = self.battle() else {
            return;
        };
        let u = b.unit(id);
        let Some(&a) = u.kind.abilities().get(i) else {
            return;
        };
        if u.acted {
            self.error("Already acted this turn");
            return;
        }
        if u.cooldowns[i] > 0 {
            self.error("That ability is recharging");
            return;
        }
        let targets = b.targets(id, a);
        if targets.is_empty() {
            self.error("No valid targets in range");
            return;
        }
        if a.info().target == Target::SelfOnly {
            self.do_act(id, a, u.pos);
            return;
        }
        self.mode = if self.mode == Mode::Ability(a) {
            Mode::Move
        } else {
            Mode::Ability(a)
        };
        self.sfx.push(Sfx::Click);
    }

    pub fn end_turn(&mut self) {
        if !self.player_can_act() {
            return;
        }
        let ev = match self.battle_mut() {
            Some(b) => b.end_player_turn(),
            None => return,
        };
        self.view.feed(ev);
        self.selected = None;
        self.mode = Mode::Move;
        if self.tutorial.is_some_and(|t| t >= 3) {
            self.advance_tutorial(5);
        }
    }

    pub fn undo(&mut self) {
        let ok = self.battle_mut().is_some_and(|b| b.undo_move());
        if ok {
            if let Some(b) = self.run.as_ref().and_then(|r| r.battle.as_ref()) {
                self.view.reset(b);
            }
            self.mode = Mode::Move;
            self.sfx.push(Sfx::Click);
        } else {
            self.error("Nothing to undo");
        }
    }

    fn do_act(&mut self, id: u32, a: Ability, target: Pos) {
        let res = self.battle_mut().map(|b| b.act(id, a, target));
        match res {
            Some(Ok(ev)) => {
                self.view.feed(ev);
                self.mode = Mode::Move;
                if self.tutorial.is_some_and(|t| t <= 2) {
                    self.advance_tutorial(3);
                }
                self.after_squad_action(id);
            }
            Some(Err(e)) => self.error(e),
            None => {}
        }
    }

    fn do_move(&mut self, id: u32, dest: Pos) {
        let res = self.battle_mut().map(|b| b.move_unit(id, dest));
        match res {
            Some(Ok(ev)) => {
                self.view.feed(ev);
                if self.tutorial == Some(1) {
                    self.advance_tutorial(2);
                }
                self.after_squad_action(id);
            }
            Some(Err(e)) => self.error(e),
            None => {}
        }
    }

    fn after_squad_action(&mut self, id: u32) {
        let done = self.battle().is_some_and(|b| {
            let u = b.unit(id);
            !u.alive || (u.moved && u.acted)
        });
        if done {
            self.select_next();
        }
    }

    /// Unit under the mouse: sprite bounding boxes, front-most first.
    fn unit_under_mouse(&self) -> Option<u32> {
        let b = self.battle()?;
        let (mx, my) = self.mouse;
        let mut best: Option<(i32, u32)> = None;
        for u in b.units.iter().filter(|u| u.alive) {
            let Some(vis) = self.view.vis.get(&u.id) else {
                continue;
            };
            if !vis.visible {
                continue;
            }
            let (x, y) = iso::tile_center(vis.x, vis.y, vis.h);
            let h = sprites::unit_height(u.kind);
            let w = if u.kind == Kind::HollowKing {
                28.0
            } else {
                13.0
            };
            if mx >= x - w && mx <= x + w && my >= y - h && my <= y + 4.0 {
                let depth = u.pos.x + u.pos.y;
                if best.is_none_or(|(d, _)| depth > d) {
                    best = Some((depth, u.id));
                }
            }
        }
        best.map(|(_, id)| id)
    }

    fn update_hover(&mut self) {
        self.hover_unit = None;
        self.hover_tile = None;
        if self.hover_button.is_some() {
            return;
        }
        let Some(b) = self.battle() else {
            return;
        };
        let (mx, my) = self.mouse;
        let tile = iso::pick(&b.map, mx, my);
        // A highlighted tile under the cursor wins over a sprite overlapping it.
        let highlighted = tile.is_some_and(|t| match (self.selected, self.mode) {
            (Some(id), Mode::Move) => b.move_targets(id).contains(&t),
            (Some(id), Mode::Ability(a)) => b.targets(id, a).contains(&t),
            _ => false,
        });
        let unit = if highlighted {
            None
        } else {
            self.unit_under_mouse()
        };
        let hover_unit = unit.or_else(|| tile.and_then(|t| b.unit_at(t).map(|u| u.id)));
        let hover_tile = match (unit, tile) {
            (Some(id), _) => Some(b.unit(id).pos),
            (None, t) => t,
        };
        self.hover_unit = hover_unit;
        self.hover_tile = hover_tile;
    }

    fn click_board(&mut self) {
        let Some(tile) = self.hover_tile else {
            if let Mode::Ability(_) = self.mode {
                self.mode = Mode::Move;
            }
            return;
        };
        let Some(b) = self.battle() else {
            return;
        };
        let occupant = b.unit_at(tile).map(|u| (u.id, u.team()));
        if let (Some(id), Mode::Ability(a)) = (self.selected, self.mode) {
            if b.targets(id, a).contains(&tile) {
                self.do_act(id, a, tile);
                return;
            }
            if let Some((oid, Team::Squad)) = occupant {
                self.select(oid);
                return;
            }
            self.mode = Mode::Move;
            self.error("Not a valid target");
            return;
        }
        match occupant {
            Some((oid, Team::Squad)) => self.select(oid),
            Some((_, Team::Dark)) => {
                // Quick attack: use the first offensive ability that reaches.
                if let Some(id) = self.selected {
                    let u = b.unit(id);
                    let quick = u.kind.abilities().iter().copied().find(|a| {
                        a.info().target == Target::Enemy && b.targets(id, *a).contains(&tile)
                    });
                    if let Some(a) = quick {
                        self.do_act(id, a, tile);
                    }
                }
            }
            None => {
                if let Some(id) = self.selected {
                    if b.move_targets(id).contains(&tile) {
                        self.do_move(id, tile);
                    } else if b.map.terrain(tile) == Terrain::Brazier {
                        self.error("Select the Lamplighter and use Kindle next to it");
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- overlay

    /// Tiles a dark unit could strike next turn.
    pub fn threat_of(b: &Battle, id: u32) -> Vec<Pos> {
        let u = b.unit(id);
        let field = b.move_field(id);
        let mut stands: Vec<Pos> = Map::all_pos()
            .filter(|p| {
                field[Map::idx(*p)].is_some()
                    && b.map.walkable(*p)
                    && (b.unit_at(*p).is_none() || *p == u.pos)
            })
            .collect();
        stands.push(u.pos);
        let mut out = vec![];
        for a in u.kind.abilities() {
            let info = a.info();
            if info.target != Target::Enemy {
                continue;
            }
            for s in &stands {
                for p in Map::all_pos() {
                    let d = s.manhattan(p);
                    if d >= info.min_range && d <= info.max_range && !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
        }
        out
    }

    pub fn overlay(&self) -> Overlay {
        let mut ov = Overlay {
            hover: self.hover_tile,
            selected: self.selected,
            target_color: 0xff4a4a,
            ..Overlay::default()
        };
        let Some(b) = self.battle() else {
            return ov;
        };
        if b.phase != Phase::Player || self.view.busy() || b.outcome.is_some() {
            return ov;
        }
        if self.show_danger {
            for e in b.alive(Team::Dark) {
                for p in App::threat_of(b, e.id) {
                    if !ov.danger.contains(&p) {
                        ov.danger.push(p);
                    }
                }
            }
        }
        if let Some(h) = self.hover_unit
            && b.unit(h).team() == Team::Dark
            && !matches!(self.mode, Mode::Ability(_))
        {
            ov.threat = App::threat_of(b, h);
        }
        let Some(id) = self.selected else {
            return ov;
        };
        let u = b.unit(id);
        match self.mode {
            Mode::Move => {
                ov.moves = b.move_targets(id);
                if let Some(h) = self.hover_tile
                    && ov.moves.contains(&h)
                {
                    ov.path = b.path_to(id, h).unwrap_or_default();
                }
            }
            Mode::Ability(a) => {
                ov.targets = b.targets(id, a);
                ov.target_color = match a.info().target {
                    Target::Enemy => 0xff4a4a,
                    Target::Ally => 0x60ff90,
                    Target::Brazier => 0xffa040,
                    Target::SelfOnly => 0xffd060,
                };
                if let Some(h) = self.hover_tile
                    && ov.targets.contains(&h)
                {
                    if a == Ability::Volley {
                        ov.aoe = h.neighbors().to_vec();
                    }
                    if a.pushes()
                        && let Some(t) = b.unit_at(h)
                        && t.kind != Kind::HollowKing
                    {
                        let dir = u.pos.dir_to(h);
                        let dist = if a == Ability::Bash {
                            1 + b.cfg.mods.bash_push
                        } else {
                            1
                        };
                        let mut land = h;
                        let mut lethal = false;
                        for _ in 0..dist {
                            let n = land.add(dir.0, dir.1);
                            let terr = b.map.terrain(n);
                            if terr.blocks() || b.unit_at(n).is_some() {
                                break;
                            }
                            land = n;
                            if terr.lethal() {
                                lethal = !t.kind.flies();
                                break;
                            }
                        }
                        ov.push = Some((land, lethal));
                        ov.aoe.push(land);
                    }
                }
            }
        }
        ov
    }

    // ------------------------------------------------------------------- draw

    pub fn draw(&mut self, c: &mut Canvas) {
        let parallax = (self.mouse.0 / W as f32 - 0.5) * 2.0;
        let t = self.time;
        match self.screen {
            Screen::Title => {
                if self.sky_key != (Biome::Crystal, 1) {
                    self.pal = palette(Biome::Crystal);
                    self.sky = Sky::new(&self.pal, 1);
                    self.sky_key = (Biome::Crystal, 1);
                }
                self.sky.draw(c, t, parallax);
                self.sky.draw_near(c, t, parallax);
                c.ox = 0.0;
                c.oy = 20.0 + (t * 0.6).sin() * 6.0;
                let ov = Overlay::default();
                scene::draw_island(
                    c,
                    &self.title_map,
                    &self.title_glow,
                    &self.pal,
                    &ov,
                    t,
                    |_, _| {},
                );
                c.ox = 0.0;
                c.oy = 0.0;
                scene::apply_vignette(c, &self.vig);
                ui::draw_title(c, self);
            }
            Screen::Game | Screen::End => {
                self.sky.draw(c, t, parallax);
                self.sky.draw_near(c, t, parallax);
                let ov = self.overlay();
                let (sx, sy) = self.view.shake_offset();
                c.ox = sx;
                c.oy = sy;
                let battle = self
                    .run
                    .as_ref()
                    .and_then(|r| r.battle.as_ref())
                    .or(self.backdrop.as_ref());
                if let Some(b) = battle {
                    scene::draw_battle(c, b, &self.view, &self.pal, &ov);
                }
                c.ox = 0.0;
                c.oy = 0.0;
                if self.view.flash > 0.0 {
                    scene::apply_flash(c, self.view.flash_color, self.view.flash * 0.5);
                }
                scene::apply_vignette(c, &self.vig);
                ui::draw_game(c, self, &ov);
            }
        }
        if self.help {
            ui::draw_help(c, self);
        }
    }

    // ----------------------------------------------------------- automation

    /// Drive the squad with the autopilot for one action (used by the
    /// selftest). Returns false when nothing was done.
    pub fn autopilot_step(&mut self) -> bool {
        if !self.player_can_act() {
            return false;
        }
        let Some(b) = self.battle() else {
            return false;
        };
        let best = b
            .units
            .iter()
            .filter(|u| u.alive && u.team() == Team::Squad && !(u.moved && u.acted))
            .map(|u| (u.id, ai::plan(b, u.id)))
            .filter(|(id, p)| p.action.is_some() || p.dest != b.unit(*id).pos)
            .max_by(|x, y| x.1.score.total_cmp(&y.1.score));
        let Some((id, plan)) = best else {
            self.end_turn();
            return true;
        };
        let pos = b.unit(id).pos;
        self.selected = Some(id);
        self.mode = Mode::Move;
        if plan.dest != pos {
            self.do_move(id, plan.dest);
        }
        if let Some((a, t)) = plan.action {
            // The move animation must finish before acting.
            let res = self.battle_mut().map(|b| b.act(id, a, t));
            if let Some(Ok(ev)) = res {
                self.view.feed(ev);
                self.after_squad_action(id);
            }
        }
        if let Some(b) = self.battle_mut() {
            let u = b.unit_mut(id);
            u.moved = true;
            u.acted = true;
        }
        true
    }

    /// Screen position of a tile's centre (for synthetic clicks).
    pub fn tile_screen(&self, p: Pos) -> Option<(f32, f32)> {
        let b = self.battle()?;
        Some(iso::pos_center(&b.map, p))
    }
}
