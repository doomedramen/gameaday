//! Procedural circuits: a closed spline loop with validity checks
//! (separation, minimum radius, length, a start straight), a minimum-
//! curvature racing line, a speed profile, surfaces, hazards, pickups and
//! scenery. The world bitmap (ground, road, kerbs, barriers, ground props) is
//! rendered on demand by `render_bitmap` so headless runs never pay for it.

use crate::canvas::{Blit, Canvas, mix, scale_rgb};
use crate::math::{V2, v2};
use crate::rng::Rng;
use std::f32::consts::TAU;

/// Centreline sample spacing.
pub const STEP: f32 = 10.0;
/// Half the road width.
pub const HALF_W: f32 = 112.0;
/// Kerb band width (inside the road edge).
pub const KERB: f32 = 14.0;
/// Distance from the centreline to the barrier.
pub const WALL: f32 = HALF_W + 78.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Greenvale,
    DuneSea,
    Timberline,
    NeonNight,
    GrandPrix,
}

pub const BIOMES: [Biome; 5] = [
    Biome::Greenvale,
    Biome::DuneSea,
    Biome::Timberline,
    Biome::NeonNight,
    Biome::GrandPrix,
];

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Greenvale => "Greenvale",
            Biome::DuneSea => "Dune Sea",
            Biome::Timberline => "Timberline",
            Biome::NeonNight => "Neon Night",
            Biome::GrandPrix => "Grand Prix",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            Biome::Greenvale => "Rolling meadow circuit. Grippy asphalt, oil on the late laps.",
            Biome::DuneSea => "Desert asphalt swept by sand drifts. Sand saps grip.",
            Biome::Timberline => "Loose dirt road through the pines. Mud pools slow you.",
            Biome::NeonNight => "A city circuit after dark. Headlights and lamp posts only.",
            Biome::GrandPrix => "The championship final, in front of the full grandstands.",
        }
    }
    pub fn index(self) -> usize {
        BIOMES.iter().position(|b| *b == self).unwrap_or(0)
    }
    pub fn ground(self) -> &'static str {
        match self {
            Biome::DuneSea => "land_sand12",
            _ => "land_grass11",
        }
    }
    pub fn ground_tint(self) -> u32 {
        match self {
            Biome::Timberline => 0xC8DCA8,
            Biome::NeonNight => 0xB8C8C0,
            _ => 0xFFFFFF,
        }
    }
    pub fn road(self) -> u32 {
        match self {
            Biome::Timberline => 0xBF8A50,
            Biome::NeonNight => 0x8FA9B2,
            _ => 0xA6C8CC,
        }
    }
    pub fn kerb(self) -> (u32, u32) {
        match self {
            Biome::DuneSea => (0x3DBB6A, 0xF6F6F0),
            Biome::Timberline => (0xF6F6F0, 0x9B6533),
            Biome::GrandPrix => (0xE0393E, 0xF6F6F0),
            _ => (0xEC6A22, 0xF6F6F0),
        }
    }
    /// Grip multiplier of the road itself.
    pub fn road_grip(self) -> f32 {
        match self {
            Biome::Timberline => 0.82,
            Biome::DuneSea => 0.95,
            _ => 1.0,
        }
    }
    pub fn night(self) -> bool {
        self == Biome::NeonNight
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Road,
    Kerb,
    Runoff,
    Oil,
    Mud,
    Sand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
    Oil,
    Mud,
    Sand,
}

#[derive(Clone, Copy, Debug)]
pub struct Patch {
    pub kind: PatchKind,
    pub pos: V2,
    pub r: f32,
    pub rot: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Pad {
    pub pos: V2,
    pub angle: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Nitro,
    Repair,
}

#[derive(Clone, Copy, Debug)]
pub struct PickupSpot {
    pub pos: V2,
    pub kind: PickupKind,
}

/// Scenery. `height` > 0 draws above cars with parallax; 0 is stamped into
/// the ground bitmap.
#[derive(Clone, Debug)]
pub struct Prop {
    pub sprite: &'static str,
    pub pos: V2,
    pub rot: f32,
    pub scale: f32,
    pub height: f32,
    /// Collision radius (0 = no collision).
    pub solid: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub start_s: f32,
    pub apex_s: f32,
    pub end_s: f32,
    /// +1 right, -1 left (screen space).
    pub dir: f32,
    /// 1 = fast, 2 = medium, 3 = hairpin.
    pub severity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSpec {
    pub seed: u64,
    pub biome: Biome,
    /// 0 = flowing, 1 = full of hairpins.
    pub twisty: f32,
    /// 0..1 hazard density (oil, mud, sand, cones).
    pub hazard: f32,
    /// Target lap length in pixels.
    pub length: f32,
}

impl TrackSpec {
    pub fn new(seed: u64, biome: Biome) -> TrackSpec {
        TrackSpec {
            seed,
            biome,
            twisty: 0.4,
            hazard: 0.5,
            length: 14000.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Loc {
    pub i: usize,
    /// Distance along the lap.
    pub s: f32,
    /// Signed lateral offset, + is left of travel.
    pub lat: f32,
}

pub struct Bitmap {
    pub origin: V2,
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

pub struct Track {
    pub spec: TrackSpec,
    pub n: usize,
    pub pts: Vec<V2>,
    pub tan: Vec<V2>,
    pub nor: Vec<V2>,
    pub curv: Vec<f32>,
    pub cum: Vec<f32>,
    pub len: f32,
    /// Racing line lateral offset per sample.
    pub line: Vec<f32>,
    pub line_curv: Vec<f32>,
    pub min: V2,
    pub max: V2,
    grid: Vec<Vec<u32>>,
    gw: usize,
    gh: usize,
    pub props: Vec<Prop>,
    pub patches: Vec<Patch>,
    pub pads: Vec<Pad>,
    pub pickups: Vec<PickupSpot>,
    pub cones: Vec<V2>,
    pub lamps: Vec<V2>,
    pub corners: Vec<Corner>,
    pub attempts: u32,
}

const CELL: f32 = 256.0;

fn catmull(p0: V2, p1: V2, p2: V2, p3: V2, t: f32) -> V2 {
    let t2 = t * t;
    let t3 = t2 * t;
    (p1 * 2.0
        + (p2 - p0) * t
        + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
        + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3)
        * 0.5
}

fn resample(raw: &[V2], step: f32) -> Vec<V2> {
    let n = raw.len();
    let mut out = vec![raw[0]];
    let mut carry = 0.0;
    for i in 0..n {
        let a = raw[i];
        let b = raw[(i + 1) % n];
        let seg = a.dist(b);
        let mut t = step - carry;
        while t <= seg {
            out.push(a.lerp(b, t / seg));
            t += step;
        }
        carry = seg - (t - step);
    }
    // drop the duplicate closing point if it lands near the start
    if out.len() > 2 && out[out.len() - 1].dist(out[0]) < step * 0.5 {
        out.pop();
    }
    out
}

fn loop_from_controls(ctrl: &[V2]) -> Vec<V2> {
    let k = ctrl.len();
    let mut raw = Vec::with_capacity(k * 40);
    for i in 0..k {
        let p0 = ctrl[(i + k - 1) % k];
        let p1 = ctrl[i];
        let p2 = ctrl[(i + 1) % k];
        let p3 = ctrl[(i + 2) % k];
        for j in 0..40 {
            raw.push(catmull(p0, p1, p2, p3, j as f32 / 40.0));
        }
    }
    let mut pts = resample(&raw, STEP);
    // Laplacian smoothing removes spline kinks.
    for _ in 0..12 {
        let n = pts.len();
        let prev = pts.clone();
        for i in 0..n {
            pts[i] = (prev[(i + n - 1) % n] + prev[i] * 2.0 + prev[(i + 1) % n]) * 0.25;
        }
    }
    resample(&pts, STEP)
}

fn loop_len(pts: &[V2]) -> f32 {
    let n = pts.len();
    (0..n).map(|i| pts[i].dist(pts[(i + 1) % n])).sum()
}

/// Signed curvature per sample (right turn positive in screen space),
/// smoothed over a few samples.
fn curvature(pts: &[V2], tan: &[V2]) -> Vec<f32> {
    let n = pts.len();
    let raw: Vec<f32> = (0..n)
        .map(|i| {
            let a = tan[(i + n - 1) % n];
            let b = tan[(i + 1) % n];
            a.cross(b) / (2.0 * STEP)
        })
        .collect();
    let mut out = vec![0.0; n];
    for i in 0..n {
        let mut acc = 0.0;
        for k in -2i32..=2 {
            acc += raw[(i as i32 + k).rem_euclid(n as i32) as usize];
        }
        out[i] = acc / 5.0;
    }
    out
}

fn tangents(pts: &[V2]) -> Vec<V2> {
    let n = pts.len();
    (0..n)
        .map(|i| (pts[(i + 1) % n] - pts[(i + n - 1) % n]).norm())
        .collect()
}

/// Steering-limited and grip-limited corner speed for a given radius. Mirrors
/// the car model in `car.rs` so the AI's braking points are honest.
pub fn corner_speed(radius: f32, grip_acc: f32, yaw_max_at: impl Fn(f32) -> f32, top: f32) -> f32 {
    let grip_v = (grip_acc * radius).sqrt();
    let mut v = top;
    while v > 60.0 {
        if v / yaw_max_at(v) <= radius * 0.92 {
            break;
        }
        v -= 8.0;
    }
    v.min(grip_v).max(90.0)
}

impl Track {
    /// Generate a valid circuit from the spec, retrying derived seeds until
    /// every check passes.
    pub fn generate(spec: TrackSpec) -> Track {
        let mut rng = Rng::new(spec.seed ^ 0x51A5_7EA3);
        for attempt in 0..400u32 {
            if let Some(mut t) = Self::try_generate(spec, &mut rng) {
                t.attempts = attempt + 1;
                return t;
            }
        }
        // Fallback: a plain oval always passes.
        let mut rng = Rng::new(1);
        let mut s = spec;
        s.twisty = 0.0;
        Self::try_generate_shape(s, &mut rng, true).expect("oval")
    }

    fn try_generate(spec: TrackSpec, rng: &mut Rng) -> Option<Track> {
        Self::try_generate_shape(spec, rng, false)
    }

    fn try_generate_shape(spec: TrackSpec, rng: &mut Rng, oval: bool) -> Option<Track> {
        let k = if oval {
            8
        } else {
            12 + (spec.twisty * 4.0) as usize + rng.below(3)
        };
        let base_r = 1300.0;
        let mut ctrl = Vec::with_capacity(k);
        let phase = rng.range(0.0, TAU);
        for i in 0..k {
            let a = phase
                + i as f32 * TAU / k as f32
                + if oval {
                    0.0
                } else {
                    rng.range(-0.32, 0.32) * TAU / k as f32
                };
            let mut r = if oval {
                base_r
            } else {
                base_r * rng.range(0.55, 1.0)
            };
            if !oval && rng.chance(0.22 + spec.twisty * 0.3) {
                r *= rng.range(0.3, 0.5);
            }
            ctrl.push(v2(a.cos() * r * 1.4, a.sin() * r));
        }
        // Fit the length.
        let mut pts = loop_from_controls(&ctrl);
        for _ in 0..3 {
            let l = loop_len(&pts);
            let k = spec.length / l;
            if (k - 1.0).abs() < 0.02 {
                break;
            }
            for c in ctrl.iter_mut() {
                *c = *c * k;
            }
            pts = loop_from_controls(&ctrl);
        }
        // Minimum radius: relax over-tight bends with extra smoothing, then
        // re-fit the length (smoothing shrinks the loop).
        let mut ok = false;
        for _ in 0..10 {
            let tan = tangents(&pts);
            let curv = curvature(&pts, &tan);
            let max_k = curv.iter().fold(0.0f32, |m, c| m.max(c.abs()));
            if max_k <= 1.0 / 92.0 {
                ok = true;
                break;
            }
            let n = pts.len();
            for _ in 0..20 {
                let prev = pts.clone();
                for i in 0..n {
                    let k = curv[i].abs() * 92.0;
                    let w = if k > 0.7 { 0.5 } else { 0.15 };
                    let avg = (prev[(i + n - 1) % n] + prev[(i + 1) % n]) * 0.5;
                    pts[i] = prev[i].lerp(avg, w);
                }
            }
            let c = pts.iter().fold(V2::default(), |a, p| a + *p) * (1.0 / n as f32);
            let k = spec.length / loop_len(&pts);
            for p in pts.iter_mut() {
                *p = c + (*p - c) * k;
            }
            pts = resample(&pts, STEP);
        }
        if !ok {
            return None;
        }
        let n = pts.len();
        if n < 300 {
            return None;
        }
        let tan = tangents(&pts);
        let curv = curvature(&pts, &tan);
        // Separation between parts of the loop that are far apart along it.
        let sep = 2.0 * WALL + 70.0;
        let skip = ((sep * 1.6) / STEP) as usize;
        for i in 0..n {
            for j in (i + skip)..n {
                if n - j + i < skip {
                    continue;
                }
                if pts[i].dist(pts[j]) < sep {
                    return None;
                }
            }
        }
        // Pick the straightest 700px stretch for the start.
        let win = (700.0 / STEP) as usize;
        let mut best = (f32::MAX, 0usize);
        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..win {
                acc += curv[(i + j) % n].abs();
            }
            if acc < best.0 {
                best = (acc, i);
            }
        }
        if best.0 * STEP > 0.35 {
            return None;
        }
        let start = (best.1 + (300.0 / STEP) as usize) % n;
        pts.rotate_left(start);
        let mut t = Track::from_points(spec, pts);
        t.place_content(rng);
        Some(t)
    }

    fn from_points(spec: TrackSpec, pts: Vec<V2>) -> Track {
        let n = pts.len();
        let tan = tangents(&pts);
        let nor: Vec<V2> = tan.iter().map(|t| t.perp()).collect();
        let curv = curvature(&pts, &tan);
        let mut cum = vec![0.0; n];
        for i in 1..n {
            cum[i] = cum[i - 1] + pts[i - 1].dist(pts[i]);
        }
        let len = cum[n - 1] + pts[n - 1].dist(pts[0]);
        let mut min = v2(f32::MAX, f32::MAX);
        let mut max = v2(f32::MIN, f32::MIN);
        for p in &pts {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        }
        let margin = WALL + 420.0;
        min -= v2(margin, margin);
        max += v2(margin, margin);
        let gw = ((max.x - min.x) / CELL).ceil() as usize + 1;
        let gh = ((max.y - min.y) / CELL).ceil() as usize + 1;
        let mut grid = vec![Vec::new(); gw * gh];
        let reach = WALL + 80.0;
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let x0 = ((a.x.min(b.x) - reach - min.x) / CELL).floor().max(0.0) as usize;
            let x1 = ((a.x.max(b.x) + reach - min.x) / CELL).floor() as usize;
            let y0 = ((a.y.min(b.y) - reach - min.y) / CELL).floor().max(0.0) as usize;
            let y1 = ((a.y.max(b.y) + reach - min.y) / CELL).floor() as usize;
            for gy in y0..=y1.min(gh - 1) {
                for gx in x0..=x1.min(gw - 1) {
                    grid[gy * gw + gx].push(i as u32);
                }
            }
        }
        let mut t = Track {
            spec,
            n,
            pts,
            tan,
            nor,
            curv,
            cum,
            len,
            line: vec![0.0; n],
            line_curv: vec![0.0; n],
            min,
            max,
            grid,
            gw,
            gh,
            props: Vec::new(),
            patches: Vec::new(),
            pads: Vec::new(),
            pickups: Vec::new(),
            cones: Vec::new(),
            lamps: Vec::new(),
            corners: Vec::new(),
            attempts: 0,
        };
        t.compute_line();
        t.compute_corners();
        t
    }

    /// Minimum-curvature racing line by relaxing an elastic band within the
    /// road (outside - apex - outside falls out of it).
    fn compute_line(&mut self) {
        let n = self.n;
        let lim = HALF_W - 30.0;
        let mut off = vec![0.0f32; n];
        for pass in 0..3 {
            let span = [12usize, 6, 3][pass];
            for _ in 0..120 {
                let prev = off.clone();
                for i in 0..n {
                    let a = (i + n - span) % n;
                    let b = (i + span) % n;
                    let pa = self.pts[a] + self.nor[a] * prev[a];
                    let pb = self.pts[b] + self.nor[b] * prev[b];
                    let mid = (pa + pb) * 0.5;
                    let want = (mid - self.pts[i]).dot(self.nor[i]);
                    off[i] = (prev[i] + (want - prev[i]) * 0.5).clamp(-lim, lim);
                }
            }
        }
        self.line = off;
        let lp: Vec<V2> = (0..n).map(|i| self.line_point(i)).collect();
        let lt = tangents(&lp);
        self.line_curv = curvature(&lp, &lt);
    }

    fn compute_corners(&mut self) {
        let n = self.n;
        let th = 1.0 / 520.0;
        let mut i = 0;
        // start scanning from a straight sample so corners don't wrap oddly
        let mut first = 0;
        while first < n && self.curv[first].abs() > th {
            first += 1;
        }
        let mut corners = Vec::new();
        while i < n {
            let idx = (first + i) % n;
            if self.curv[idx].abs() > th {
                let dir = self.curv[idx].signum();
                let mut j = i;
                let mut peak = (0.0f32, idx);
                while j < n {
                    let k = (first + j) % n;
                    if self.curv[k].abs() <= th * 0.7 || self.curv[k].signum() != dir {
                        break;
                    }
                    if self.curv[k].abs() > peak.0 {
                        peak = (self.curv[k].abs(), k);
                    }
                    j += 1;
                }
                let radius = 1.0 / peak.0.max(1e-5);
                let severity = if radius < 150.0 {
                    3
                } else if radius < 280.0 {
                    2
                } else {
                    1
                };
                if (j - i) as f32 * STEP > 60.0 {
                    corners.push(Corner {
                        start_s: self.cum[idx],
                        apex_s: self.cum[peak.1],
                        end_s: self.cum[(first + j.min(n - 1)) % n],
                        dir,
                        severity,
                    });
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
        corners.sort_by(|a, b| a.start_s.partial_cmp(&b.start_s).unwrap());
        self.corners = corners;
    }

    pub fn line_point(&self, i: usize) -> V2 {
        let i = i % self.n;
        self.pts[i] + self.nor[i] * self.line[i]
    }

    pub fn idx_at(&self, s: f32) -> usize {
        let s = s.rem_euclid(self.len);
        ((s / STEP) as usize).min(self.n - 1)
    }

    /// Centre point at distance s plus a lateral offset.
    pub fn point_at(&self, s: f32, lat: f32) -> V2 {
        let s = s.rem_euclid(self.len);
        let i = self.idx_at(s);
        let j = (i + 1) % self.n;
        let f = ((s - self.cum[i]) / STEP).clamp(0.0, 1.0);
        let c = self.pts[i].lerp(self.pts[j], f);
        let nrm = self.nor[i].lerp(self.nor[j], f).norm();
        c + nrm * lat
    }

    pub fn angle_at(&self, s: f32) -> f32 {
        self.tan[self.idx_at(s)].angle()
    }

    fn project(&self, p: V2, i: usize) -> (f32, Loc) {
        let a = self.pts[i];
        let b = self.pts[(i + 1) % self.n];
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.len2()).clamp(0.0, 1.0);
        let c = a + ab * t;
        let d = p.dist(c);
        let lat = (p - c).dot(self.nor[i]);
        let s = (self.cum[i] + t * STEP).rem_euclid(self.len);
        (d, Loc { i, s, lat })
    }

    /// Nearest centreline location; searches near `hint` first.
    pub fn locate(&self, p: V2, hint: Option<usize>) -> Loc {
        if let Some(h) = hint {
            let mut best = (f32::MAX, Loc::default());
            for k in -25i32..=25 {
                let i = (h as i32 + k).rem_euclid(self.n as i32) as usize;
                let (d, l) = self.project(p, i);
                if d < best.0 {
                    best = (d, l);
                }
            }
            if best.0 < WALL + 60.0 {
                return best.1;
            }
        }
        let gx = ((p.x - self.min.x) / CELL).floor();
        let gy = ((p.y - self.min.y) / CELL).floor();
        let mut best = (f32::MAX, Loc::default());
        if gx >= 0.0 && gy >= 0.0 && (gx as usize) < self.gw && (gy as usize) < self.gh {
            for &i in &self.grid[gy as usize * self.gw + gx as usize] {
                let (d, l) = self.project(p, i as usize);
                if d < best.0 {
                    best = (d, l);
                }
            }
        }
        if best.0 == f32::MAX {
            for i in 0..self.n {
                let (d, l) = self.project(p, i);
                if d < best.0 {
                    best = (d, l);
                }
            }
        }
        best.1
    }

    pub fn is_corner_zone(&self, i: usize) -> bool {
        let n = self.n;
        (-6i32..=6)
            .any(|k| self.curv[(i as i32 + k).rem_euclid(n as i32) as usize].abs() > 1.0 / 700.0)
    }

    pub fn surface(&self, p: V2, loc: &Loc) -> Surface {
        for patch in &self.patches {
            if p.dist(patch.pos) < patch.r {
                return match patch.kind {
                    PatchKind::Oil => Surface::Oil,
                    PatchKind::Mud => Surface::Mud,
                    PatchKind::Sand => Surface::Sand,
                };
            }
        }
        let d = loc.lat.abs();
        if d < HALF_W - KERB {
            Surface::Road
        } else if d < HALF_W + 4.0 {
            if self.is_corner_zone(loc.i) {
                Surface::Kerb
            } else {
                Surface::Road
            }
        } else {
            Surface::Runoff
        }
    }

    // ---- content ----------------------------------------------------------

    fn place_content(&mut self, rng: &mut Rng) {
        let b = self.spec.biome;
        let hz = self.spec.hazard;
        let len = self.len;
        // Hazard patches: on corner exits and straights, never in the first
        // 600px after the start.
        let kind = match b {
            Biome::Greenvale | Biome::GrandPrix | Biome::NeonNight => PatchKind::Oil,
            Biome::DuneSea => PatchKind::Sand,
            Biome::Timberline => PatchKind::Mud,
        };
        let count = match kind {
            PatchKind::Oil => (hz * 5.0).round() as usize,
            _ => 4 + (hz * 6.0).round() as usize,
        };
        let mut tries = 0;
        while self.patches.len() < count && tries < 400 {
            tries += 1;
            let s = rng.range(700.0, len - 500.0);
            let r = match kind {
                PatchKind::Oil => rng.range(28.0, 36.0),
                PatchKind::Mud => rng.range(40.0, 62.0),
                PatchKind::Sand => rng.range(50.0, 80.0),
            };
            // keep at least one car-width lane free of the patch
            let lat = rng.range(-(HALF_W - r * 0.5), HALF_W - r * 0.5)
                * if kind == PatchKind::Oil { 0.6 } else { 1.0 };
            let pos = self.point_at(s, lat);
            if self
                .patches
                .iter()
                .any(|p| p.pos.dist(pos) < p.r + r + 260.0)
            {
                continue;
            }
            self.patches.push(Patch {
                kind,
                pos,
                r,
                rot: rng.range(0.0, TAU),
            });
        }
        // Boost pads on the longer straights.
        let mut s = 900.0;
        while s < len - 400.0 {
            let i = self.idx_at(s);
            let straight = (0..40).all(|k| self.curv[(i + k) % self.n].abs() < 1.0 / 1400.0);
            if straight && rng.chance(0.5) {
                let lat = rng.range(-HALF_W * 0.55, HALF_W * 0.55);
                let pos = self.point_at(s + 120.0, lat);
                if self.patches.iter().all(|p| p.pos.dist(pos) > p.r + 120.0) {
                    self.pads.push(Pad {
                        pos,
                        angle: self.angle_at(s + 120.0),
                    });
                    s += 1800.0;
                    continue;
                }
            }
            s += 300.0;
        }
        // Pickups: rows of nitro cans, occasional repair.
        let mut s = 500.0;
        let mut row = 0;
        while s < len - 300.0 {
            let kind = if row % 3 == 2 {
                PickupKind::Repair
            } else {
                PickupKind::Nitro
            };
            let lats: &[f32] = if kind == PickupKind::Repair {
                &[0.0]
            } else {
                &[-60.0, 0.0, 60.0]
            };
            let off = rng.range(-30.0, 30.0);
            for &l in lats {
                let pos = self.point_at(s, l + off);
                if self.patches.iter().all(|p| p.pos.dist(pos) > p.r + 40.0)
                    && self.pads.iter().all(|p| p.pos.dist(pos) > 80.0)
                {
                    self.pickups.push(PickupSpot { pos, kind });
                }
            }
            row += 1;
            s += rng.range(1300.0, 1900.0);
        }
        // Cones on the apex kerbs of slow corners.
        if matches!(
            b,
            Biome::Greenvale | Biome::GrandPrix | Biome::NeonNight | Biome::DuneSea
        ) {
            for c in self.corners.clone() {
                if c.severity >= 2 && rng.chance(0.4 + hz * 0.4) {
                    for k in 0..3 {
                        let lat = -c.dir * (HALF_W - 6.0);
                        self.cones
                            .push(self.point_at(c.apex_s + (k as f32 - 1.0) * 26.0, lat));
                    }
                }
            }
        }
        self.place_scenery(rng);
    }

    fn place_scenery(&mut self, rng: &mut Rng) {
        let b = self.spec.biome;
        let len = self.len;
        // Grandstands and tents by the start/finish.
        let stands = if b == Biome::GrandPrix { 7 } else { 2 };
        for k in 0..stands {
            let s = (len - 260.0
                + k as f32 * 260.0 * if b == Biome::GrandPrix { 1.0 } else { 1.6 })
            .rem_euclid(len);
            for side in [-1.0f32, 1.0] {
                if b != Biome::GrandPrix && side < 0.0 {
                    continue;
                }
                let i = self.idx_at(s);
                if self.is_corner_zone(i) {
                    continue;
                }
                let pos = self.point_at(s, side * (WALL + 70.0));
                if self.clear_of_track(pos, 120.0) {
                    let rot = self.angle_at(s)
                        + if side > 0.0 {
                            0.0
                        } else {
                            std::f32::consts::PI
                        };
                    self.props.push(Prop {
                        sprite: "tribune_full",
                        pos,
                        rot,
                        scale: 1.0,
                        height: 0.0,
                        solid: 0.0,
                    });
                    let over = if b == Biome::GrandPrix {
                        "tribune_overhang_red"
                    } else {
                        "tribune_overhang_striped"
                    };
                    self.props.push(Prop {
                        sprite: over,
                        pos: pos + V2::from_angle(rot - std::f32::consts::FRAC_PI_2) * 40.0,
                        rot,
                        scale: 1.0,
                        height: 0.6,
                        solid: 0.0,
                    });
                }
            }
        }
        // Tyre stacks on the outside of slow corners.
        for c in self.corners.clone() {
            if c.severity >= 2 {
                for k in -2..=2 {
                    let pos = self.point_at(c.apex_s + k as f32 * 34.0, c.dir * (WALL + 16.0));
                    if !self.clear_of_track(pos, WALL + 6.0) {
                        continue;
                    }
                    let spr = if k % 2 == 0 {
                        "tires_red"
                    } else {
                        "tires_white"
                    };
                    self.props.push(Prop {
                        sprite: spr,
                        pos,
                        rot: rng.range(0.0, TAU),
                        scale: 1.0,
                        height: 0.0,
                        solid: 0.0,
                    });
                }
            }
        }
        // Lamp posts at night, along the barrier.
        if b.night() {
            let mut s = 120.0;
            let mut side = 1.0;
            while s < len {
                let pos = self.point_at(s, side * (WALL + 18.0));
                if self.clear_of_track(pos, WALL + 10.0) {
                    self.lamps.push(pos);
                }
                s += 420.0;
                side = -side;
            }
        }
        // Scatter: trees, rocks, tents, barrels.
        let (n_scatter, kinds): (usize, &[(&'static str, f32, f32)]) = match b {
            Biome::Greenvale => (
                150,
                &[
                    ("tree_large", 1.0, 0.55),
                    ("tree_small", 1.0, 0.25),
                    ("tent_red", 0.0, 0.05),
                    ("tent_blue", 0.0, 0.05),
                    ("rock2", 0.0, 0.05),
                    ("tires_white_alt", 0.0, 0.05),
                ],
            ),
            Biome::DuneSea => (
                110,
                &[
                    ("rock1", 0.0, 0.3),
                    ("rock2", 0.0, 0.25),
                    ("rock3", 0.0, 0.25),
                    ("barrel_red", 0.0, 0.08),
                    ("tent_red_large", 0.0, 0.06),
                    ("tree_small", 1.0, 0.06),
                ],
            ),
            Biome::Timberline => (
                300,
                &[
                    ("tree_large", 1.2, 0.6),
                    ("tree_small", 1.0, 0.32),
                    ("rock3", 0.0, 0.08),
                ],
            ),
            Biome::NeonNight => (
                90,
                &[
                    ("tree_small", 1.0, 0.35),
                    ("barrel_blue", 0.0, 0.2),
                    ("tent_blue_large", 0.0, 0.1),
                    ("tires_red_alt", 0.0, 0.2),
                    ("tree_large", 1.0, 0.15),
                ],
            ),
            Biome::GrandPrix => (
                120,
                &[
                    ("tent_red_large", 0.0, 0.18),
                    ("tent_blue_large", 0.0, 0.18),
                    ("tree_large", 1.0, 0.35),
                    ("tree_small", 1.0, 0.2),
                    ("barrel_blue", 0.0, 0.09),
                ],
            ),
        };
        let total: f32 = kinds.iter().map(|k| k.2).sum();
        let mut placed = 0;
        let mut tries = 0;
        while placed < n_scatter && tries < n_scatter * 30 {
            tries += 1;
            let pos = v2(
                rng.range(self.min.x, self.max.x),
                rng.range(self.min.y, self.max.y),
            );
            let mut r = rng.range(0.0, total);
            let mut pick = kinds[0];
            for k in kinds {
                if r < k.2 {
                    pick = *k;
                    break;
                }
                r -= k.2;
            }
            let big = pick.0.starts_with("tree_large") || pick.0.contains("large");
            // Timberline trees hang over the barrier a little.
            let clear = if b == Biome::Timberline && pick.0.starts_with("tree") {
                WALL + 6.0
            } else if big {
                WALL + 70.0
            } else {
                WALL + 40.0
            };
            if !self.clear_of_track(pos, clear) {
                continue;
            }
            let rad = if big { 60.0 } else { 30.0 };
            if self.props.iter().any(|p| {
                p.pos.dist(pos)
                    < rad
                        + if p.sprite.contains("large") || p.sprite.starts_with("tribune") {
                            70.0
                        } else {
                            26.0
                        }
            }) {
                continue;
            }
            let scale = if pick.0.starts_with("tree") {
                rng.range(0.85, 1.2)
            } else {
                1.0
            };
            self.props.push(Prop {
                sprite: pick.0,
                pos,
                rot: rng.range(0.0, TAU),
                scale,
                height: pick.1,
                solid: 0.0,
            });
            placed += 1;
        }
    }

    fn clear_of_track(&self, p: V2, d: f32) -> bool {
        let l = self.locate(p, None);
        l.lat.abs() > d
    }

    // ---- world bitmap ------------------------------------------------------

    /// Render ground, road, kerbs, barriers and ground-level props.
    pub fn render_bitmap(&self) -> Bitmap {
        let assets = crate::assets::get();
        let b = self.spec.biome;
        let w = (self.max.x - self.min.x).ceil() as usize;
        let h = (self.max.y - self.min.y).ceil() as usize;
        let origin = self.min;
        // ground texture
        let tile = assets.s(b.ground());
        let tint = b.ground_tint();
        let mut px = vec![0u32; w * h];
        for y in 0..h {
            let ty = y % tile.h;
            for x in 0..w {
                let c = tile.px[ty * tile.w + x % tile.w] & 0xFFFFFF;
                px[y * w + x] = c;
            }
        }
        if tint != 0xFFFFFF {
            for p in px.iter_mut() {
                *p = mul(*p, tint);
            }
        }
        // distance field: nearest segment per pixel within the barrier reach
        let reach = WALL + 14.0;
        let mut dist = vec![i16::MAX; w * h];
        let mut seg = vec![0u16; w * h];
        for i in 0..self.n {
            let a = self.pts[i] - origin;
            let bb = self.pts[(i + 1) % self.n] - origin;
            let x0 = (a.x.min(bb.x) - reach).floor().max(0.0) as usize;
            let x1 = ((a.x.max(bb.x) + reach).ceil() as usize).min(w);
            let y0 = (a.y.min(bb.y) - reach).floor().max(0.0) as usize;
            let y1 = ((a.y.max(bb.y) + reach).ceil() as usize).min(h);
            let ab = bb - a;
            let l2 = ab.len2();
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = v2(x as f32 + 0.5, y as f32 + 0.5);
                    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
                    let d = p.dist(a + ab * t);
                    if d < reach {
                        let q = (d * 8.0) as i16;
                        let k = y * w + x;
                        if q < dist[k] {
                            dist[k] = q;
                            seg[k] = i as u16;
                        }
                    }
                }
            }
        }
        let road = b.road();
        let edge = scale_rgb(road, 0.86);
        let (k1, k2) = b.kerb();
        let corner: Vec<bool> = (0..self.n).map(|i| self.is_corner_zone(i)).collect();
        let mut nrng = Rng::new(self.spec.seed ^ 77);
        let noise: Vec<u8> = (0..4096).map(|_| nrng.below(9) as u8).collect();
        for y in 0..h {
            for x in 0..w {
                let k = y * w + x;
                if dist[k] == i16::MAX {
                    continue;
                }
                let d = dist[k] as f32 / 8.0;
                let i = seg[k] as usize;
                let g = px[k];
                // along-track coordinate for stripes
                let p = v2(x as f32 + 0.5, y as f32 + 0.5) + origin;
                let s = self.cum[i] + (p - self.pts[i]).dot(self.tan[i]);
                let c = if d < HALF_W - KERB {
                    let nz = noise[(x * 7 + y * 131) & 4095] as f32 / 8.0;
                    scale_rgb(road, 0.985 + nz * 0.03)
                } else if d < HALF_W {
                    if corner[i] {
                        if ((s / 24.0).floor() as i64) % 2 == 0 {
                            k1
                        } else {
                            k2
                        }
                    } else if d < HALF_W - KERB + 5.0 {
                        road
                    } else if d < HALF_W - 3.0 {
                        0xF4F4EE
                    } else {
                        road
                    }
                } else if d < HALF_W + 4.0 {
                    edge
                } else if d < WALL - 9.0 {
                    // runoff: ground, slightly shaded near the barrier
                    let sh = smooth01((d - (WALL - 22.0)) / 13.0);
                    mix(g, scale_rgb(g, 0.82), sh)
                } else if d < WALL + 3.0 {
                    // barrier: concrete with red/white blocks on top
                    let block = ((s / 40.0).floor() as i64) % 2 == 0;
                    if d < WALL - 5.0 {
                        0xDADCE0
                    } else if block {
                        0xD8463C
                    } else {
                        0xF2F2F2
                    }
                } else {
                    // barrier shadow
                    let t = (d - (WALL + 3.0)) / 11.0;
                    mix(scale_rgb(g, 0.72), g, t)
                };
                // anti-alias the road edge
                let c = if (d - HALF_W - 4.0).abs() < 1.0 {
                    let ground = if d < WALL - 9.0 { g } else { c };
                    mix(edge, ground, d - HALF_W - 3.0)
                } else {
                    c
                };
                px[k] = c;
            }
        }
        drop(dist);
        drop(seg);
        let mut cv = Canvas {
            w,
            h,
            px,
            fonts: crate::font::Fonts::new(),
        };
        // start/finish chequers and grid boxes
        self.stamp_start(&mut cv, origin);
        // hazard decals
        for p in &self.patches {
            let c = p.pos - origin;
            match p.kind {
                PatchKind::Oil => {
                    let spr = assets.s("oil");
                    cv.blit(
                        spr,
                        c.x,
                        c.y,
                        Blit {
                            rot: p.rot,
                            scale: p.r * 2.2 / spr.w as f32,
                            ..Blit::default()
                        },
                    );
                }
                PatchKind::Mud => blob(&mut cv, c, p.r, 0x6E4A2A, 0x5A3B20, p.rot),
                PatchKind::Sand => blob(&mut cv, c, p.r, 0xEBD9AA, 0xDCC690, p.rot),
            }
        }
        // ground props with soft shadows
        for pr in &self.props {
            if pr.height > 0.0 {
                continue;
            }
            let spr = assets.s(pr.sprite);
            let c = pr.pos - origin;
            cv.blit(
                spr,
                c.x + 4.0,
                c.y + 5.0,
                Blit {
                    rot: pr.rot,
                    scale: pr.scale,
                    alpha: 0.25,
                    solid: Some(0x10202A),
                    ..Blit::default()
                },
            );
            cv.blit(
                spr,
                c.x,
                c.y,
                Blit {
                    rot: pr.rot,
                    scale: pr.scale,
                    ..Blit::default()
                },
            );
        }
        Bitmap {
            origin,
            w,
            h,
            px: cv.px,
        }
    }

    fn stamp_start(&self, cv: &mut Canvas, origin: V2) {
        let t = self.tan[0];
        let nrm = self.nor[0];
        let c0 = self.pts[0] - origin;
        let sq = 11.0;
        let rows = 3;
        let cols = ((2.0 * (HALF_W - KERB)) / sq) as i32;
        for r in 0..rows {
            for cidx in 0..cols {
                let lat = -(HALF_W - KERB) + (cidx as f32 + 0.5) * sq;
                let along = (r as f32 - 1.0) * sq;
                let p = c0 + nrm * lat + t * along;
                let col = if (r + cidx) % 2 == 0 {
                    0x22252C
                } else {
                    0xF4F4F0
                };
                let ang = t.angle();
                let corners =
                    [(-0.5f32, -0.5f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)].map(|(a, b)| {
                        let q = p + t * (a * sq) + nrm * (b * sq);
                        (q.x, q.y)
                    });
                let _ = ang;
                cv.tri([corners[0], corners[1], corners[2]], col, 1.0);
                cv.tri([corners[0], corners[2], corners[3]], col, 1.0);
            }
        }
        // grid boxes: two columns, staggered
        for slot in 0..10 {
            let (s, lat) = grid_slot(slot);
            let p = self.point_at(s, lat) - origin;
            let ang = self.angle_at(s);
            let f = V2::from_angle(ang);
            let side = f.perp();
            let a = p + f * 26.0 + side * 22.0;
            let b2 = p + f * 26.0 - side * 22.0;
            let c = p - f * 6.0 + side * 22.0;
            let d = p - f * 6.0 - side * 22.0;
            cv.line(a.x, a.y, b2.x, b2.y, 3.0, 0xF4F4F0, 0.85);
            cv.line(a.x, a.y, c.x, c.y, 3.0, 0xF4F4F0, 0.85);
            cv.line(b2.x, b2.y, d.x, d.y, 3.0, 0xF4F4F0, 0.85);
        }
    }

    /// The starting position (s, lateral) for grid slot k (0 = pole).
    pub fn grid_pos(&self, slot: usize) -> (V2, f32) {
        let (s, lat) = grid_slot(slot);
        (self.point_at(s, lat), self.angle_at(s))
    }
}

/// Grid slot k: distance behind the line and lateral lane.
pub fn grid_slot(k: usize) -> (f32, f32) {
    let row = k / 2;
    let lane = if k % 2 == 0 { 45.0 } else { -45.0 };
    (-60.0 - row as f32 * 78.0 - (k % 2) as f32 * 36.0, lane)
}

fn smooth01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mul(a: u32, b: u32) -> u32 {
    let ch = |s: u32| ((((a >> s) & 255) * ((b >> s) & 255)) / 255) << s;
    ch(16) | ch(8) | ch(0)
}

/// Irregular soft-edged blob (mud, sand drifts).
fn blob(cv: &mut Canvas, c: V2, r: f32, fill: u32, rim: u32, rot: f32) {
    for k in 0..7 {
        let a = rot + k as f32 * TAU / 7.0;
        let off = V2::from_angle(a) * (r * 0.45);
        let rr = r * (0.55 + 0.12 * ((k * 37) % 5) as f32 / 5.0);
        cv.circle(c.x + off.x, c.y + off.y, rr, rim, 0.9);
    }
    for k in 0..7 {
        let a = rot + k as f32 * TAU / 7.0;
        let off = V2::from_angle(a) * (r * 0.4);
        let rr = r * (0.45 + 0.1 * ((k * 53) % 5) as f32 / 5.0);
        cv.circle(c.x + off.x, c.y + off.y, rr, fill, 0.95);
    }
}
