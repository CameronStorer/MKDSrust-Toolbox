//! Dev helper: run the character or kart wizard and install the result as an MKDSrust mod (race checks).
//! Usage: wizmod <rom> <character|kart> <model file> <project dir> <MKDSrust game folder> [v3]
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
                hips: 0.45,
                rotation: [0., 0., 0., 1.],
            },
        )
    }
    .unwrap();
    for w in &r.warnings {
        println!("warning: {w}");
    }
    // Legacy package format: what the current MKDSrust game reads (as Test Bench does).
    // Pass "v3" as a sixth argument for the small format-3 package (needs a format-3 game).
    let packed = if a.get(6).map(String::as_str) == Some("v3") {
        let r = toolbox_core::pack::pack(&p, &s).unwrap();
        println!(
            "format-3 package: {} bytes",
            std::fs::metadata(&r.path).unwrap().len()
        );
        std::fs::read(&r.path).unwrap()
    } else {
        toolbox_core::pack::build_legacy(&p, &s).unwrap().0
    };
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
