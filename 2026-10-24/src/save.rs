//! Persistent save file: bests, the Workshop bank and perks, options.
//! Plain `key=value` lines; unknown keys are ignored, corrupt values reset.

use std::path::{Path, PathBuf};

pub const PERK_COUNT: usize = 5;
pub const STAGES: usize = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Save {
    pub best_score: i64,
    /// Furthest stage reached (1-6, 7 = victory).
    pub best_stage: u32,
    pub runs: u32,
    pub wins: u32,
    pub tutorial_done: bool,
    /// Banked glimmer for the Workshop.
    pub glimmer: u32,
    pub perks: [u8; PERK_COUNT],
    /// Best clear time per stage in milliseconds (0 = none).
    pub best_ms: [u32; STAGES],
    pub heat_unlocked: u8,
    pub heat: u8,
    pub aim_assist: bool,
    /// 0 = full speed, 1 = 85%, 2 = 70%.
    pub speed: u8,
    /// Invincible practice mode; runs don't count for records.
    pub explorer: bool,
    pub muted: bool,
    /// Bitmask of one-time hints already shown.
    pub hints: u32,
}

impl Default for Save {
    fn default() -> Save {
        Save {
            best_score: 0,
            best_stage: 0,
            runs: 0,
            wins: 0,
            tutorial_done: false,
            glimmer: 0,
            perks: [0; PERK_COUNT],
            best_ms: [0; STAGES],
            heat_unlocked: 0,
            heat: 0,
            aim_assist: true,
            speed: 0,
            explorer: false,
            muted: false,
            hints: 0,
        }
    }
}

impl Save {
    /// `$CRAGSWING_SAVE`, else `~/.cragswing/save.txt`.
    pub fn default_path() -> PathBuf {
        if let Ok(p) = std::env::var("CRAGSWING_SAVE") {
            return PathBuf::from(p);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cragswing").join("save.txt")
    }

    pub fn load(path: &Path) -> Save {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Save::default();
        };
        Save::parse(&text)
    }

    pub fn parse(text: &str) -> Save {
        let mut s = Save::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            let b = v == "1" || v == "true";
            match k.trim() {
                "best_score" => s.best_score = v.parse().unwrap_or(0),
                "best_stage" => s.best_stage = v.parse().unwrap_or(0),
                "runs" => s.runs = v.parse().unwrap_or(0),
                "wins" => s.wins = v.parse().unwrap_or(0),
                "tutorial_done" => s.tutorial_done = b,
                "glimmer" => s.glimmer = v.parse().unwrap_or(0),
                "heat_unlocked" => s.heat_unlocked = v.parse().unwrap_or(0),
                "heat" => s.heat = v.parse().unwrap_or(0),
                "aim_assist" => s.aim_assist = b,
                "speed" => s.speed = v.parse().unwrap_or(0).min(2),
                "explorer" => s.explorer = b,
                "muted" => s.muted = b,
                "hints" => s.hints = v.parse().unwrap_or(0),
                "perks" => {
                    for (i, p) in v.split(',').enumerate().take(PERK_COUNT) {
                        s.perks[i] = p.trim().parse().unwrap_or(0);
                    }
                }
                "best_ms" => {
                    for (i, p) in v.split(',').enumerate().take(STAGES) {
                        s.best_ms[i] = p.trim().parse().unwrap_or(0);
                    }
                }
                _ => {}
            }
        }
        s.heat = s.heat.min(s.heat_unlocked);
        s
    }

    pub fn serialize(&self) -> String {
        let join = |v: &[String]| v.join(",");
        let perks: Vec<String> = self.perks.iter().map(|p| p.to_string()).collect();
        let best: Vec<String> = self.best_ms.iter().map(|p| p.to_string()).collect();
        format!(
            "best_score={}\nbest_stage={}\nruns={}\nwins={}\ntutorial_done={}\nglimmer={}\nheat_unlocked={}\nheat={}\naim_assist={}\nspeed={}\nexplorer={}\nmuted={}\nhints={}\nperks={}\nbest_ms={}\n",
            self.best_score,
            self.best_stage,
            self.runs,
            self.wins,
            b(self.tutorial_done),
            self.glimmer,
            self.heat_unlocked,
            self.heat,
            b(self.aim_assist),
            self.speed,
            b(self.explorer),
            b(self.muted),
            self.hints,
            join(&perks),
            join(&best),
        )
    }

    pub fn store(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.serialize())
    }

    /// Fold a finished run in. Returns true if it set a new best score.
    pub fn record_run(&mut self, score: i64, stage_reached: u32, won: bool, glimmer: u32) -> bool {
        self.runs += 1;
        self.glimmer += glimmer;
        if won {
            self.wins += 1;
            self.heat_unlocked = (self.heat_unlocked + 1).min(3).max(self.heat_unlocked);
        }
        self.best_stage = self.best_stage.max(stage_reached);
        if score > self.best_score {
            self.best_score = score;
            true
        } else {
            false
        }
    }

    /// Record a stage clear time; true if it is a new best.
    pub fn record_time(&mut self, stage: usize, seconds: f32) -> bool {
        let ms = (seconds * 1000.0) as u32;
        if stage >= STAGES || ms == 0 {
            return false;
        }
        if self.best_ms[stage] == 0 || ms < self.best_ms[stage] {
            self.best_ms[stage] = ms;
            return true;
        }
        false
    }
}

fn b(v: bool) -> u8 {
    if v { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_corruption() {
        let s = Save {
            best_score: 1234,
            best_stage: 5,
            runs: 3,
            wins: 1,
            tutorial_done: true,
            glimmer: 410,
            perks: [1, 0, 2, 0, 1],
            best_ms: [61000, 0, 0, 0, 0, 0],
            heat_unlocked: 1,
            heat: 1,
            aim_assist: false,
            speed: 2,
            explorer: false,
            muted: true,
            hints: 0b1011,
        };
        assert_eq!(Save::parse(&s.serialize()), s);
        let bad = Save::parse("best_score=abc\nwins=2\ngarbage\nperks=1,x,3\n");
        assert_eq!(bad.best_score, 0);
        assert_eq!(bad.wins, 2);
        assert_eq!(bad.perks, [1, 0, 3, 0, 0]);
    }

    #[test]
    fn records_runs_and_times() {
        let mut s = Save::default();
        assert!(s.record_run(100, 2, false, 50));
        assert!(!s.record_run(50, 4, false, 20));
        assert!(s.record_run(900, 7, true, 100));
        assert_eq!((s.runs, s.wins, s.best_stage, s.glimmer), (3, 1, 7, 170));
        assert_eq!(s.heat_unlocked, 1);
        assert!(s.record_time(0, 50.0));
        assert!(!s.record_time(0, 60.0));
        assert!(s.record_time(0, 40.0));
        assert_eq!(s.best_ms[0], 40000);
    }

    #[test]
    fn store_and_load_file() {
        let dir = std::env::temp_dir().join(format!("cragswing-test-{}", std::process::id()));
        let path = dir.join("save.txt");
        let s = Save {
            best_score: 77,
            ..Save::default()
        };
        s.store(&path).unwrap();
        assert_eq!(Save::load(&path), s);
        let _ = std::fs::remove_dir_all(dir);
    }
}
