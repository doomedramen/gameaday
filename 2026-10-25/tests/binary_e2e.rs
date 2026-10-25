//! Runs the shipped binary headlessly: the full selftest (generator oracle,
//! whole Cups through the real app, elimination, time trial ghosts, save
//! file), then the screenshot mode.

use std::process::Command;

#[test]
fn binary_selftest_passes() {
    let out = Command::new(env!("CARGO_BIN_EXE_slipstream-cup"))
        .arg("--selftest")
        .output()
        .expect("run binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "selftest failed:\n{stdout}\n{stderr}");
    assert!(stdout.contains("SELFTEST OK"));
    assert!(stdout.contains("victory reachable"));
    assert!(stdout.contains("defeat reachable"));
    assert!(stdout.contains("difficulty rises"));
    assert!(stdout.contains("after 15 min"));
}

#[test]
fn binary_writes_screenshots() {
    let dir = std::env::temp_dir().join(format!("slipstream-shots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = Command::new(env!("CARGO_BIN_EXE_slipstream-cup"))
        .arg("--screenshot")
        .arg(&dir)
        .output()
        .expect("run binary");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pngs: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
        .collect();
    assert!(pngs.len() >= 20, "only {} screenshots", pngs.len());
    for p in &pngs {
        let bytes = std::fs::read(p.path()).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        assert!(bytes.len() > 20_000, "{:?} suspiciously small", p.path());
    }
    let _ = std::fs::remove_dir_all(&dir);
}
