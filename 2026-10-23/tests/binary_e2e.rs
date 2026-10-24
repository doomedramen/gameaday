use std::process::Command;
#[test]
fn shipped_binary_selftest_and_screenshots() {
    let exe = env!("CARGO_BIN_EXE_night_signal");
    let out = Command::new(exe).arg("--selftest").output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dir = std::env::temp_dir().join("night-signal-shots");
    let _ = std::fs::remove_dir_all(&dir);
    let out = Command::new(exe)
        .arg("--screenshot")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(std::fs::read_dir(dir).unwrap().count(), 4);
}
