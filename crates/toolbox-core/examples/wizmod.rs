//! Dev helper: run the character or kart wizard and install the result as an MKDSrust mod (race checks).
//! Usage: wizmod <rom> <character|kart> <model file> <project dir> <MKDSrust game folder>
use std::io::Read;
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
    wizard,
};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = RomSession::open(std::path::Path::new(&a[1])).unwrap();
    let (project_dir, game) = (std::path::Path::new(&a[4]), std::path::Path::new(&a[5]));
    let _ = std::fs::remove_dir_all(project_dir);
    let mut p = Project::create(project_dir, &s, Manifest::default()).unwrap();
    let r = if a[2] == "kart" {
        wizard::kart(
            &mut p,
            &s,
            &wizard::KartOptions {
                slot: 0,
                base: 0,
                source: a[3].clone(),
                tire: "M".into(),
                wheels: None,
                seat: [0.; 3],
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: 1.,
            },
        )
    } else {
        wizard::character(
            &mut p,
            &s,
            &wizard::CharacterOptions {
                slot: 0,
                base: 0,
                source: a[3].clone(),
                hips: 0.,
                rotation: [0., 0., 0., 1.],
            },
        )
    }
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
    println!("installed {id}");
}
