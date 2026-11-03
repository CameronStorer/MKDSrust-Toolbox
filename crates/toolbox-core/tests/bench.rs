use std::{
    path::Path,
    time::{Duration, Instant},
};
use toolbox_core::{
    bench::Bench,
    project::{Manifest, Project},
    session::RomSession,
    synthetic,
};
#[test]
fn game_process_fixture() {
    if !std::env::args().any(|a| a == "--exact") {
        return;
    }
    println!("synthetic stdout");
    eprintln!("synthetic stderr");
    std::thread::sleep(Duration::from_millis(300));
    std::process::exit(7);
}
#[test]
fn game_process_wait() {
    if !std::env::args().any(|a| a == "--exact") {
        return;
    }
    println!("waiting");
    std::thread::sleep(Duration::from_secs(30));
}
fn setup(root: &Path, helper: &str) -> (Project, RomSession) {
    let folder = root.join("games/synthetic");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::create_dir(root.join("data")).unwrap();
    let exe = format!("fixture{}", std::env::consts::EXE_SUFFIX);
    std::fs::copy(std::env::current_exe().unwrap(), folder.join(&exe)).unwrap();
    std::fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(
            &serde_json::json!({"executable":exe,"launch_args":["--exact",helper,"--nocapture"]}),
        )
        .unwrap(),
    )
    .unwrap();
    let session = RomSession::from_bytes(synthetic::rom(), root.join("own.nds")).unwrap();
    let mut project =
        Project::create(&root.join("project"), &session, Manifest::default()).unwrap();
    let path = "data/Studio/demo.carc/art/studio.nclr";
    let mut palette = nitro_formats::graphics::Nclr::parse(session.file(path).unwrap()).unwrap();
    palette.colors[0] ^= 1;
    project
        .edit(path, palette.write(), "test palette", &session)
        .unwrap();
    (project, session)
}
#[test]
fn installs_replaces_preserves_load_order_and_streams_exit() {
    let temp = tempfile::tempdir().unwrap();
    let (project, session) = setup(temp.path(), "game_process_fixture");
    let mods = temp.path().join("games/synthetic/mods");
    std::fs::create_dir(&mods).unwrap();
    std::fs::write(mods.join("enabled.txt"), "another.mod\n").unwrap();
    let mut bench = Bench::default();
    bench
        .play(temp.path(), "synthetic", &project, &session)
        .unwrap();
    assert!(bench.snapshot(temp.path()).running);
    let deadline = Instant::now() + Duration::from_secs(10);
    while bench.snapshot(temp.path()).running {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
        bench.poll().unwrap();
    }
    let snapshot = bench.snapshot(temp.path());
    assert_eq!(snapshot.last_run.as_ref().unwrap().exit_code, Some(7));
    let logs = snapshot.logs.join("");
    assert!(logs.contains("synthetic stdout"));
    assert!(logs.contains("synthetic stderr"));
    let order = std::fs::read_to_string(mods.join("enabled.txt")).unwrap();
    assert_eq!(
        order,
        format!("another.mod\n{}\n", project.config.manifest.id)
    );
    bench
        .play(temp.path(), "synthetic", &project, &session)
        .unwrap();
    bench.stop().unwrap();
    assert_eq!(
        std::fs::read_to_string(mods.join("enabled.txt")).unwrap(),
        order
    );
    assert!(mods
        .join(&project.config.manifest.id)
        .join("manifest.json")
        .is_file());
    assert!(!std::fs::read_dir(mods)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with(".toolbox-")));
}
#[test]
fn stops_owned_child_and_rejects_bad_game() {
    let temp = tempfile::tempdir().unwrap();
    let (project, session) = setup(temp.path(), "game_process_wait");
    let mut bench = Bench::default();
    assert!(bench
        .play(temp.path(), "../escape", &project, &session)
        .is_err());
    bench
        .play(temp.path(), "synthetic", &project, &session)
        .unwrap();
    assert!(bench
        .play(temp.path(), "synthetic", &project, &session)
        .is_err());
    bench.stop().unwrap();
    let snapshot = bench.snapshot(temp.path());
    assert!(!snapshot.running);
    assert!(snapshot.last_run.unwrap().stopped);
}
