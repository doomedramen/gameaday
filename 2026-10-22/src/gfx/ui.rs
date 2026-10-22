//! HUD, panels, buttons and full-screen menus.

use super::canvas::{Canvas, H, W, lerp_color};
use super::palette::palette;
use super::scene::Overlay;
use super::sprites::{self, Look};
use crate::app::{App, Button, Mode, Screen, TUTORIAL};
use crate::battle::{Battle, GLOOM_ROUNDS, LONG_NIGHT, Phase, REINFORCE_EVERY};
use crate::map::Pickup;
use crate::run::{ISLANDS, Node, Stage, Upgrade};
use crate::units::{Kind, Target, Team};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

const GOLD: u32 = 0xffd27a;
const INK: u32 = 0x0c0a12;
const TEXT: u32 = 0xeae4f4;
const DIM: u32 = 0x9a92b0;

fn panel(c: &mut Canvas, r: Rect, a: f32) {
    c.round_rect(r.x, r.y, r.w, r.h, 8.0, INK, 0.78 * a);
    c.round_rect_outline(r.x, r.y, r.w, r.h, 8.0, 0x6a5a8a, 0.55 * a);
}

fn button(c: &mut Canvas, r: Rect, label: &str, hot: bool, enabled: bool, accent: u32) {
    let base = if enabled { 0x241c34 } else { 0x16121e };
    c.round_rect(
        r.x,
        r.y,
        r.w,
        r.h,
        7.0,
        if hot && enabled {
            lerp_color(base, accent, 0.35)
        } else {
            base
        },
        0.92,
    );
    c.round_rect_outline(
        r.x,
        r.y,
        r.w,
        r.h,
        7.0,
        if enabled { accent } else { 0x3a3448 },
        if hot { 1.0 } else { 0.6 },
    );
    let s = 2.0;
    let tw = Canvas::text_width(label, s);
    c.text_shadow(
        r.x + (r.w - tw) / 2.0,
        r.y + (r.h - 16.0) / 2.0,
        label,
        s,
        if enabled { TEXT } else { 0x5a5468 },
        1.0,
    );
}

pub fn card_rect(i: usize) -> Rect {
    Rect::new(14.0, 70.0 + i as f32 * 84.0, 232.0, 76.0)
}

pub fn ability_rect(i: usize) -> Rect {
    Rect::new(372.0 + i as f32 * 182.0, 642.0, 172.0, 62.0)
}

pub const END_TURN: Rect = Rect::new(1086.0, 646.0, 180.0, 54.0);
pub const UNDO: Rect = Rect::new(924.0, 652.0, 74.0, 44.0);
pub const DANGER: Rect = Rect::new(1004.0, 652.0, 74.0, 44.0);
pub const HELP: Rect = Rect::new(1226.0, 12.0, 40.0, 32.0);

pub fn draft_rect(i: usize, n: usize) -> Rect {
    let w = 300.0;
    let gap = 30.0;
    let total = n as f32 * w + (n as f32 - 1.0) * gap;
    Rect::new(
        (W as f32 - total) / 2.0 + i as f32 * (w + gap),
        230.0,
        w,
        330.0,
    )
}

pub fn route_rect(i: usize, n: usize) -> Rect {
    let w = 380.0;
    let gap = 40.0;
    let total = n as f32 * w + (n as f32 - 1.0) * gap;
    Rect::new(
        (W as f32 - total) / 2.0 + i as f32 * (w + gap),
        240.0,
        w,
        300.0,
    )
}

pub fn title_rect(i: usize) -> Rect {
    Rect::new(
        W as f32 / 2.0 - 375.0 + i as f32 * 260.0,
        572.0,
        230.0,
        50.0,
    )
}

pub const CONTINUE: Rect = Rect::new(W as f32 / 2.0 - 140.0, 600.0, 280.0, 52.0);

/// All clickable regions for the current state.
pub fn buttons(app: &App) -> Vec<(Button, Rect)> {
    let mut out = vec![];
    match app.screen {
        Screen::Title => {
            out.push((Button::NewRun, title_rect(0)));
            out.push((Button::HowTo, title_rect(1)));
            out.push((Button::Quit, title_rect(2)));
        }
        Screen::End => out.push((Button::Continue, CONTINUE)),
        Screen::Game => {
            let Some(run) = app.run.as_ref() else {
                return out;
            };
            match &run.stage {
                Stage::Battle => {
                    if let Some(b) = run.battle.as_ref() {
                        for (i, u) in b
                            .units
                            .iter()
                            .filter(|u| u.team() == Team::Squad)
                            .enumerate()
                        {
                            out.push((Button::Unit(u.id), card_rect(i)));
                        }
                        if let Some(id) = app.selected {
                            for i in 0..b.unit(id).kind.abilities().len() {
                                out.push((Button::Ability(i), ability_rect(i)));
                            }
                        }
                    }
                    out.push((Button::EndTurn, END_TURN));
                    out.push((Button::Undo, UNDO));
                    out.push((Button::Danger, DANGER));
                    out.push((Button::Help, HELP));
                }
                Stage::Draft { options, .. } => {
                    for i in 0..options.len() {
                        out.push((Button::Draft(i), draft_rect(i, options.len())));
                    }
                }
                Stage::Route { options } => {
                    for i in 0..options.len() {
                        out.push((Button::Route(i), route_rect(i, options.len())));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

// ------------------------------------------------------------------- title

pub fn draw_title(c: &mut Canvas, app: &App) {
    let t = app.time;
    let cx = W as f32 / 2.0;
    // Logo
    c.glow_ellipse(
        cx,
        150.0,
        420.0,
        90.0,
        0xff9a40,
        0.25 + 0.05 * (t * 2.0).sin(),
    );
    let title = "LANTERNFALL";
    let s = 9.0;
    let tw = Canvas::text_width(title, s);
    c.text(cx - tw / 2.0 + 5.0, 112.0 + 5.0, title, s, 0x000000, 0.6);
    c.text(cx - tw / 2.0, 112.0, title, s, 0xffe2a0, 1.0);
    // A flickering lantern over the second L
    let lx = cx - tw / 2.0 + 7.0 * s * 4.0 + 28.0;
    c.glow(lx, 100.0, 40.0 + (t * 9.0).sin() * 3.0, 0xffa040, 0.5);
    c.text_center(
        cx,
        200.0,
        "A tactics roguelite of light and shadow",
        2.0,
        0xd8c8f0,
        1.0,
    );
    let labels = ["New Run", "How to Play", "Quit"];
    for (i, l) in labels.iter().enumerate() {
        let r = title_rect(i);
        let btn = [Button::NewRun, Button::HowTo, Button::Quit][i];
        button(c, r, l, app.hover_button == Some(btn), true, GOLD);
    }
    let s = &app.save;
    let stats = if s.runs > 0 {
        format!(
            "Best score {}   Furthest island {}/{}   Runs {}   Victories {}",
            s.best_score, s.best_island, ISLANDS, s.runs, s.wins
        )
    } else {
        "Enter / Space to begin  -  H for help".to_string()
    };
    c.text_center(cx, 646.0, &stats, 2.0, DIM, 1.0);
    c.text_center(cx, 690.0, "GameADay #3", 1.0, 0x6a6080, 1.0);
}

// -------------------------------------------------------------------- game

pub fn draw_game(c: &mut Canvas, app: &App, ov: &Overlay) {
    let Some(run) = app.run.as_ref() else {
        return;
    };
    match &run.stage {
        Stage::Battle => {
            if let Some(b) = run.battle.as_ref() {
                draw_hud(c, app, b, ov);
            }
        }
        Stage::Draft { options, picks } => draw_draft(c, app, options, *picks),
        Stage::Route { options } => draw_route(c, app, options),
        _ => {}
    }
    draw_banner(c, app);
    if app.screen == Screen::End {
        draw_end(c, app);
    }
}

fn draw_banner(c: &mut Canvas, app: &App) {
    if app.screen == Screen::End {
        return;
    }
    if let Some(bn) = &app.view.banner
        && bn.small
    {
        let k = bn.life / bn.max;
        let a = (k * 4.0).min(1.0).min((1.0 - k) * 8.0 + 0.2).min(1.0);
        let w = Canvas::text_width(&bn.text, 3.0) + 60.0;
        let r = Rect::new(W as f32 / 2.0 - w / 2.0, 112.0, w, 46.0);
        c.round_rect(r.x, r.y, r.w, r.h, 10.0, 0x000000, 0.55 * a);
        c.round_rect_outline(r.x, r.y, r.w, r.h, 10.0, bn.color, 0.6 * a);
        c.text_center(W as f32 / 2.0, r.y + 12.0, &bn.text, 3.0, bn.color, a);
    } else if let Some(bn) = &app.view.banner {
        let k = bn.life / bn.max;
        let a = (k * 4.0).min(1.0).min((1.0 - k) * 8.0 + 0.2).min(1.0);
        let y = 236.0;
        c.vgradient(0.0, y - 10.0, W as f32, 92.0, 0x000000, 0x000000, 0.45 * a);
        c.fill_rect(0.0, y - 10.0, W as f32, 1.0, bn.color, 0.6 * a);
        c.fill_rect(0.0, y + 81.0, W as f32, 1.0, bn.color, 0.6 * a);
        c.text_center(W as f32 / 2.0, y + 4.0, &bn.text, 5.0, bn.color, a);
        if !bn.sub.is_empty() {
            c.text_center(W as f32 / 2.0, y + 56.0, &bn.sub, 2.0, TEXT, a);
        }
    }
    if let Some((n, t)) = app.view.chain {
        let pop = 1.0 + (t - 1.2).max(0.0) * 3.0;
        let a = (t * 2.0).min(1.0);
        c.text_center(
            W as f32 / 2.0,
            96.0,
            &format!("CHAIN x{n}"),
            4.0 * pop,
            0xffb040,
            a,
        );
    }
}

fn draw_hud(c: &mut Canvas, app: &App, b: &Battle, ov: &Overlay) {
    let run = app.run.as_ref().expect("run");
    // ---- top bar
    c.vgradient(0.0, 0.0, W as f32, 58.0, 0x07050c, 0x07050c, 0.75);
    c.fill_rect(0.0, 58.0, W as f32, 1.0, 0x6a5a8a, 0.5);
    c.text_shadow(
        16.0,
        12.0,
        &format!("ISLAND {}/{}", b.cfg.island, ISLANDS),
        2.0,
        GOLD,
        1.0,
    );
    let tag = if b.cfg.boss {
        "Boss".to_string()
    } else if b.cfg.elite {
        "Elite".to_string()
    } else {
        "Skirmish".to_string()
    };
    c.text_shadow(
        16.0,
        34.0,
        &format!("{} - {tag}", b.cfg.biome.name()),
        1.0,
        DIM,
        1.0,
    );
    let phase = match b.phase {
        Phase::Player => "YOUR TURN",
        Phase::Enemy => "THE DARK MOVES",
        Phase::Over => "",
    };
    let cx = W as f32 / 2.0;
    c.text_center(cx, 10.0, &format!("ROUND {}", b.round), 2.0, TEXT, 1.0);
    c.text_center(
        cx,
        32.0,
        phase,
        1.0,
        if b.phase == Phase::Player {
            0x9fe0ff
        } else {
            0xc8a0ff
        },
        1.0,
    );
    // Gloom track
    let next_gloom = GLOOM_ROUNDS.iter().find(|r| **r > b.round);
    let warn = if b.round >= LONG_NIGHT {
        format!("LONG NIGHT: -{} HP/round", 1 + (b.round - LONG_NIGHT) / 2)
    } else if let Some(g) = next_gloom {
        format!("Gloom in {}", g - b.round)
    } else {
        format!("Long night in {}", LONG_NIGHT - b.round)
    };
    let reinf = if b.cfg.boss {
        String::new()
    } else {
        let next = (b.round / REINFORCE_EVERY + 1) * REINFORCE_EVERY;
        format!("   Shades in {}", next - b.round)
    };
    c.text_shadow(
        cx + 120.0,
        16.0,
        &format!("{warn}{reinf}"),
        1.0,
        if b.round >= LONG_NIGHT {
            0xff7070
        } else {
            0xb8a0e0
        },
        1.0,
    );
    let score = run.score + b.score;
    let st = format!("SCORE {score}");
    c.text_shadow(
        1210.0 - Canvas::text_width(&st, 2.0),
        12.0,
        &st,
        2.0,
        GOLD,
        1.0,
    );
    let foes = b.alive(Team::Dark).count();
    let ft = format!("{foes} foes remain");
    c.text_shadow(
        1210.0 - Canvas::text_width(&ft, 1.0),
        36.0,
        &ft,
        1.0,
        DIM,
        1.0,
    );
    button(
        c,
        HELP,
        "?",
        app.hover_button == Some(Button::Help),
        true,
        0x8a7ab0,
    );

    // Boss bar
    if let Some(k) = b.boss()
        && k.alive
    {
        let r = Rect::new(cx - 260.0, 66.0, 520.0, 18.0);
        let vis_hp = app.view.vis.get(&k.id).map(|v| v.hp).unwrap_or(k.hp);
        c.round_rect(
            r.x - 2.0,
            r.y - 2.0,
            r.w + 4.0,
            r.h + 4.0,
            5.0,
            0x000000,
            0.8,
        );
        c.round_rect(
            r.x,
            r.y,
            r.w * vis_hp.max(0) as f32 / k.max_hp as f32,
            r.h,
            4.0,
            0xc01830,
            1.0,
        );
        c.text_center(cx, r.y + 5.0, "THE HOLLOW KING", 1.0, TEXT, 1.0);
        if !b.is_lit(k.pos) {
            c.text_center(
                cx,
                r.y + 24.0,
                "Shrouded - light it with a flare or brazier!",
                1.0,
                0xb090ff,
                1.0,
            );
        }
    }

    // ---- squad cards
    for (i, u) in b
        .units
        .iter()
        .filter(|u| u.team() == Team::Squad)
        .enumerate()
    {
        let r = card_rect(i);
        let sel = app.selected == Some(u.id);
        let hot = app.hover_button == Some(Button::Unit(u.id));
        // Follow the animation, so a card falls when the sprite does.
        let alive = u.alive || app.view.vis.get(&u.id).is_some_and(|v| v.visible);
        c.round_rect(
            r.x,
            r.y,
            r.w,
            r.h,
            8.0,
            if sel { 0x2a2040 } else { INK },
            0.82,
        );
        c.round_rect_outline(
            r.x,
            r.y,
            r.w,
            r.h,
            8.0,
            if sel {
                GOLD
            } else if hot {
                0xb0a0d0
            } else {
                0x4a3e62
            },
            if sel { 1.0 } else { 0.7 },
        );
        let look = Look {
            facing: 1.0,
            t: app.time + i as f32,
            light: if alive { 1.0 } else { 0.3 },
            alpha: if alive { 1.0 } else { 0.5 },
            flash: 0.0,
            shrouded: false,
            wounded: false,
        };
        c.glow(
            r.x + 32.0,
            r.y + 46.0,
            30.0,
            0xffa040,
            if alive { 0.15 } else { 0.0 },
        );
        sprites::draw_unit(c, u.kind, r.x + 30.0, r.y + 66.0, look);
        c.text_shadow(
            r.x + 62.0,
            r.y + 8.0,
            u.kind.name(),
            2.0,
            if alive { TEXT } else { 0x6a6478 },
            1.0,
        );
        if !alive {
            c.text_shadow(r.x + 62.0, r.y + 32.0, "FALLEN", 2.0, 0xff6a6a, 1.0);
            continue;
        }
        // HP bar
        let hp_shown = app.view.vis.get(&u.id).map(|v| v.hp).unwrap_or(u.hp);
        let bw = 150.0;
        c.fill_rect(r.x + 62.0, r.y + 30.0, bw, 10.0, 0x000000, 0.7);
        let frac = hp_shown.max(0) as f32 / u.max_hp as f32;
        c.fill_rect(
            r.x + 62.0,
            r.y + 30.0,
            bw * frac,
            10.0,
            if frac < 0.35 { 0xffb040 } else { 0x6ad860 },
            1.0,
        );
        c.text(
            r.x + 64.0,
            r.y + 31.0,
            &format!("{}/{}", hp_shown, u.max_hp),
            1.0,
            0x0a0a0a,
            1.0,
        );
        // Status chips
        let chip = |c: &mut Canvas, x: f32, label: &str, on: bool| {
            c.round_rect(
                x,
                r.y + 48.0,
                46.0,
                18.0,
                4.0,
                if on { 0x3a6a4a } else { 0x2a2436 },
                0.95,
            );
            c.text(
                x + 5.0,
                r.y + 53.0,
                label,
                1.0,
                if on { 0xd8ffe0 } else { 0x6a6478 },
                1.0,
            );
        };
        chip(c, r.x + 62.0, "MOVE", !u.moved);
        chip(c, r.x + 112.0, "ACT", !u.acted);
        c.glow(r.x + 176.0, r.y + 57.0, 9.0, 0xffa040, 0.6);
        c.text(
            r.x + 186.0,
            r.y + 53.0,
            &format!("R{}", u.lantern),
            1.0,
            0xffd090,
            1.0,
        );
        if u.guarded {
            c.text(r.x + 208.0, r.y + 53.0, "G", 1.0, 0x90c8ff, 1.0);
        }
        if u.weakened > 0 {
            c.text(r.x + 218.0, r.y + 53.0, "W", 1.0, 0xc070ff, 1.0);
        }
    }

    // ---- log
    let log_r = Rect::new(14.0, 330.0, 232.0, 150.0);
    panel(c, log_r, 0.8);
    let lines: Vec<&String> = b.log.iter().rev().take(9).collect();
    for (i, l) in lines.iter().enumerate() {
        let a = 1.0 - i as f32 * 0.09;
        let s: String = l.chars().take(32).collect();
        c.text(
            log_r.x + 8.0,
            log_r.y + 8.0 + i as f32 * 15.0,
            &s,
            1.0,
            if i == 0 { TEXT } else { DIM },
            a,
        );
    }

    // ---- upgrades
    if !run.upgrades.is_empty() {
        let r = Rect::new(14.0, 490.0, 232.0, 140.0);
        panel(c, r, 0.8);
        c.text(r.x + 8.0, r.y + 8.0, "BOONS", 1.0, GOLD, 1.0);
        for (i, u) in run.upgrades.iter().take(11).enumerate() {
            let col = i % 2;
            let row = i / 2;
            let s: String = u.name().chars().take(14).collect();
            c.text(
                r.x + 8.0 + col as f32 * 114.0,
                r.y + 24.0 + row as f32 * 18.0,
                &s,
                1.0,
                TEXT,
                1.0,
            );
        }
    }

    // ---- action bar
    draw_action_bar(c, app, b);

    // ---- inspector
    draw_inspector(c, app, b, ov);

    // ---- tutorial
    if let Some(step) = app.tutorial
        && b.phase == Phase::Player
    {
        let r = Rect::new(300.0, 568.0, 680.0, 64.0);
        let pulse = 0.6 + 0.4 * (app.time * 3.0).sin();
        c.round_rect(r.x, r.y, r.w, r.h, 8.0, 0x1a1408, 0.9);
        c.round_rect_outline(r.x, r.y, r.w, r.h, 8.0, GOLD, pulse);
        c.text(
            r.x + 12.0,
            r.y + 8.0,
            &format!("GUIDE {}/{}", step + 1, TUTORIAL.len()),
            1.0,
            GOLD,
            1.0,
        );
        c.text_wrap(
            r.x + 12.0,
            r.y + 24.0,
            r.w - 24.0,
            TUTORIAL[step],
            1.0,
            TEXT,
            1.0,
        );
    }
    if let Some((msg, t)) = &app.last_error {
        let a = t.min(1.0);
        c.text_center(W as f32 / 2.0, 548.0, msg, 2.0, 0xff8a7a, a);
    }
}

fn draw_action_bar(c: &mut Canvas, app: &App, b: &Battle) {
    let can = app.player_can_act();
    let undo_ok = can && b.can_undo();
    button(
        c,
        UNDO,
        "Undo",
        app.hover_button == Some(Button::Undo),
        undo_ok,
        0x80b0ff,
    );
    button(
        c,
        DANGER,
        "Dngr",
        app.hover_button == Some(Button::Danger) || app.show_danger,
        true,
        0xff6a6a,
    );
    let pulse = if can && b.alive(Team::Squad).all(|u| u.moved && u.acted) {
        0.5 + 0.5 * (app.time * 5.0).sin()
    } else {
        0.0
    };
    if pulse > 0.0 {
        c.glow_ellipse(
            END_TURN.center().0,
            END_TURN.center().1,
            120.0,
            40.0,
            GOLD,
            0.25 * pulse,
        );
    }
    button(
        c,
        END_TURN,
        "End Turn",
        app.hover_button == Some(Button::EndTurn),
        can,
        GOLD,
    );
    c.text(END_TURN.x + 60.0, END_TURN.y + 44.0, "SPACE", 1.0, DIM, 1.0);
    let Some(id) = app.selected else {
        if can {
            c.text_center(640.0, 666.0, "Select a squad member (TAB)", 2.0, DIM, 1.0);
        }
        return;
    };
    let u = b.unit(id);
    for (i, a) in u.kind.abilities().iter().enumerate() {
        let r = ability_rect(i);
        let info = a.info();
        let cd = u.cooldowns[i];
        let usable = can && !u.acted && cd == 0;
        let active = app.mode == Mode::Ability(*a);
        let hot = app.hover_button == Some(Button::Ability(i));
        let accent = match info.target {
            Target::Enemy => 0xff6a5a,
            Target::Ally => 0x6aff9a,
            Target::Brazier => 0xffa040,
            Target::SelfOnly => 0x90c8ff,
        };
        let bg = if active {
            lerp_color(0x241c34, accent, 0.4)
        } else if hot && usable {
            lerp_color(0x241c34, accent, 0.2)
        } else {
            0x1a1426
        };
        c.round_rect(r.x, r.y, r.w, r.h, 7.0, bg, 0.92);
        c.round_rect_outline(
            r.x,
            r.y,
            r.w,
            r.h,
            7.0,
            if usable { accent } else { 0x3a3448 },
            if active { 1.0 } else { 0.7 },
        );
        c.round_rect(
            r.x + 6.0,
            r.y + 6.0,
            20.0,
            20.0,
            4.0,
            if usable { accent } else { 0x3a3448 },
            0.9,
        );
        c.text(
            r.x + 12.0,
            r.y + 12.0,
            crate::units::Ability::hotkey_label(i),
            1.0,
            INK,
            1.0,
        );
        c.text_shadow(
            r.x + 32.0,
            r.y + 9.0,
            info.name,
            1.0,
            if usable { TEXT } else { 0x6a6478 },
            1.0,
        );
        let detail = if cd > 0 {
            format!("Recharging ({cd})")
        } else if u.acted {
            "Used this turn".to_string()
        } else if info.damage > 0 {
            let rng = if info.max_range > 1 {
                format!(
                    " r{}-{}",
                    info.min_range,
                    info.max_range + if info.ranged { u.range_bonus } else { 0 }
                )
            } else {
                String::new()
            };
            format!("{} dmg{rng}", info.damage + u.power)
        } else if info.cooldown > 0 {
            format!("cd {}", info.cooldown)
        } else {
            "ready".to_string()
        };
        c.text(
            r.x + 32.0,
            r.y + 24.0,
            &detail,
            1.0,
            if cd > 0 { 0xff9a7a } else { DIM },
            1.0,
        );
        if hot || active {
            // Description tooltip above
            let tr = Rect::new(r.x - 10.0, r.y - 58.0, 360.0, 50.0);
            panel(c, tr, 1.0);
            c.text_wrap(
                tr.x + 8.0,
                tr.y + 8.0,
                tr.w - 16.0,
                info.desc,
                1.0,
                TEXT,
                1.0,
            );
        }
    }
}

fn draw_inspector(c: &mut Canvas, app: &App, b: &Battle, ov: &Overlay) {
    let r = Rect::new(1030.0, 70.0, 236.0, 230.0);
    let Some(tile) = app.hover_tile else {
        return;
    };
    panel(c, r, 1.0);
    let mut y = r.y + 10.0;
    let x = r.x + 10.0;
    let lit = b.is_lit(tile);
    if let Some(u) = b.unit_at(tile) {
        let col = if u.team() == Team::Squad {
            0x9fe0ff
        } else {
            0xff9a8a
        };
        let name = if u.elite {
            format!("Elite {}", u.kind.name())
        } else {
            u.kind.name().to_string()
        };
        c.text_shadow(
            x,
            y,
            &name,
            if name.len() > 15 { 1.0 } else { 2.0 },
            col,
            1.0,
        );
        y += 22.0;
        c.text(
            x,
            y,
            &format!("HP {}/{}   Move {}", u.hp, u.max_hp, u.move_range),
            1.0,
            TEXT,
            1.0,
        );
        y += 14.0;
        y += c.text_wrap(x, y, r.w - 20.0, u.kind.blurb(), 1.0, DIM, 1.0) + 4.0;
        if u.team() == Team::Dark {
            if lit {
                c.text(x, y, "In light: takes full damage", 1.0, 0xffe0a0, 1.0);
            } else {
                c.text(x, y, "SHROUDED: takes half damage", 1.0, 0xb090ff, 1.0);
            }
            y += 14.0;
            for a in u.kind.abilities() {
                let info = a.info();
                let line = if info.damage > 0 {
                    format!(
                        "{}: {} dmg r{}-{}",
                        info.name,
                        info.damage + u.power,
                        info.min_range,
                        info.max_range
                    )
                } else {
                    info.name.to_string()
                };
                c.text(x, y, &line, 1.0, TEXT, 1.0);
                y += 12.0;
            }
        }
        // Damage preview
        if let (Some(sel), Mode::Ability(a)) = (app.selected, app.mode)
            && ov.targets.contains(&tile)
            && a.info().damage > 0
        {
            let att = b.unit(sel);
            let calc = b.calc(att, a, att.pos, u);
            let pr = Rect::new(r.x, r.y + r.h + 8.0, r.w, 92.0);
            panel(c, pr, 1.0);
            let kill = calc.amount >= u.hp;
            c.text_shadow(
                pr.x + 10.0,
                pr.y + 10.0,
                &format!(
                    "{} DAMAGE{}",
                    calc.amount,
                    if kill { " - KILL" } else { "" }
                ),
                2.0,
                if kill { 0xffd060 } else { 0xff8a7a },
                1.0,
            );
            let mut parts = vec![];
            if calc.flank {
                parts.push("flank +1");
            }
            if calc.high_ground {
                parts.push("high ground +1");
            }
            if calc.weakened {
                parts.push("weakened -1");
            }
            if calc.shrouded {
                parts.push("shrouded x0.5");
            }
            if calc.guarded {
                parts.push("guarded -1");
            }
            let mut yy = pr.y + 34.0;
            for p in parts {
                c.text(pr.x + 10.0, yy, p, 1.0, TEXT, 1.0);
                yy += 12.0;
            }
            if let Some((_, lethal)) = ov.push {
                c.text(
                    pr.x + 10.0,
                    yy,
                    if lethal {
                        "SHOVE: INTO THE ABYSS!"
                    } else {
                        "Shove 1 tile"
                    },
                    1.0,
                    if lethal { 0xffd060 } else { DIM },
                    1.0,
                );
            }
        }
    } else {
        let t = b.map.tile(tile);
        c.text_shadow(x, y, t.terrain.name(), 2.0, TEXT, 1.0);
        y += 22.0;
        c.text(x, y, &format!("Height {}", t.height), 1.0, DIM, 1.0);
        y += 14.0;
        c.text(
            x,
            y,
            if lit { "Lit" } else { "Dark" },
            1.0,
            if lit { 0xffe0a0 } else { 0xb090ff },
            1.0,
        );
        y += 16.0;
        match t.terrain {
            crate::map::Terrain::Brazier => {
                let s = if t.lit {
                    "Burning: lights radius 3. Snuffers hunt these."
                } else {
                    "Cold. A Lamplighter can Kindle it from an adjacent tile."
                };
                c.text_wrap(x, y, r.w - 20.0, s, 1.0, TEXT, 1.0);
            }
            crate::map::Terrain::Chasm | crate::map::Terrain::Water => {
                c.text_wrap(
                    x,
                    y,
                    r.w - 20.0,
                    "Shove foes here to destroy them instantly.",
                    1.0,
                    TEXT,
                    1.0,
                );
            }
            _ => {
                if let Some(p) = t.pickup {
                    c.text_shadow(
                        x,
                        y,
                        p.name(),
                        1.0,
                        if p == Pickup::Ember {
                            0xffa060
                        } else {
                            0xfff0a0
                        },
                        1.0,
                    );
                    c.text_wrap(x, y + 14.0, r.w - 20.0, p.describe(), 1.0, TEXT, 1.0);
                } else if t.height > 0 {
                    c.text_wrap(
                        x,
                        y,
                        r.w - 20.0,
                        "High ground: +1 damage downhill, +1 range for ranged attacks.",
                        1.0,
                        TEXT,
                        1.0,
                    );
                }
            }
        }
    }
}

fn draw_draft(c: &mut Canvas, app: &App, options: &[Upgrade], picks: u8) {
    c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x05030a, 0.55);
    let cx = W as f32 / 2.0;
    c.text_center(cx, 110.0, "CHOOSE A BOON", 5.0, GOLD, 1.0);
    let sub = if picks > 1 {
        format!("Elite spoils - {picks} picks remaining")
    } else {
        "Press 1-3 or click a card".to_string()
    };
    c.text_center(cx, 160.0, &sub, 2.0, TEXT, 1.0);
    if let Some(s) = app.run.as_ref().and_then(|r| r.last_summary.as_ref()) {
        let line = format!(
            "Island {} cleansed in {} rounds  -  {} kills  -  +{} score{}",
            s.island,
            s.rounds,
            s.kills,
            s.battle_score + s.bonus,
            if s.flawless { "  -  FLAWLESS" } else { "" }
        );
        c.text_center(cx, 190.0, &line, 1.0, DIM, 1.0);
    }
    for (i, u) in options.iter().enumerate() {
        let r = draft_rect(i, options.len());
        let hot = app.hover_button == Some(Button::Draft(i));
        let lift = if hot { -8.0 } else { 0.0 };
        let r = Rect::new(r.x, r.y + lift, r.w, r.h);
        if hot {
            c.glow_ellipse(r.center().0, r.center().1, r.w * 0.8, r.h * 0.6, GOLD, 0.18);
        }
        c.round_rect(r.x, r.y, r.w, r.h, 12.0, 0x16101e, 0.95);
        c.round_rect_outline(
            r.x,
            r.y,
            r.w,
            r.h,
            12.0,
            if hot { GOLD } else { 0x6a5a8a },
            1.0,
        );
        c.vgradient(
            r.x + 10.0,
            r.y + 10.0,
            r.w - 20.0,
            150.0,
            0x2a1c3a,
            0x120c18,
            1.0,
        );
        let (px, py) = (r.center().0, r.y + 140.0);
        match u.owner() {
            Some(k) => {
                c.glow(px, py - 30.0, 70.0, 0xffa040, 0.25);
                sprites::draw_unit(
                    c,
                    k,
                    px,
                    py,
                    Look {
                        facing: 1.0,
                        t: app.time,
                        light: 1.0,
                        alpha: 1.0,
                        flash: 0.0,
                        shrouded: false,
                        wounded: false,
                    },
                );
            }
            None => {
                for (j, k) in Kind::SQUAD.iter().enumerate() {
                    let ox = (j as f32 - 1.0) * 46.0;
                    c.glow(px + ox, py - 30.0, 40.0, 0xffa040, 0.15);
                    sprites::draw_unit(
                        c,
                        *k,
                        px + ox,
                        py,
                        Look {
                            facing: 1.0,
                            t: app.time + j as f32,
                            light: 1.0,
                            alpha: 1.0,
                            flash: 0.0,
                            shrouded: false,
                            wounded: false,
                        },
                    );
                }
            }
        }
        c.round_rect(r.x + 12.0, r.y + 12.0, 26.0, 26.0, 5.0, GOLD, 0.95);
        c.text(r.x + 21.0, r.y + 21.0, &format!("{}", i + 1), 1.0, INK, 1.0);
        c.text_center(r.center().0, r.y + 186.0, u.name(), 2.0, GOLD, 1.0);
        c.text_wrap(
            r.x + 22.0,
            r.y + 222.0,
            r.w - 44.0,
            u.desc(),
            2.0,
            TEXT,
            1.0,
        );
        if let Some(k) = u.owner() {
            c.text_center(r.center().0, r.y + r.h - 26.0, k.name(), 1.0, DIM, 1.0);
        } else {
            c.text_center(r.center().0, r.y + r.h - 26.0, "Whole squad", 1.0, DIM, 1.0);
        }
    }
}

fn draw_route(c: &mut Canvas, app: &App, options: &[Node]) {
    c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x05030a, 0.6);
    let cx = W as f32 / 2.0;
    let next = app.run.as_ref().map(|r| r.island + 1).unwrap_or(1);
    c.text_center(cx, 110.0, "CHART YOUR COURSE", 5.0, GOLD, 1.0);
    c.text_center(
        cx,
        160.0,
        &format!("Island {next} of {ISLANDS}"),
        2.0,
        TEXT,
        1.0,
    );
    if let Some(run) = app.run.as_ref() {
        let hp: Vec<String> = run
            .squad
            .iter()
            .map(|m| format!("{} {}/{}", m.kind.name(), m.hp, m.max_hp))
            .collect();
        c.text_center(cx, 190.0, &hp.join("    "), 1.0, DIM, 1.0);
    }
    for (i, n) in options.iter().enumerate() {
        let r = route_rect(i, options.len());
        let hot = app.hover_button == Some(Button::Route(i));
        let r = Rect::new(r.x, r.y + if hot { -8.0 } else { 0.0 }, r.w, r.h);
        let pal = palette(n.biome());
        if hot {
            c.glow_ellipse(
                r.center().0,
                r.center().1,
                r.w * 0.8,
                r.h * 0.6,
                pal.accent,
                0.2,
            );
        }
        c.round_rect(r.x, r.y, r.w, r.h, 12.0, 0x120c18, 0.95);
        // Biome vignette: sky gradient and a tiny floating island.
        c.vgradient(
            r.x + 10.0,
            r.y + 10.0,
            r.w - 20.0,
            150.0,
            pal.sky_top,
            pal.sky_low,
            1.0,
        );
        let (ix, iy) = (r.center().0, r.y + 80.0);
        for k in 0..3 {
            let k = k as f32;
            c.poly(
                &[
                    (ix - 70.0 + k * 6.0, iy + k * 4.0),
                    (ix + 70.0 - k * 6.0, iy + k * 4.0),
                    (ix + 20.0, iy + 60.0 - k * 6.0),
                    (ix - 15.0, iy + 64.0 - k * 6.0),
                ],
                lerp_color(pal.side, pal.side_dark, k * 0.3),
                1.0,
            );
        }
        c.poly(
            &[
                (ix - 70.0, iy),
                (ix, iy - 26.0),
                (ix + 70.0, iy),
                (ix, iy + 26.0),
            ],
            pal.ground[1],
            1.0,
        );
        match n {
            Node::Sanctuary => {
                c.glow(ix, iy - 10.0, 60.0, 0xffe0a0, 0.6);
                c.circle(ix, iy - 12.0, 6.0, 0xfff6d0, 1.0);
            }
            Node::Throne => {
                c.glow(ix, iy - 20.0, 60.0, 0xff2040, 0.6);
                sprites::draw_unit(
                    c,
                    Kind::HollowKing,
                    ix,
                    iy + 6.0,
                    Look {
                        facing: -1.0,
                        t: app.time,
                        light: 0.8,
                        alpha: 1.0,
                        flash: 0.0,
                        shrouded: false,
                        wounded: false,
                    },
                );
            }
            Node::Elite(_) => {
                c.glow(ix, iy - 10.0, 40.0, 0xffd040, 0.4);
                sprites::brazier(c, ix, iy, true, 1.0, app.time, 3);
            }
            Node::Skirmish(_) => {
                sprites::brazier(c, ix, iy, true, 1.0, app.time, 1);
            }
        }
        c.round_rect_outline(
            r.x,
            r.y,
            r.w,
            r.h,
            12.0,
            if hot { pal.accent } else { 0x6a5a8a },
            1.0,
        );
        c.round_rect(r.x + 12.0, r.y + 12.0, 26.0, 26.0, 5.0, GOLD, 0.95);
        c.text(r.x + 21.0, r.y + 21.0, &format!("{}", i + 1), 1.0, INK, 1.0);
        c.text_center(
            r.center().0,
            r.y + 178.0,
            n.title(),
            3.0,
            if *n == Node::Throne { 0xff6a7a } else { GOLD },
            1.0,
        );
        c.text_center(
            r.center().0,
            r.y + 212.0,
            n.biome().name(),
            2.0,
            pal.accent,
            1.0,
        );
        c.text_wrap(
            r.x + 20.0,
            r.y + 236.0,
            r.w - 40.0,
            n.desc(),
            1.5,
            TEXT,
            1.0,
        );
    }
}

fn draw_end(c: &mut Canvas, app: &App) {
    c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x05030a, 0.7);
    let Some(run) = app.run.as_ref() else {
        return;
    };
    let cx = W as f32 / 2.0;
    let won = run.won();
    if won {
        c.glow_ellipse(cx, 150.0, 500.0, 120.0, 0xffc060, 0.35);
        c.text_center(cx, 110.0, "DAWN BREAKS", 7.0, 0xffe6a0, 1.0);
        c.text_center(
            cx,
            180.0,
            "The Hollow King is unmade. The islands remember the light.",
            2.0,
            TEXT,
            1.0,
        );
    } else {
        c.text_center(cx, 110.0, "THE LIGHT GOES OUT", 6.0, 0xff7a7a, 1.0);
        let cause = match &run.stage {
            Stage::Defeat { cause } => cause.clone(),
            _ => String::new(),
        };
        c.text_center(
            cx,
            180.0,
            &format!("{cause} on island {}.", run.island),
            2.0,
            TEXT,
            1.0,
        );
    }
    let r = Rect::new(cx - 300.0, 230.0, 600.0, 340.0);
    panel(c, r, 1.0);
    let rows = [
        ("Final score", format!("{}", run.score)),
        ("Islands reached", format!("{}/{}", run.island, ISLANDS)),
        ("Foes destroyed", format!("{}", run.kills)),
        ("Best chain", format!("x{}", run.best_chain)),
        ("Boons gathered", format!("{}", run.upgrades.len())),
        ("Best ever", format!("{}", app.save.best_score)),
        (
            "Runs / victories",
            format!("{} / {}", app.save.runs, app.save.wins),
        ),
    ];
    for (i, (k, v)) in rows.iter().enumerate() {
        let y = r.y + 24.0 + i as f32 * 36.0;
        c.text_shadow(r.x + 40.0, y, k, 2.0, DIM, 1.0);
        c.text_shadow(
            r.x + r.w - 40.0 - Canvas::text_width(v, 2.0),
            y,
            v,
            2.0,
            TEXT,
            1.0,
        );
    }
    if app.new_best {
        let p = 0.7 + 0.3 * (app.time * 5.0).sin();
        c.text_center(cx, r.y + r.h - 50.0, "NEW BEST SCORE!", 3.0, GOLD, p);
    }
    button(
        c,
        CONTINUE,
        "Continue",
        app.hover_button == Some(Button::Continue),
        true,
        GOLD,
    );
}

pub fn draw_help(c: &mut Canvas, app: &App) {
    c.fill_rect(0.0, 0.0, W as f32, H as f32, 0x05030a, 0.85);
    let r = Rect::new(90.0, 40.0, 1100.0, 640.0);
    panel(c, r, 1.0);
    c.text_center(W as f32 / 2.0, r.y + 20.0, "HOW TO PLAY", 4.0, GOLD, 1.0);
    let left = [
        (
            "GOAL",
            "Lead three lantern-bearers across 7 islands and destroy the Hollow King. Clear each island of foes.",
        ),
        (
            "TURNS",
            "Each squad member may MOVE once and ACT once per turn, in any order. Then the dark moves.",
        ),
        (
            "LIGHT",
            "Lanterns light around each hero; braziers and flares light more. Foes in darkness are SHROUDED: they take half damage.",
        ),
        (
            "SHOVES",
            "Shield Bash shoves foes. Into the void or water they die instantly. Into walls or units: +1 damage.",
        ),
        (
            "BONUSES",
            "+1 damage from higher ground, +1 when an ally flanks the target. Ranged attacks reach +1 from high ground.",
        ),
        (
            "PRESSURE",
            "Shades crawl in every 4 rounds. The gloom dims lanterns at rounds 5 and 9. From round 14, the long night bites.",
        ),
    ];
    let mut y = r.y + 70.0;
    for (h, t) in left {
        c.text_shadow(r.x + 30.0, y, h, 2.0, 0x9fe0ff, 1.0);
        y += 20.0;
        y += c.text_wrap(r.x + 30.0, y, 500.0, t, 1.0, TEXT, 1.0) + 12.0;
    }
    let right = [
        ("CONTROLS", ""),
        ("Click unit / TAB", "select squad member"),
        ("Click blue tile", "move (U to undo)"),
        ("1 2 3 / click", "choose ability, click target"),
        ("Click a foe", "quick-attack with first ability"),
        ("Hover a foe", "see its reach next turn"),
        ("D", "toggle all danger zones"),
        ("SPACE / ENTER", "end turn"),
        ("ESC / right click", "cancel"),
        ("M", "mute    H: this help"),
    ];
    let mut y = r.y + 70.0;
    for (k, v) in right {
        if v.is_empty() {
            c.text_shadow(r.x + 580.0, y, k, 2.0, 0x9fe0ff, 1.0);
            y += 26.0;
            continue;
        }
        c.text_shadow(r.x + 580.0, y, k, 1.0, GOLD, 1.0);
        c.text_shadow(r.x + 760.0, y, v, 1.0, TEXT, 1.0);
        y += 18.0;
    }
    y += 14.0;
    c.text_shadow(r.x + 580.0, y, "THE DARK", 2.0, 0xff9a8a, 1.0);
    y += 24.0;
    for k in Kind::ENEMIES {
        c.text_shadow(r.x + 580.0, y, k.name(), 1.0, 0xffc0b0, 1.0);
        y += c.text_wrap(r.x + 720.0, y, 360.0, k.blurb(), 1.0, TEXT, 1.0) + 4.0;
    }
    c.text_center(
        W as f32 / 2.0,
        r.y + r.h - 30.0,
        "Click or press H to close",
        1.0,
        DIM,
        (app.time * 3.0).sin() * 0.3 + 0.7,
    );
}
