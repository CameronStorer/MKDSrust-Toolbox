use std::{fs, process::Command};
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
    synthetic,
};
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mkdstb"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn headless_commands_share_the_project_pipeline() {
    let dir = tempfile::tempdir().unwrap();
    let rom = dir.path().join("synthetic.nds");
    fs::write(&rom, synthetic::rom()).unwrap();
    let r = rom.to_str().unwrap();
    let result = cli(&["ls", r, "data/Studio/demo.carc"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("art/studio.nclr"));
    let path = "data/Studio/demo.carc/art/studio.nclr";
    let palette = dir.path().join("palette.nclr");
    let result = cli(&["extract", r, path, "-o", palette.to_str().unwrap()]);
    assert!(result.status.success());
    let result = cli(&["convert", palette.to_str().unwrap(), "--to", "png"]);
    assert!(result.status.success());
    assert!(fs::read(palette.with_extension("png"))
        .unwrap()
        .starts_with(b"\x89PNG"));
    let result = cli(&["convert", palette.to_str().unwrap(), "--to", "json"]);
    assert!(result.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(palette.with_extension("json")).unwrap()).unwrap();
    assert_eq!(json["colors"].as_array().unwrap().len(), 256);
    let session = RomSession::open(&rom).unwrap();
    let project_dir = dir.path().join("project");
    let mut project = Project::create(&project_dir, &session, Manifest::default()).unwrap();
    let mut colors = nitro_formats::graphics::Nclr::parse(session.file(path).unwrap()).unwrap();
    colors.colors[0] ^= 31;
    project
        .edit(path, colors.write(), "palette", &session)
        .unwrap();
    let result = cli(&["validate", project_dir.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = cli(&["pack", project_dir.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(project_dir.join("build/author.my-mod-0.1.0.zip").exists());
    assert!(
        !cli(&["extract", r, "../missing", "-o", palette.to_str().unwrap()])
            .status
            .success()
    );
}
