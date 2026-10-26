//! Software illustration, typography and hit regions shared by the native app and screenshot mode.
use crate::*;
use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use std::collections::HashMap;
pub const W: usize = 1440;
pub const H: usize = 900;
const CREAM: u32 = 0xF4EEDD;
const INK: u32 = 0x243E35;
const GREEN: u32 = 0x41674E;
const MUTED: u32 = 0x73765F;
const GOLD: u32 = 0xB38D4F;
const RED: u32 = 0xAA5140;
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }
}
pub struct Hit {
    pub rect: Rect,
    pub action: Action,
}
pub struct Sprite {
    w: usize,
    h: usize,
    pixels: Vec<u8>,
}
pub struct View {
    pub pixels: Vec<u32>,
    display: FontArc,
    body: FontArc,
    art: HashMap<&'static str, Sprite>,
    pub hits: Vec<Hit>,
    hover: Action,
}
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0., 1.);
    let r = ((a >> 16 & 255) as f32 * (1. - t) + (b >> 16 & 255) as f32 * t) as u32;
    let g = ((a >> 8 & 255) as f32 * (1. - t) + (b >> 8 & 255) as f32 * t) as u32;
    let b = ((a & 255) as f32 * (1. - t) + (b & 255) as f32 * t) as u32;
    r << 16 | g << 8 | b
}
impl Default for View {
    fn default() -> Self {
        Self::new()
    }
}
impl View {
    pub fn new() -> Self {
        let mut art = HashMap::new();
        macro_rules! load {($($name:literal),*)=>{$(
            let tree=resvg::usvg::Tree::from_data(include_bytes!(concat!("../assets/art/",$name,".svg")),&resvg::usvg::Options::default()).unwrap();
            let mut pix=resvg::tiny_skia::Pixmap::new(288,288).unwrap();
            let transform=resvg::tiny_skia::Transform::from_scale(288./tree.size().width(),288./tree.size().height());
            resvg::render(&tree,transform,&mut pix.as_mut());
            let mut data=pix.data().to_vec();
            // tiny-skia stores premultiplied alpha; the sprite sampler expects straight RGBA.
            for px in data.chunks_mut(4){if px[3]>0{for k in 0..3{px[k]=(px[k] as u32*255/px[3] as u32).min(255) as u8;}}}
            art.insert($name,Sprite{w:288,h:288,pixels:data});
        )*}}
        load!(
            "seed",
            "sunflower",
            "rose",
            "tulip",
            "clover",
            "herb",
            "mushroom",
            "cactus",
            "cherry",
            "lotus",
            "tree",
            "bee",
            "butterfly",
            "beetle",
            "snail",
            "spider",
            "web",
            "locust",
            "snow",
            "fire",
            "water",
            "sun",
            "moon",
            "leaf",
            "shears",
            "spark",
            "jar",
            "ladybug"
        );
        Self {
            pixels: vec![0; W * H],
            display: FontArc::try_from_slice(include_bytes!("../assets/fonts/Fraunces.ttf"))
                .unwrap(),
            body: FontArc::try_from_slice(include_bytes!("../assets/fonts/Lato.ttf")).unwrap(),
            art,
            hits: vec![],
            hover: Action::None,
        }
    }
    fn pixel(&mut self, x: i32, y: i32, c: u32, a: f32) {
        if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
            let k = y as usize * W + x as usize;
            self.pixels[k] = mix(self.pixels[k], c, a);
        }
    }
    fn rect(&mut self, r: Rect, c: u32) {
        let x0 = r.x.max(0.) as usize;
        let x1 = (r.x + r.w).clamp(0., W as f32) as usize;
        let y0 = r.y.max(0.) as usize;
        let y1 = (r.y + r.h).clamp(0., H as f32) as usize;
        for y in y0..y1 {
            self.pixels[y * W + x0..y * W + x1].fill(c);
        }
    }
    fn round(&mut self, r: Rect, radius: f32, c: u32) {
        self.rect(Rect::new(r.x + radius, r.y, r.w - 2. * radius, r.h), c);
        self.rect(Rect::new(r.x, r.y + radius, r.w, r.h - 2. * radius), c);
        for (x, y) in [
            (r.x + radius, r.y + radius),
            (r.x + r.w - radius, r.y + radius),
            (r.x + radius, r.y + r.h - radius),
            (r.x + r.w - radius, r.y + r.h - radius),
        ] {
            self.circle(x, y, radius, c, 1.);
        }
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, c: u32, a: f32) {
        for yy in (y - r).max(0.) as i32..=(y + r).min(H as f32 - 1.) as i32 {
            let d = ((r * r - (yy as f32 - y).powi(2)).max(0.)).sqrt();
            for xx in (x - d).max(0.) as i32..=(x + d).min(W as f32 - 1.) as i32 {
                self.pixel(xx, yy, c, a);
            }
        }
    }
    fn line(&mut self, x: f32, y: f32, xx: f32, yy: f32, c: u32, width: f32) {
        let n = (xx - x).abs().max((yy - y).abs()).max(1.) as usize;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.circle(x + (xx - x) * t, y + (yy - y) * t, width / 2., c, 1.);
        }
    }
    fn text_width(&self, s: &str, size: f32, serif: bool) -> f32 {
        let font = if serif { &self.display } else { &self.body };
        let scale = font.as_scaled(size);
        s.chars().map(|c| scale.h_advance(font.glyph_id(c))).sum()
    }
    fn text(&mut self, s: &str, x: f32, y: f32, size: f32, c: u32, serif: bool) {
        let font = if serif { &self.display } else { &self.body };
        let scale = font.as_scaled(PxScale::from(size));
        let mut caret = point(x, y + scale.ascent());
        let mut glyphs = Vec::new();
        for ch in s.chars() {
            let id = font.glyph_id(ch);
            let g = id.with_scale_and_position(size, caret);
            caret.x += scale.h_advance(id);
            if let Some(out) = font.outline_glyph(g) {
                glyphs.push(out);
            }
        }
        for g in glyphs {
            let b = g.px_bounds();
            g.draw(|x, y, a| {
                self.pixel(b.min.x as i32 + x as i32, b.min.y as i32 + y as i32, c, a);
            });
        }
    }
    fn center(&mut self, s: &str, x: f32, y: f32, size: f32, c: u32, serif: bool) {
        let w = self.text_width(s, size, serif);
        self.text(s, x - w / 2., y, size, c, serif);
    }
    fn wrap(&mut self, s: &str, x: f32, y: f32, w: f32, size: f32, c: u32) -> f32 {
        let mut line = String::new();
        let mut yy = y;
        for word in s.split_whitespace() {
            let candidate = if line.is_empty() {
                word.into()
            } else {
                format!("{line} {word}")
            };
            if self.text_width(&candidate, size, false) > w && !line.is_empty() {
                self.text(&line, x, yy, size, c, false);
                yy += size * 1.28;
                line = word.into();
            } else {
                line = candidate
            }
        }
        if !line.is_empty() {
            self.text(&line, x, yy, size, c, false);
            yy += size * 1.28;
        }
        yy
    }
    fn icon(&mut self, name: &str, x: f32, y: f32, size: f32, alpha: f32) {
        let Some(s) = self.art.get(name) else { return };
        for yy in 0..size as usize {
            let dy = y as i32 + yy as i32;
            if dy < 0 || dy >= H as i32 {
                continue;
            }
            let sy = (yy as f32 / size * (s.h - 1) as f32).clamp(0., (s.h - 1) as f32);
            let y0 = sy as usize;
            let y1 = (y0 + 1).min(s.h - 1);
            let fy = sy.fract();
            for xx in 0..size as usize {
                let dx = x as i32 + xx as i32;
                if dx < 0 || dx >= W as i32 {
                    continue;
                }
                let sx = (xx as f32 / size * (s.w - 1) as f32).clamp(0., (s.w - 1) as f32);
                let x0 = sx as usize;
                let x1 = (x0 + 1).min(s.w - 1);
                let fx = sx.fract();
                let samples = [
                    ((y0 * s.w + x0) * 4, (1. - fx) * (1. - fy)),
                    ((y0 * s.w + x1) * 4, fx * (1. - fy)),
                    ((y1 * s.w + x0) * 4, (1. - fx) * fy),
                    ((y1 * s.w + x1) * 4, fx * fy),
                ];
                let mut rgba = [0_f32; 4];
                for (k, w) in samples {
                    let a = s.pixels[k + 3] as f32 / 255.;
                    rgba[3] += a * w;
                    for (j, v) in rgba.iter_mut().take(3).enumerate() {
                        *v += s.pixels[k + j] as f32 * a * w;
                    }
                }
                if rgba[3] > 0.001 {
                    let c = ((rgba[0] / rgba[3]) as u32) << 16
                        | ((rgba[1] / rgba[3]) as u32) << 8
                        | (rgba[2] / rgba[3]) as u32;
                    let k = dy as usize * W + dx as usize;
                    self.pixels[k] = mix(self.pixels[k], c, rgba[3] * alpha);
                }
            }
        }
    }
    fn hit(&mut self, r: Rect, a: Action) {
        self.hits.push(Hit { rect: r, action: a });
    }
    fn button(&mut self, r: Rect, label: &str, a: Action, primary: bool) {
        let hovered = self.hover == a;
        let c = if primary {
            if hovered { 0x52775D } else { INK }
        } else if hovered {
            0xE2D8BF
        } else {
            0xEDE5D0
        };
        self.round(Rect::new(r.x, r.y + 3., r.w, r.h), 6., 0xD0C6AE);
        self.round(r, 6., c);
        self.center(
            label,
            r.x + r.w / 2.,
            r.y + (r.h - 22.) / 2.,
            22.,
            if primary { CREAM } else { INK },
            false,
        );
        self.hit(r, a);
    }
    pub fn action_at(&self, p: (f32, f32)) -> Action {
        self.hits
            .iter()
            .rev()
            .find(|h| h.rect.contains(p))
            .map_or(Action::None, |h| h.action)
    }
    pub fn render(&mut self, a: &App) {
        self.hover = a.hover;
        self.hits.clear();
        self.background(a);
        match a.game.screen {
            Screen::Title => self.title(a),
            Screen::Battle => self.battle(a),
            Screen::Reward => self.reward(a),
            Screen::Route => self.route(a),
            Screen::Prune => self.prune(a),
            Screen::Won | Screen::Lost => self.ending(a),
        }
        if a.overlay != Overlay::None {
            self.overlay(a);
        }
    }
    fn background(&mut self, a: &App) {
        let season = a.game.season();
        let top = [0xE5E6CA, 0xEADCB9, 0xE8D3BE, 0xDDE5DF][season];
        for y in 0..H {
            let t = y as f32 / H as f32;
            let c = mix(top, CREAM, (t * 1.5).min(1.));
            for x in 0..W {
                let noise = ((x.wrapping_mul(37) ^ y.wrapping_mul(67)) % 7) as f32 / 255.;
                self.pixels[y * W + x] = mix(c, 0xA7A28C, noise);
            }
        }
        // A quiet architectural layer: translucent glass panes, iron ribs and botanical margins.
        for k in 0..7 {
            let x = k as f32 * 240.;
            self.line(x, 0., x, 585., 0xD0D4B9, 2.);
            self.line(x, 0., 720., -150., 0xCBD0B5, 2.);
        }
        self.line(0., 134., 1440., 134., 0xD4D5BA, 2.);
        self.line(0., 585., 1440., 585., 0xD4D0BA, 2.);
        let t = if a.motion { a.time } else { 0. };
        for i in 0..18 {
            let x = (i as f32 * 107. + (t * 6. + i as f32).sin() * 8.) % 1440.;
            let y = 150. + ((i * 79) % 420) as f32 + (t * 0.7 + i as f32).sin() * 9.;
            self.circle(x, y, 2., GOLD, 0.16);
        }
        self.icon("herb", -80., 340., 270., 0.16);
        self.icon("herb", 1265., 275., 280., 0.15);
        let specimen = ["cherry", "sunflower", "leaf", "snow"][season];
        self.icon(specimen, 1300., 70., 140., 0.18);
        if season == 0 {
            for i in 0..20 {
                let x = (i * 83) as f32 % 1440.;
                let y = 140. + ((i * 71) as f32 + t * 30.) % 400.;
                self.line(x, y, x - 5., y + 17., 0xCFD9C5, 1.);
            }
        }
        if season == 3 {
            for i in 0..28 {
                let x = ((i * 67) as f32 + (t * 0.7 + i as f32).sin() * 12.) % 1440.;
                let y = 140. + ((i * 71) as f32 + t * 14.) % 400.;
                self.circle(x, y, 2.5, 0xFFFFFF, 0.65);
            }
        }
    }
    fn top(&mut self, a: &App) {
        let g = &a.game;
        self.rect(Rect::new(0., 0., 1440., 80.), INK);
        self.text("THE LAST GLASSHOUSE", 30., 15., 26., CREAM, true);
        self.text(
            &format!(
                "{}  /  ENCOUNTER {:02} OF 18",
                SEASONS[g.season()].1,
                g.stage + 1
            ),
            31.,
            49.,
            14.,
            0xC5CEB5,
            false,
        );
        self.icon("rose", 385., 19., 34., 1.);
        self.text(
            &format!("{} / {}", g.hp, g.max_hp),
            427.,
            22.,
            28.,
            CREAM,
            false,
        );
        self.rect(Rect::new(427., 57., 140., 4.), 0x53705C);
        self.rect(
            Rect::new(427., 57., 140. * g.hp as f32 / g.max_hp as f32, 4.),
            0xD6A27B,
        );
        self.text(
            &format!("{}", g.battle.shield),
            605.,
            20.,
            30.,
            CREAM,
            false,
        );
        self.text("SHELTER", 600., 54., 12., 0xBFCBB5, false);
        self.text(
            &format!("{}", g.battle.energy),
            713.,
            20.,
            30.,
            0xE9C66D,
            false,
        );
        self.text("ACTIONS", 702., 54., 12., 0xBFCBB5, false);
        self.text(&format!("{:04}", g.score), 807., 20., 28., CREAM, false);
        self.text("POLLEN SCORE", 802., 54., 12., 0xBFCBB5, false);
        self.button(
            Rect::new(977., 18., 120., 43.),
            "Field guide",
            Action::Codex,
            false,
        );
        self.button(
            Rect::new(1110., 18., 115., 43.),
            "How to play",
            Action::Help,
            false,
        );
        self.button(
            Rect::new(1238., 18., 170., 43.),
            "Pause / settings",
            Action::Pause,
            false,
        );
        for i in 0..18 {
            let x = 37. + i as f32 * 25.;
            self.circle(
                x,
                105.,
                if i == g.stage { 7. } else { 4. },
                if i <= g.stage { GREEN } else { 0xC3C7B0 },
                1.,
            );
            if matches!(i, 5 | 11 | 17) {
                self.circle(x, 105., 2., CREAM, 1.);
            }
        }
        for (i, &r) in g.relics.iter().enumerate() {
            let x = 1030. + i as f32 * 37.;
            self.icon(RELICS[r].1, x, 91., 26., 1.);
            self.hit(Rect::new(x - 2., 87., 34., 36.), Action::Relic(r));
        }
        self.text(SEASONS[g.season()].2, 530., 95., 18., MUTED, false);
    }
    fn title(&mut self, a: &App) {
        self.text("A BOTANICAL DECKBUILDER", 85., 85., 17., GREEN, false);
        self.text("The Last", 80., 144., 88., INK, true);
        self.text("Glasshouse", 78., 235., 108., INK, true);
        self.rect(Rect::new(85., 365., 110., 3.), GOLD);
        self.wrap(
            "One little garden. One impossible winter.",
            85.,
            393.,
            580.,
            29.,
            INK,
        );
        self.wrap("Plant a hand of seeds. Let them grow. Find the moment to harvest. Keep the last warm place alive.",85.,448.,520.,23.,MUTED);
        self.button(
            Rect::new(85., 566., 345., 62.),
            "Open the glasshouse  →",
            Action::Start,
            true,
        );
        self.button(
            Rect::new(85., 649., 165., 49.),
            "Daily seed",
            Action::Daily,
            false,
        );
        self.button(
            Rect::new(267., 649., 163., 49.),
            "How to play",
            Action::Help,
            false,
        );
        self.button(
            Rect::new(85., 719., 165., 46.),
            "Fullscreen",
            Action::Fullscreen,
            false,
        );
        self.button(
            Rect::new(267., 719., 163., 46.),
            if a.game.easy {
                "Gentle: on"
            } else {
                "Gentle: off"
            },
            Action::Assist,
            false,
        );
        if a.save.wins > 0 {
            self.button(
                Rect::new(85., 788., 345., 44.),
                "Start with the wild seed deck",
                Action::Alternate,
                false,
            );
        } else {
            self.text(
                "18 encounters · 4 seasons · a garden of your own",
                85.,
                806.,
                18.,
                MUTED,
                false,
            );
        }
        self.title_garden(a, 945., 445.);
        self.text("FIELD NOTES  /  VOL. VII", 1050., 42., 16., MUTED, false);
        self.text(
            &format!(
                "BEST {:04}   ·   {} GARDENS SAVED",
                a.save.best, a.save.wins
            ),
            890.,
            801.,
            18.,
            GREEN,
            false,
        );
        self.text(
            "Click to play · right-click to compost · Esc for settings",
            85.,
            860.,
            16.,
            MUTED,
            false,
        );
        self.text(
            "Twemoji artwork: CC BY 4.0 · credits in README",
            995.,
            868.,
            13.,
            MUTED,
            false,
        );
    }
    fn title_garden(&mut self, a: &App, x: f32, y: f32) {
        self.round(Rect::new(x - 254., y - 280., 510., 610.), 250., 0xCDD5B8);
        self.round(Rect::new(x - 237., y - 263., 476., 582.), 230., 0xDEE1C9);
        self.line(x, y - 260., x, y + 220., 0xA7B294, 4.);
        self.line(x - 236., y - 15., x + 236., y - 15., 0xA7B294, 4.);
        self.line(x - 230., y + 160., x + 230., y + 160., 0xA7B294, 4.);
        let sway = if a.motion {
            (a.time * 0.8).sin() * 5.
        } else {
            0.
        };
        self.icon("tree", x - 160., y - 130., 310., 0.15);
        self.icon("herb", x - 242., y + 35., 250., 1.);
        self.icon("rose", x - 113. + sway, y - 100., 234., 1.);
        self.icon("sunflower", x + 28. - sway, y - 190., 228., 1.);
        self.icon("lotus", x - 35., y + 115., 190., 1.);
        self.icon("mushroom", x + 135., y + 155., 105., 1.);
        self.icon("butterfly", x - 180. + sway * 3., y - 222., 83., 1.);
        self.icon("bee", x + 140. - sway * 2., y - 37., 60., 1.);
        self.round(Rect::new(x - 263., y + 301., 530., 28.), 4., 0x8C9675);
        self.round(Rect::new(x - 279., y + 324., 562., 16.), 4., 0xBEC5A5);
        self.text("01", x - 227., y + 271., 16., GREEN, false);
        self.text("THE RAIN ROOMS", x - 192., y + 270., 17., GREEN, false);
    }
    fn battle(&mut self, a: &App) {
        self.top(a);
        let g = &a.game;
        let b = &g.battle;
        self.text(
            &format!("YOUR GARDEN  /  TURN {:02}", b.turn),
            42.,
            158.,
            18.,
            GREEN,
            false,
        );
        self.text(
            "Mature neighbours add +3 damage to a harvest.",
            42.,
            186.,
            17.,
            MUTED,
            false,
        );
        for i in 0..3 {
            self.bed_panel(a, i);
        }
        self.text("THE UNINVITED", 867., 158., 18., RED, false);
        self.text(
            "Click a pest to target. Intentions resolve at end turn.",
            867.,
            186.,
            17.,
            MUTED,
            false,
        );
        let n = b.enemies.len();
        let gap = if n == 1 { 0. } else { 167. };
        for (i, e) in b.enemies.iter().enumerate() {
            let x = if n == 1 { 1050. } else { 924. + i as f32 * gap };
            let selected = b.target == i;
            let dead = e.hp <= 0;
            let bob = if a.motion && !dead {
                (a.time * 1.7 + i as f32 * 2.).sin() * 5.
            } else {
                0.
            };
            if selected && !dead {
                self.round(Rect::new(x - 71., 251., 146., 191.), 70., 0xE2D5BC);
                self.line(x - 53., 451., x + 53., 451., RED, 2.);
            }
            self.icon(
                FOES[e.kind].1,
                x - 54.,
                275. + bob,
                108.,
                if dead { 0.16 } else { 1. },
            );
            self.center(
                FOES[e.kind].0,
                x,
                222.,
                18.,
                if dead { MUTED } else { INK },
                true,
            );
            self.rect(Rect::new(x - 59., 405., 118., 5.), 0xD1C9AE);
            self.rect(
                Rect::new(x - 59., 405., 118. * e.hp.max(0) as f32 / e.max as f32, 5.),
                if e.kind >= 6 { RED } else { GREEN },
            );
            self.center(
                &format!("{} / {}", e.hp.max(0), e.max),
                x,
                421.,
                17.,
                INK,
                false,
            );
            if !dead {
                self.center(&e.intent.label(), x, 467., 18., RED, false);
                if e.poison > 0 || e.armour > 0 {
                    self.center(
                        &format!("{} poison  ·  {} armour", e.poison, e.armour),
                        x,
                        496.,
                        14.,
                        MUTED,
                        false,
                    );
                }
                self.hit(Rect::new(x - 77., 213., 154., 300.), Action::Target(i));
            } else {
                self.center("CLEARED", x, 465., 14., MUTED, false);
            }
        }
        self.text(
            &format!("Incoming: {} health after shelter", g.expected_damage()),
            867.,
            548.,
            20.,
            if g.expected_damage() > 0 { RED } else { GREEN },
            false,
        );
        self.message(a);
        // Cards sit on a continuous table, with all text above the lower edge.
        self.rect(Rect::new(0., 638., 1440., 262.), 0xDDD3B9);
        self.rect(Rect::new(0., 638., 1440., 3.), 0xB7AF91);
        let len = b.hand.len();
        let cw = if len > 7 { 139. } else { 160. };
        let step = if len > 7 { 142. } else { 164. };
        let start = 28. + ((1170. - len as f32 * step) / 2.).max(0.);
        for (i, c) in b.hand.iter().enumerate() {
            let sel = b.selected == Some(i);
            let hover = a.hover == Action::Card(i);
            let y = if sel || hover { 646. } else { 662. };
            self.card(
                *c,
                Rect::new(start + i as f32 * step, y, cw, 213.),
                Action::Card(i),
                b.energy >= CARDS[c.id].cost,
                sel,
            );
            self.text(
                &(i + 1).to_string(),
                start + i as f32 * step + cw - 17.,
                y + 188.,
                12.,
                MUTED,
                false,
            );
        }
        self.button(
            Rect::new(1221., 663., 189., 57.),
            "End turn  /  Space",
            Action::EndTurn,
            true,
        );
        self.button(
            Rect::new(1221., 735., 189., 43.),
            "Undo decision  /  U",
            Action::Undo,
            false,
        );
        self.text(
            &format!("Draw {}  ·  Discard {}", b.draw.len(), b.discard.len()),
            1224.,
            795.,
            17.,
            INK,
            false,
        );
        self.text(
            &format!("Exhausted {}", b.exhausted),
            1224.,
            821.,
            16.,
            MUTED,
            false,
        );
        self.wrap(
            if b.composted {
                "COMPOST USED"
            } else {
                "Right-click a card: +1 action. Once per turn."
            },
            1224.,
            851.,
            185.,
            14.,
            if b.composted { MUTED } else { GREEN },
        );
    }
    fn bed_panel(&mut self, a: &App, i: usize) {
        let g = &a.game;
        let x = 42. + i as f32 * 255.;
        let y = 240.;
        let r = Rect::new(x, y, 231., 310.);
        let selected = matches!(a.hover,Action::Bed(j)if j==i);
        self.round(Rect::new(x, y + 5., 231., 310.), 110., 0xB9BFA2);
        self.round(r, 110., if selected { 0xF8F3E2 } else { 0xE9E7CE });
        self.line(x + 16., y + 277., x + 215., y + 277., 0xB4B599, 2.);
        self.center(
            &format!("BED 0{}", i + 1),
            x + 115.,
            y + 285.,
            14.,
            GREEN,
            false,
        );
        if let Some(p) = &g.battle.beds[i] {
            let spec = CARDS[p.card.id];
            let mature = p.mature();
            self.center(spec.name, x + 115., y + 28., 23., INK, true);
            let sway = if a.motion {
                (a.time * 1.5 + i as f32).sin() * 3.
            } else {
                0.
            };
            if mature {
                self.circle(x + 115., y + 147., 85., 0xE7C15E, 0.12);
            }
            let scale = if mature {
                128.
            } else {
                86. + p.growth as f32 * 9.
            };
            self.icon(
                spec.icon,
                x + 115. - scale / 2. + sway,
                y + 77. + (130. - scale),
                scale,
                1.,
            );
            if mature {
                for n in 0..6 {
                    let angle = n as f32 / 6. * std::f32::consts::TAU + a.time * 0.3;
                    self.circle(
                        x + 115. + angle.cos() * 75.,
                        y + 144. + angle.sin() * 64.,
                        2.,
                        GOLD,
                        0.65,
                    );
                }
            }
            for n in 0..spec.grow {
                self.round(
                    Rect::new(
                        x + 115. - spec.grow as f32 * 13. + n as f32 * 26.,
                        y + 228.,
                        20.,
                        7.,
                    ),
                    3.,
                    if n < p.growth { GREEN } else { 0xC6CCAE },
                );
            }
            self.center(
                if mature { "HARVEST  →" } else { "GROWING" },
                x + 115.,
                y + 246.,
                17.,
                if mature { GREEN } else { MUTED },
                false,
            );
        } else {
            self.icon("seed", x + 80., y + 90., 70., 0.25);
            self.center("A little possibility", x + 115., y + 195., 19., MUTED, true);
            self.center(
                if g.battle.selected.is_some() {
                    "CLICK TO PLANT"
                } else {
                    "Play a seed here"
                },
                x + 115.,
                y + 233.,
                16.,
                GREEN,
                false,
            );
        }
        self.hit(r, Action::Bed(i));
    }
    fn message(&mut self, a: &App) {
        let g = &a.game;
        let note = if g.stage == 0 && g.tutorial < 4 {
            match g.tutorial {
                0 => "01 / Click a seed card, then an empty bed. Planting costs 1 action.",
                1 => "02 / Play Water on a bed. A full growth ring means it is ripe.",
                2 => "03 / Click a ripe plant to harvest. Harvesting is free!",
                _ => "04 / Shelter blocks pest bites. Space ends your turn and grows every bed.",
            }
        } else {
            &g.message
        };
        self.icon(
            if g.impact >= 3 { "spark" } else { "leaf" },
            42.,
            591.,
            24.,
            1.,
        );
        let hover_note = match a.hover {
            Action::Card(i) if i < g.battle.hand.len() => Some(format!(
                "{} — {}",
                CARDS[g.battle.hand[i].id].name,
                g.battle.hand[i].description()
            )),
            Action::Relic(r) => Some(format!("{} — {}", RELICS[r].0, RELICS[r].2)),
            Action::Bed(i) => g.battle.beds[i].as_ref().map(|p| {
                format!(
                    "{} — {}  Growth {}/{}.",
                    CARDS[p.card.id].name,
                    p.card.description(),
                    p.growth,
                    CARDS[p.card.id].grow
                )
            }),
            Action::Target(i) if i < g.battle.enemies.len() => Some(format!(
                "{} — {}. Armour blocks each hit; poison ticks BEFORE bites.",
                FOES[g.battle.enemies[i].kind].0,
                g.battle.enemies[i].intent.label()
            )),
            _ => None,
        };
        self.text(
            hover_note.as_deref().unwrap_or(note),
            78.,
            592.,
            19.,
            INK,
            false,
        );
        if a.motion {
            let age = a.time - a.event_time;
            if age < 0.65 && g.impact > 0 {
                for i in 0..20 {
                    let angle = i as f32 * 2.399;
                    let r = age * 130. + 15.;
                    self.circle(
                        715. + angle.cos() * r,
                        390. + angle.sin() * r - age * 80.,
                        (3. - age * 3.).max(1.),
                        if g.impact == 4 { RED } else { GOLD },
                        (1. - age / 0.65).max(0.),
                    );
                }
            }
        }
    }
    fn card(&mut self, c: Card, r: Rect, action: Action, enabled: bool, selected: bool) {
        let spec = CARDS[c.id];
        let large = r.h > 300.;
        let hover = self.hover == action;
        let border = if selected {
            GREEN
        } else if c.gold {
            GOLD
        } else {
            0xC0B398
        };
        self.round(Rect::new(r.x + 2., r.y + 5., r.w, r.h), 8., 0xC2B69B);
        self.round(r, 8., border);
        self.round(
            Rect::new(r.x + 2., r.y + 2., r.w - 4., r.h - 4.),
            7.,
            if hover || selected {
                0xFFFCF0
            } else {
                0xF9F3E3
            },
        );
        self.rect(
            Rect::new(
                r.x + 9.,
                r.y + 43.,
                r.w - 18.,
                if large { 173. } else { 73. },
            ),
            if c.id < 10 { 0xDCE4C6 } else { 0xE7DFC8 },
        );
        self.icon(
            spec.icon,
            r.x + r.w / 2. - if large { 72. } else { 32. },
            r.y + 46.,
            if large { 144. } else { 64. },
            if enabled { 1. } else { 0.5 },
        );
        self.circle(
            r.x + 22.,
            r.y + 22.,
            14.,
            if enabled { GREEN } else { MUTED },
            1.,
        );
        self.center(
            &spec.cost.to_string(),
            r.x + 22.,
            r.y + 10.,
            22.,
            CREAM,
            false,
        );
        let name = if c.gold {
            format!("{} +", spec.name)
        } else {
            spec.name.into()
        };
        let name_size = if self.text_width(&name, 18., true) > r.w - 49. {
            15.
        } else {
            18.
        };
        self.text(&name, r.x + 43., r.y + 12., name_size, INK, true);
        let desc = c.description();
        let mut body_size = if large { 23. } else { 17. };
        let available = r.h - if large { 235. } else { 125. } - 28.;
        loop {
            let mut lines = 1;
            let mut width = 0.;
            for word in desc.split_whitespace() {
                let w = self.text_width(word, body_size, false) + body_size * 0.25;
                if width + w > r.w - 24. {
                    lines += 1;
                    width = w;
                } else {
                    width += w;
                }
            }
            if lines as f32 * body_size * 1.28 <= available || body_size <= 13. {
                break;
            }
            body_size -= 1.;
        }
        self.wrap(
            &desc,
            r.x + 12.,
            r.y + if large { 235. } else { 125. },
            r.w - 24.,
            body_size,
            if enabled { INK } else { MUTED },
        );
        self.text(
            if c.id < 10 {
                "SPECIMEN"
            } else if matches!(c.id, 14 | 25 | 26 | 27) {
                "ONE USE / ENCOUNTER"
            } else {
                "TENDING"
            },
            r.x + 12.,
            r.y + r.h - 23.,
            11.,
            if c.gold { GOLD } else { GREEN },
            false,
        );
        self.hit(r, action);
    }
    fn page_heading(&mut self, kicker: &str, title: &str, sub: &str) {
        self.text(kicker, 70., 63., 17., GREEN, false);
        self.text(title, 66., 104., 62., INK, true);
        self.wrap(sub, 70., 185., 1250., 23., MUTED);
    }
    fn reward(&mut self, a: &App) {
        self.page_heading("A ROOM RECLAIMED","Something worth keeping.","Take one cutting for your deck. It will appear in your next encounter. Or keep your deck lean.");
        let g = &a.game;
        for i in 0..3 {
            self.card(
                g.rewards[i],
                Rect::new(245. + i as f32 * 330., 285., 280., 370.),
                Action::Reward(i),
                true,
                false,
            );
            self.button(
                Rect::new(245. + i as f32 * 330., 688., 280., 55.),
                "Take this cutting",
                Action::Reward(i),
                i == 0,
            );
        }
        self.button(
            Rect::new(540., 792., 360., 51.),
            "Skip / keep my deck",
            Action::Reward(3),
            false,
        );
        self.text(
            &format!(
                "Health {} / {}   ·   Deck {} cards   ·   Pollen {}",
                g.hp,
                g.max_hp,
                g.deck.len(),
                g.score
            ),
            70.,
            867.,
            18.,
            GREEN,
            false,
        );
        if matches!(g.stage, 5 | 11) && !g.relics.is_empty() {
            let r = *g.relics.last().unwrap();
            self.text(
                &format!("KEEPER'S RELIC: {} — {}", RELICS[r].0, RELICS[r].2),
                70.,
                238.,
                20.,
                GREEN,
                false,
            );
        }
    }
    fn route(&mut self, a: &App) {
        self.page_heading("THE PATH THROUGH","Three doors. Your choice.","Your plants begin afresh in the next room. Your cards, relics and health stay with you.");
        let opts = [
            (
                "The sheltered walk",
                "herb",
                "Heal 14 health. The next room has its normal pest population.",
                "Take shelter",
            ),
            (
                "The overgrown door",
                "bee",
                "Find a new relic. Every pest in the next room gains 5 health and +1 bite.",
                "Take the risk",
            ),
            (
                "The potting bench",
                "shears",
                "Remove a card, or graft one into a stronger gold version. Minimum deck size: 8.",
                "Tend the deck",
            ),
        ];
        for (i, (name, icon, desc, label)) in opts.iter().enumerate() {
            let x = 100. + i as f32 * 435.;
            self.round(Rect::new(x, 300., 370., 420.), 10., 0xE6E4CB);
            self.icon(icon, x + 123., 330., 124., 1.);
            self.center(name, x + 185., 485., 29., INK, true);
            self.wrap(desc, x + 30., 543., 310., 22., MUTED);
            self.button(
                Rect::new(x + 30., 643., 310., 53.),
                label,
                Action::Route(i),
                i == 0,
            );
        }
        self.text(
            &format!(
                "NEXT: ENCOUNTER {:02} / 18   ·   HEALTH {} / {}",
                a.game.stage + 2,
                a.game.hp,
                a.game.max_hp
            ),
            100.,
            791.,
            20.,
            GREEN,
            false,
        );
        self.relic_strip(a, 840.);
    }
    fn relic_strip(&mut self, a: &App, y: f32) {
        let g = &a.game;
        self.text("RELICS", 40., y + 8., 14., MUTED, false);
        for (i, &r) in g.relics.iter().enumerate() {
            let x = 126. + i as f32 * 142.;
            self.icon(RELICS[r].1, x, y, 32., 1.);
            self.hit(Rect::new(x, y, 138., 44.), Action::Relic(r));
            self.wrap(RELICS[r].0, x + 39., y, 96., 14., GREEN);
        }
    }
    fn prune(&mut self, a: &App) {
        self.page_heading("THE POTTING BENCH","Make room to grow.","Remove a card, or graft a stronger version. Choose one improvement, then continue to the next room.");
        for (i, c) in a.game.deck.iter().enumerate() {
            let col = i % 8;
            let row = i / 8;
            let x = 35. + col as f32 * 176.;
            let y = 265. + row as f32 * 145.;
            let r = Rect::new(x, y, 162., 127.);
            self.round(
                r,
                6.,
                if self.hover == Action::Prune(i) {
                    0xE5D4B8
                } else {
                    0xE5E5CD
                },
            );
            self.icon(CARDS[c.id].icon, x + 61., y + 9., 43., 1.);
            self.center(CARDS[c.id].name, x + 81., y + 64., 17., INK, true);
            self.center(
                if c.gold {
                    "UPGRADED  /  REMOVE"
                } else {
                    "REMOVE"
                },
                x + 81.,
                y + 76.,
                11.,
                RED,
                false,
            );
            self.hit(Rect::new(x, y, 162., 86.), Action::Prune(i));
            self.rect(Rect::new(x + 4., y + 88., 154., 35.), 0xD7DCC1);
            self.center(
                if c.gold { "ALREADY GRAFTED" } else { "GRAFT +" },
                x + 81.,
                y + 96.,
                14.,
                GREEN,
                false,
            );
            self.hit(Rect::new(x + 4., y + 88., 154., 35.), Action::Upgrade(i));
        }
    }
    fn ending(&mut self, a: &App) {
        let g = &a.game;
        let won = g.screen == Screen::Won;
        self.text(
            if won {
                "THE FIRST MORNING"
            } else {
                "THE LONG NIGHT"
            },
            90.,
            90.,
            19.,
            GREEN,
            false,
        );
        self.text(
            if won {
                "Spring, at last."
            } else {
                "Even gardens sleep."
            },
            85.,
            157.,
            74.,
            INK,
            true,
        );
        self.wrap(if won{"You carried a little warmth through every season. Somewhere beyond the glass, the world is beginning again."}else{"The pests wore through your shelter and your health reached zero. Your field notes survive. Another seed, another garden."},90.,276.,640.,26.,MUTED);
        self.icon(if won { "sunflower" } else { "leaf" }, 945., 200., 300., 1.);
        let stats = [
            ("POLLEN", g.score.to_string()),
            (
                "ENCOUNTERS",
                format!("{} / 18", if won { 18 } else { g.stage }),
            ),
            ("HARVESTS", g.total_harvests.to_string()),
            ("BEST CHAIN", g.peak_chain.to_string()),
        ];
        for (i, (label, value)) in stats.iter().enumerate() {
            let x = 90. + i as f32 * 185.;
            self.text(value, x, 466., 46., INK, true);
            self.text(label, x, 525., 15., MUTED, false);
        }
        self.button(
            Rect::new(90., 616., 350., 63.),
            "Plant another garden  →",
            Action::Restart,
            true,
        );
        self.button(
            Rect::new(90., 706., 165., 48.),
            "Field guide",
            Action::Codex,
            false,
        );
        self.button(
            Rect::new(273., 706., 167., 48.),
            "Fullscreen",
            Action::Fullscreen,
            false,
        );
        self.text(
            &format!(
                "PERSONAL BEST {:04}  ·  {} GARDENS SAVED",
                a.save.best, a.save.wins
            ),
            90.,
            803.,
            20.,
            GREEN,
            false,
        );
        if let Some(e) = &a.save_error {
            self.wrap(
                &format!("Could not save records: {e}"),
                90.,
                843.,
                1100.,
                16.,
                RED,
            );
        }
        self.relic_strip(a, 855.);
    }
    fn overlay(&mut self, a: &App) {
        for p in &mut self.pixels {
            *p = mix(*p, INK, 0.72);
        }
        self.hits.clear();
        let r = Rect::new(130., 65., 1180., 770.);
        self.round(r, 12., CREAM);
        self.button(
            Rect::new(1130., 85., 145., 45.),
            "Close / Esc",
            Action::Close,
            false,
        );
        match a.overlay {
            Overlay::Pause => {
                self.text("A moment in the shade.", 190., 145., 51., INK, true);
                self.wrap(
                    "The garden waits for you. Change settings at any time.",
                    190.,
                    223.,
                    900.,
                    23.,
                    MUTED,
                );
                self.button(
                    Rect::new(190., 300., 450., 60.),
                    "Return to the garden",
                    Action::Close,
                    true,
                );
                self.button(
                    Rect::new(190., 390., 450., 60.),
                    "Toggle fullscreen",
                    Action::Fullscreen,
                    false,
                );
                self.button(
                    Rect::new(190., 480., 450., 60.),
                    if a.muted {
                        "Music and sound: OFF"
                    } else {
                        "Music and sound: ON"
                    },
                    Action::Mute,
                    false,
                );
                self.button(
                    Rect::new(710., 300., 450., 60.),
                    if a.game.easy {
                        "Gentle mode: ON"
                    } else {
                        "Gentle mode: OFF"
                    },
                    Action::Assist,
                    false,
                );
                self.wrap(
                    "Gentle mode reduces incoming bites by one third. No score penalty.",
                    710.,
                    380.,
                    440.,
                    22.,
                    MUTED,
                );
                self.button(
                    Rect::new(710., 480., 450., 60.),
                    if a.motion {
                        "Motion and particles: ON"
                    } else {
                        "Motion and particles: OFF"
                    },
                    Action::Motion,
                    false,
                );
                self.button(
                    Rect::new(190., 650., 450., 58.),
                    "How to play",
                    Action::Help,
                    false,
                );
                self.button(
                    Rect::new(710., 650., 450., 58.),
                    "Quit",
                    Action::Quit,
                    false,
                );
                self.text(
                    "Resizable window · macOS green button · F fullscreen · M mute",
                    190.,
                    760.,
                    20.,
                    MUTED,
                    false,
                );
            }
            Overlay::Help => {
                self.text("A gardener's field notes", 184., 119., 48., INK, true);
                let notes = [
                    (
                        "01  Plant",
                        "Click a specimen card, then an empty bed. Actions pay for cards. Hand size is capped at 9.",
                    ),
                    (
                        "02  Tend",
                        "Water adds growth. Every bed also grows +1 after an enemy turn. Full rings mean a plant is mature.",
                    ),
                    (
                        "03  Harvest",
                        "Click a mature plant with no card selected. It is FREE. Mature adjacent plants add +3 damage. Use them in the right order.",
                    ),
                    (
                        "04  Shelter",
                        "Pest intentions are shown above the table. Shelter absorbs their bites, then resets next turn. Click pests to change target.",
                    ),
                    (
                        "05  Compost",
                        "Right-click a card for +1 action, once each turn. It goes to discard, so you can draw it again. Some cards exhaust for the encounter.",
                    ),
                    (
                        "06  Carry on",
                        "Space ends a turn. U undoes decisions until the enemy turn is committed. New rooms restore 4 health; the sheltered route heals 14 more.",
                    ),
                ];
                for (i, (head, body)) in notes.iter().enumerate() {
                    let col = i % 2;
                    let row = i / 2;
                    let x = 190. + col as f32 * 550.;
                    let y = 223. + row as f32 * 177.;
                    self.text(head, x, y, 28., GREEN, true);
                    self.wrap(body, x, y + 45., 490., 21., INK);
                }
                self.text("1–9 cards · click bed · right-click compost · Space end turn · U undo · H help · Esc pause",190.,777.,17.,MUTED,false);
            }
            Overlay::Codex => {
                self.text("The living collection", 181., 119., 48., INK, true);
                self.text("28 cuttings and tools. Gold-edged cards are stronger. Your run's relics are listed below.",185.,181.,20.,MUTED,false);
                for (i, s) in CARDS.iter().enumerate() {
                    let col = i % 7;
                    let row = i / 7;
                    let x = 183. + col as f32 * 157.;
                    let y = 239. + row as f32 * 121.;
                    self.icon(s.icon, x + 42., y, 49., 1.);
                    self.center(s.name, x + 67., y + 54., 16., INK, true);
                    self.center(
                        &format!("{} action{}", s.cost, if s.cost == 1 { "" } else { "s" }),
                        x + 67.,
                        y + 77.,
                        13.,
                        MUTED,
                        false,
                    );
                    self.hit(Rect::new(x, y, 140., 105.), Action::Card(100 + i));
                }
                self.relic_strip(a, 744.);
                if let Action::Card(i) = a.hover
                    && (100..128).contains(&i)
                {
                    let s = CARDS[i - 100];
                    self.rect(Rect::new(180., 798., 1090., 24.), CREAM);
                    self.text(s.text, 185., 798., 18., GREEN, false);
                }
            }
            Overlay::None => {}
        }
    }
    pub fn save_png(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W as u32, H as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header()?;
        let mut data = Vec::with_capacity(W * H * 3);
        for p in &self.pixels {
            data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        writer.write_image_data(&data)?;
        Ok(())
    }
}

/// Captures real game states; no screenshot-only text or game state overrides.
pub fn screenshots(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    let mut v = View::new();
    let mut a = App::new(7);
    a.save = Save::default();
    a.time = 2.;
    v.render(&a);
    v.save_png(&dir.join("01_title.png"))?;
    a.game.start();
    for _ in 0..4 {
        bot_step(&mut a.game);
    }
    v.render(&a);
    v.save_png(&dir.join("02_early_garden.png"))?;
    a.game = Game::new(7, false, false);
    a.game.start();
    let mut shots = [false; 4];
    for _ in 0..20000 {
        if matches!(a.game.screen, Screen::Won | Screen::Lost) {
            break;
        }
        bot_step(&mut a.game);
        a.time += 0.05;
        if a.game.screen == Screen::Reward && !shots[0] {
            v.render(&a);
            v.save_png(&dir.join("03_cutting_draft.png"))?;
            shots[0] = true;
        }
        if a.game.screen == Screen::Route && !shots[1] {
            v.render(&a);
            v.save_png(&dir.join("04_three_doors.png"))?;
            shots[1] = true;
        }
        if a.game.screen == Screen::Battle
            && a.game.stage == 11
            && a.game.battle.beds.iter().flatten().count() >= 1
            && !shots[2]
        {
            v.render(&a);
            v.save_png(&dir.join("05_keeper_of_ash.png"))?;
            shots[2] = true;
        }
        if a.game.screen == Screen::Battle
            && a.game.stage == 17
            && a.game.battle.turn >= 2
            && a.game.battle.beds.iter().flatten().count() >= 1
            && !shots[3]
        {
            v.render(&a);
            v.save_png(&dir.join("06_white_conservatory.png"))?;
            shots[3] = true;
        }
    }
    if !shots.iter().all(|b| *b) || a.game.screen != Screen::Won {
        return Err("Screenshot campaign did not reach every required moment".into());
    }
    a.save.record(&a.game);
    v.render(&a);
    v.save_png(&dir.join("07_ending.png"))?;
    a.overlay = Overlay::Help;
    v.render(&a);
    v.save_png(&dir.join("08_field_notes.png"))?;
    a.overlay = Overlay::Pause;
    v.render(&a);
    v.save_png(&dir.join("09_settings.png"))?;
    a.overlay = Overlay::Codex;
    v.render(&a);
    v.save_png(&dir.join("10_collection.png"))?;
    Ok(())
}
