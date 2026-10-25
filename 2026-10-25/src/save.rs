//! Persistent save: settings, best Cup score, Cups won, Time Trial records
//! with their ghost laps and calibrated medal times. Plain text, one
//! `key=value` per line; ghosts are comma-separated integers.

use crate::race::GhostFrame;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub steer_assist: bool,
    pub auto_throttle: bool,
    /// 100, 85 or 70 percent.
    pub game_speed: u32,
    pub shake: bool,
    pub music: u32,
    pub sfx: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            steer_assist: false,
            auto_throttle: false,
            game_speed: 100,
            shake: true,
            music: 8,
            sfx: 10,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Record {
    pub best: Option<f32>,
    pub ghost: Vec<GhostFrame>,
    /// The autopilot's lap (gold medal reference).
    pub par: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Save {
    pub settings: Settings,
    pub best_score: u32,
    pub cups_won: u32,
    pub cups_played: u32,
    pub best_round: u32,
    pub tutorial_done: bool,
    pub records: [Record; 5],
}

/// Medal for a lap against the par: 3 gold, 2 silver, 1 bronze, 0 none.
pub fn medal(lap: f32, par: f32) -> u8 {
    if lap <= par * 1.04 {
        3
    } else if lap <= par * 1.12 {
        2
    } else if lap <= par * 1.22 {
        1
    } else {
        0
    }
}

impl Save {
    pub fn default_path() -> PathBuf {
        if let Ok(p) = std::env::var("SLIPSTREAM_SAVE") {
            return PathBuf::from(p);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".slipstream-cup").join("save.txt")
    }

    pub fn load(path: &Path) -> Save {
        std::fs::read_to_string(path)
            .map(|t| Save::parse(&t))
            .unwrap_or_default()
    }

    pub fn store(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.serialize())?;
        std::fs::rename(tmp, path)
    }

    pub fn serialize(&self) -> String {
        let s = &self.settings;
        let mut out = String::from("slipstream-cup-save=1\n");
        out += &format!(
            "steer_assist={}\nauto_throttle={}\ngame_speed={}\nshake={}\nmusic={}\nsfx={}\n",
            s.steer_assist, s.auto_throttle, s.game_speed, s.shake, s.music, s.sfx
        );
        out += &format!(
            "best_score={}\ncups_won={}\ncups_played={}\nbest_round={}\ntutorial_done={}\n",
            self.best_score, self.cups_won, self.cups_played, self.best_round, self.tutorial_done
        );
        for (i, r) in self.records.iter().enumerate() {
            if let Some(b) = r.best {
                out += &format!("tt{i}_best={b}\n");
            }
            if let Some(p) = r.par {
                out += &format!("tt{i}_par={p}\n");
            }
            if !r.ghost.is_empty() {
                let g: Vec<String> = r
                    .ghost
                    .iter()
                    .map(|f| {
                        format!(
                            "{},{},{}",
                            f.0.round() as i32,
                            f.1.round() as i32,
                            (f.2 * 1000.0).round() as i32
                        )
                    })
                    .collect();
                out += &format!("tt{i}_ghost={}\n", g.join(","));
            }
        }
        out
    }

    pub fn parse(text: &str) -> Save {
        let mut s = Save::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let b = v == "true";
            let n = v.parse::<u32>().ok();
            match k {
                "steer_assist" => s.settings.steer_assist = b,
                "auto_throttle" => s.settings.auto_throttle = b,
                "game_speed" => {
                    s.settings.game_speed = n.filter(|x| [70, 85, 100].contains(x)).unwrap_or(100)
                }
                "shake" => s.settings.shake = b,
                "music" => s.settings.music = n.unwrap_or(8).min(10),
                "sfx" => s.settings.sfx = n.unwrap_or(10).min(10),
                "best_score" => s.best_score = n.unwrap_or(0),
                "cups_won" => s.cups_won = n.unwrap_or(0),
                "cups_played" => s.cups_played = n.unwrap_or(0),
                "best_round" => s.best_round = n.unwrap_or(0),
                "tutorial_done" => s.tutorial_done = b,
                _ => {
                    if let Some(rest) = k.strip_prefix("tt") {
                        let Some((idx, field)) = rest.split_once('_') else {
                            continue;
                        };
                        let Ok(i) = idx.parse::<usize>() else {
                            continue;
                        };
                        if i >= 5 {
                            continue;
                        }
                        match field {
                            "best" => s.records[i].best = v.parse().ok(),
                            "par" => s.records[i].par = v.parse().ok(),
                            "ghost" => {
                                let nums: Vec<i32> =
                                    v.split(',').filter_map(|x| x.parse().ok()).collect();
                                s.records[i].ghost = nums
                                    .chunks_exact(3)
                                    .map(|c| (c[0] as f32, c[1] as f32, c[2] as f32 / 1000.0))
                                    .collect();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut s = Save::default();
        s.settings.steer_assist = true;
        s.settings.game_speed = 85;
        s.best_score = 4321;
        s.cups_won = 2;
        s.records[2].best = Some(21.5);
        s.records[2].par = Some(19.25);
        s.records[2].ghost = vec![(10.0, -20.0, 1.5), (12.0, -18.0, 1.25)];
        let t = Save::parse(&s.serialize());
        assert_eq!(t, s);
    }

    #[test]
    fn medals() {
        assert_eq!(medal(20.0, 20.0), 3);
        assert_eq!(medal(22.0, 20.0), 2);
        assert_eq!(medal(24.0, 20.0), 1);
        assert_eq!(medal(30.0, 20.0), 0);
    }

    #[test]
    fn garbage_is_tolerated() {
        let s = Save::parse("nonsense\ngame_speed=13\ntt9_best=4\ntt1_ghost=1,2\n");
        assert_eq!(s.settings.game_speed, 100);
        assert!(s.records[1].ghost.is_empty());
    }
}
