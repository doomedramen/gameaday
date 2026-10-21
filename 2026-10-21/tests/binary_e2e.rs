//! End-to-end test of the shipping binary: builds it, runs `--selftest`
//! which drives the real update/render/audio pipeline headlessly, and checks
//! it exits cleanly.
//!
//! This is the quality gate required by the project: the game must be
//! playable in one sitting without bugs.

use std::process::Command;

#[test]
fn binary_selftest_passes() {
    let exe = env!("CARGO_BIN_EXE_gameaday_2026_10_21");
    let out = Command::new(exe)
        .arg("--selftest")
        .output()
        .expect("failed to run the game binary");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "selftest failed with {:?}\n{}",
        out.status,
        stderr
    );
    assert!(
        stderr.contains("selftest: OK"),
        "selftest did not report OK:\n{stderr}"
    );
}
