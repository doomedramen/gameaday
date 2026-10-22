//! Persistent save file: best results and the tutorial flag.
//! Plain `key=value` lines; unknown keys are ignored, corrupt values reset.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Save {
    pub best_score: i64,
    pub best_island: u32,
    pub runs: u32,
    pub wins: u32,
    pub tutorial_done: bool,
}

impl Save {
    /// `$LANTERNFALL_SAVE`, else `~/.lanternfall/save.txt`.
    pub fn default_path() -> PathBuf {
        if let Ok(p) = std::env::var("LANTERNFALL_SAVE") {
            return PathBuf::from(p);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".lanternfall").join("save.txt")
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
            match k.trim() {
                "best_score" => s.best_score = v.parse().unwrap_or(0),
                "best_island" => s.best_island = v.parse().unwrap_or(0),
                "runs" => s.runs = v.parse().unwrap_or(0),
                "wins" => s.wins = v.parse().unwrap_or(0),
                "tutorial_done" => s.tutorial_done = v == "1" || v == "true",
                _ => {}
            }
        }
        s
    }

    pub fn serialize(&self) -> String {
        format!(
            "best_score={}\nbest_island={}\nruns={}\nwins={}\ntutorial_done={}\n",
            self.best_score,
            self.best_island,
            self.runs,
            self.wins,
            if self.tutorial_done { 1 } else { 0 }
        )
    }

    pub fn store(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.serialize())
    }

    /// Fold a finished run in. Returns true if it set a new best score.
    pub fn record_run(&mut self, score: i64, island: u32, won: bool) -> bool {
        self.runs += 1;
        if won {
            self.wins += 1;
        }
        self.best_island = self.best_island.max(island);
        if score > self.best_score {
            self.best_score = score;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_corruption() {
        let s = Save {
            best_score: 1234,
            best_island: 5,
            runs: 3,
            wins: 1,
            tutorial_done: true,
        };
        assert_eq!(Save::parse(&s.serialize()), s);
        let bad = Save::parse("best_score=abc\nwins=2\ngarbage\n");
        assert_eq!(bad.best_score, 0);
        assert_eq!(bad.wins, 2);
    }

    #[test]
    fn record_run_tracks_best() {
        let mut s = Save::default();
        assert!(s.record_run(100, 2, false));
        assert!(!s.record_run(50, 4, false));
        assert!(s.record_run(900, 7, true));
        assert_eq!(
            (s.runs, s.wins, s.best_island, s.best_score),
            (3, 1, 7, 900)
        );
    }

    #[test]
    fn store_and_load_file() {
        let dir = std::env::temp_dir().join(format!("lanternfall-test-{}", std::process::id()));
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
