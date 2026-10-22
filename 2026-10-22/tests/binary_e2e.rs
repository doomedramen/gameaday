//! Runs the shipped binary headlessly: the full selftest (simulation
//! playtests + the real app driven by synthetic input to victory and
//! defeat), then the screenshot mode.

use std::process::Command;

#[test]
fn binary_selftest_passes() {
    let out = Command::new(env!("CARGO_BIN_EXE_lanternfall"))
        .arg("--selftest")
        .output()
        .expect("run binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "selftest failed:\n{stdout}\n{stderr}");
    assert!(stdout.contains("SELFTEST OK"));
    assert!(stdout.contains("victory reachable"));
}

#[test]
fn binary_writes_screenshots() {
    let dir = std::env::temp_dir().join(format!("lanternfall-shots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = Command::new(env!("CARGO_BIN_EXE_lanternfall"))
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
    assert!(pngs.len() >= 10, "only {} screenshots", pngs.len());
    for p in &pngs {
        let bytes = std::fs::read(p.path()).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        assert!(bytes.len() > 20_000, "{:?} suspiciously small", p.path());
    }
    let _ = std::fs::remove_dir_all(&dir);
}
