//! Dev helper: run the track wizard on a map and install the result as an MKDSrust mod (for race checks).
use std::io::Read;
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
    track_wizard,
};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (rom, map, project_dir, game) = (
        &a[1],
        &a[2],
        std::path::Path::new(&a[3]),
        std::path::Path::new(&a[4]),
    );
    let s = RomSession::open(std::path::Path::new(rom)).unwrap();
    let _ = std::fs::remove_dir_all(project_dir);
    let mut p = Project::create(project_dir, &s, Manifest::default()).unwrap();
    let src = toolbox_core::model_import::import(std::path::Path::new(map)).unwrap();
    let r = track_wizard::track_from(
        &mut p,
        &s,
        &track_wizard::TrackOptions {
            course: 20,
            source: String::new(),
            line: vec![
                [0.15, 0.2],
                [0.85, 0.15],
                [0.9, 0.5],
                [0.6, 0.55],
                [0.75, 0.85],
                [0.2, 0.85],
                [0.1, 0.5],
            ],
            width: 100.,
            laps: 3,
            item_rows: 3,
        },
        src,
    )
    .unwrap();
    for w in &r.warnings {
        println!("warning: {w}");
    }
    // Legacy package format: what the current MKDSrust game reads (as Test Bench does).
    let (packed, _) = toolbox_core::pack::build_legacy(&p, &s).unwrap();
    let id = p.config.manifest.id.clone();
    let dest = game.join("mods").join(&id);
    let _ = std::fs::remove_dir_all(&dest);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(packed)).unwrap();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if f.is_dir() {
            continue;
        }
        let out = dest.join(f.name().unwrap().to_string());
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let mut b = vec![];
        f.read_to_end(&mut b).unwrap();
        std::fs::write(out, b).unwrap();
    }
    std::fs::write(game.join("mods/enabled.txt"), format!("{id}\n")).unwrap();
    println!("installed {id} into {}", dest.display());
}
