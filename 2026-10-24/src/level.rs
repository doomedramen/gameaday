//! Level data: a tile grid plus spawn lists. Levels are immutable; the
//! simulation keeps all mutable state (broken glass, anchors, enemies) itself.

/// Tile edge in world pixels.
pub const T: f32 = 32.0;

pub type Tile = u8;
pub const AIR: Tile = 0;
pub const ROCK: Tile = 1;
pub const ICE: Tile = 2;
pub const GLASS: Tile = 3;
pub const CRUMBLE: Tile = 4;
pub const SPIKE_UP: Tile = 5;
pub const SPIKE_DOWN: Tile = 6;
pub const SPIKE_LEFT: Tile = 7;
pub const SPIKE_RIGHT: Tile = 8;

pub fn is_spike(t: Tile) -> bool {
    (SPIKE_UP..=SPIKE_RIGHT).contains(&t)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Biome {
    Mossbluff,
    Glimmerdeep,
    Frostspire,
    Cinderworks,
    Stormcrown,
    Beacon,
}

impl Biome {
    pub const RUN: [Biome; 5] = [
        Biome::Mossbluff,
        Biome::Glimmerdeep,
        Biome::Frostspire,
        Biome::Cinderworks,
        Biome::Stormcrown,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Biome::Mossbluff => "Mossbluff",
            Biome::Glimmerdeep => "Glimmerdeep",
            Biome::Frostspire => "Frostspire",
            Biome::Cinderworks => "Cinderworks",
            Biome::Stormcrown => "Stormcrown",
            Biome::Beacon => "The Beacon",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Biome::Mossbluff => "Sunlit meadows and crumbling bridges",
            Biome::Glimmerdeep => "Dark caverns lit by crystal, spitters and wisps",
            Biome::Frostspire => "Slippery ice, crosswinds and spinning saws",
            Biome::Cinderworks => "Geysers of fire over a molten drop",
            Biome::Stormcrown => "Lightning, updrafts and everything at once",
            Biome::Beacon => "The Stormheart waits at the summit",
        }
    }

    /// Dark caverns drawn through a light map.
    pub fn dark(self) -> bool {
        matches!(self, Biome::Glimmerdeep)
    }

    /// Cave biomes get ceilings.
    pub fn cave(self) -> bool {
        matches!(self, Biome::Glimmerdeep | Biome::Cinderworks)
    }

    pub fn index(self) -> usize {
        match self {
            Biome::Mossbluff => 0,
            Biome::Glimmerdeep => 1,
            Biome::Frostspire => 2,
            Biome::Cinderworks => 3,
            Biome::Stormcrown => 4,
            Biome::Beacon => 5,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnchorKind {
    /// Holds forever.
    Ring,
    /// Cracks and drops after about a second of tension.
    Brittle,
    /// Slides back and forth along a rail.
    Rail,
}

#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
    pub kind: AnchorKind,
    /// Other end of the rail (Rail only).
    pub x2: f32,
    pub y2: f32,
    pub phase: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntKind {
    Crystal,
    EchoOrb,
    HeartShard,
    Chest,
    Skitter,
    Bulwark,
    Spitter,
    Saw,
    Geyser,
    Wisp,
    Bolt,
    Strike,
    Bloom,
    Wind,
}

impl EntKind {
    pub fn name(self) -> &'static str {
        match self {
            EntKind::Crystal => "crystal",
            EntKind::EchoOrb => "echo orb",
            EntKind::HeartShard => "heart shard",
            EntKind::Chest => "relic chest",
            EntKind::Skitter => "Skitter",
            EntKind::Bulwark => "Bulwark",
            EntKind::Spitter => "Spitter",
            EntKind::Saw => "Sawblade",
            EntKind::Geyser => "Geyser",
            EntKind::Wisp => "Wisp",
            EntKind::Bolt => "bolt",
            EntKind::Strike => "lightning",
            EntKind::Bloom => "springbloom",
            EntKind::Wind => "wind",
        }
    }

    /// Kinds that count as foes for statistics and difficulty tests.
    pub fn is_threat(self) -> bool {
        matches!(
            self,
            EntKind::Skitter
                | EntKind::Bulwark
                | EntKind::Spitter
                | EntKind::Saw
                | EntKind::Geyser
                | EntKind::Wisp
                | EntKind::Strike
        )
    }
}

/// Generic spawn record; `a`, `b`, `c` meanings depend on the kind:
/// Saw: (x2, y2, speed). Wind: (accel x, accel y, half width) with `w`/`h`
/// the zone half extents stored in `hw`/`hh`. Geyser: (period, phase, _).
#[derive(Clone, Copy, Debug)]
pub struct EntSpawn {
    pub kind: EntKind,
    pub x: f32,
    pub y: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub hw: f32,
    pub hh: f32,
}

impl EntSpawn {
    pub fn new(kind: EntKind, x: f32, y: f32) -> EntSpawn {
        EntSpawn {
            kind,
            x,
            y,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            hw: 0.0,
            hh: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BeatKind {
    Walk,
    Jump,
    Shaft,
    Swing,
    Boss,
}

/// One traversal challenge, in order. Used by the autopilot and progress bar.
#[derive(Clone, Debug)]
pub struct Beat {
    pub kind: BeatKind,
    /// Stand position (feet) at the start and end, in pixels.
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    /// Indices into `Level::anchors` used by a Swing.
    pub anchors: Vec<usize>,
    /// For jumps and swings: where the take-off platform ends.
    pub edge_x: f32,
}

#[derive(Clone, Debug)]
pub struct Prompt {
    pub x0: f32,
    pub x1: f32,
    pub text: &'static str,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    pub biome: Biome,
    pub seed: u64,
    /// Player centre spawn.
    pub spawn: (f32, f32),
    /// Exit gate centre.
    pub exit: (f32, f32),
    /// Checkpoint lamp positions (feet).
    pub checkpoints: Vec<(f32, f32)>,
    pub ents: Vec<EntSpawn>,
    pub anchors: Vec<Anchor>,
    pub beats: Vec<Beat>,
    pub prompts: Vec<Prompt>,
    /// Par time in seconds for a gold medal.
    pub par: f32,
    pub tutorial: bool,
    pub boss: bool,
    /// Index into `tiles` -> block id + 1 for glass and crumble tiles.
    pub block_ix: Vec<u32>,
    /// Glass/crumble tile coordinates in block id order.
    pub block_tiles: Vec<(i32, i32)>,
}

impl Level {
    pub fn new(w: i32, h: i32, biome: Biome, seed: u64) -> Level {
        Level {
            w,
            h,
            tiles: vec![AIR; (w * h) as usize],
            biome,
            seed,
            spawn: (0.0, 0.0),
            exit: (0.0, 0.0),
            checkpoints: vec![],
            ents: vec![],
            anchors: vec![],
            beats: vec![],
            prompts: vec![],
            par: 60.0,
            tutorial: false,
            boss: false,
            block_ix: vec![],
            block_tiles: vec![],
        }
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    /// Tile lookup; outside the grid is rock at the sides and top, air below.
    #[inline]
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.w || y < 0 {
            ROCK
        } else if y >= self.h {
            AIR
        } else {
            self.tiles[(y * self.w + x) as usize]
        }
    }

    pub fn set(&mut self, x: i32, y: i32, t: Tile) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.tiles[i] = t;
        }
    }

    pub fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, t: Tile) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, t);
            }
        }
    }

    /// Call once after generation: indexes glass and crumble blocks.
    pub fn finalize(&mut self) {
        self.block_ix = vec![0; self.tiles.len()];
        self.block_tiles.clear();
        for y in 0..self.h {
            for x in 0..self.w {
                let t = self.tiles[self.idx(x, y)];
                if t == GLASS || t == CRUMBLE {
                    self.block_tiles.push((x, y));
                    let i = self.idx(x, y);
                    self.block_ix[i] = self.block_tiles.len() as u32;
                }
            }
        }
    }

    #[inline]
    pub fn block_at(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        let b = self.block_ix[(y * self.w + x) as usize];
        if b == 0 { None } else { Some(b as usize - 1) }
    }

    pub fn pixel_w(&self) -> f32 {
        self.w as f32 * T
    }

    pub fn pixel_h(&self) -> f32 {
        self.h as f32 * T
    }

    /// Lowest world y before a fall counts as a pit.
    pub fn pit_y(&self) -> f32 {
        self.pixel_h() + 120.0
    }
}
