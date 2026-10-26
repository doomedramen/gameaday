use std::{fs, process::Command};
#[test]
fn shipped_binary_plays_long_campaigns_and_renders() {
    let binary = env!("CARGO_BIN_EXE_the-last-glasshouse");
    let output = Command::new(binary)
        .arg("--selftest")
        .output()
        .expect("run shipped binary");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(report.contains("SELFTEST OK"));
    assert!(report.contains("48"));
    assert!(report.contains("idle loss reachable"));
    let dir = std::env::temp_dir().join(format!("glasshouse-e2e-{}", std::process::id()));
    let result = Command::new(binary)
        .arg("--screenshot")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for name in [
        "01_title.png",
        "02_early_garden.png",
        "03_cutting_draft.png",
        "04_three_doors.png",
        "05_keeper_of_ash.png",
        "06_white_conservatory.png",
        "07_ending.png",
        "08_field_notes.png",
        "09_settings.png",
        "10_collection.png",
    ] {
        let data = fs::read(dir.join(name)).unwrap();
        assert!(data.len() > 25000, "blank {name}");
        let decoder = png::Decoder::new(std::io::Cursor::new(&data));
        let reader = decoder.read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (1440, 900));
    }
    fs::remove_dir_all(dir).unwrap();
}
