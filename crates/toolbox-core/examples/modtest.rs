//! Dev helper: rewrite one retail file through our parser/writer and install it as an MKDSrust mod.
use std::{collections::BTreeMap, io::Read};
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (rom, project_dir, game, mode) = (
        &a[1],
        std::path::Path::new(&a[2]),
        std::path::Path::new(&a[3]),
        a[4].as_str(),
    );
    let s = RomSession::open(std::path::Path::new(rom)).unwrap();
    let _ = std::fs::remove_dir_all(project_dir);
    let mut p = Project::create(project_dir, &s, Manifest::default()).unwrap();
    let path = "data/Course/cross_course.carc/course_model.nsbmd";
    let mut m = nitro_formats::model::Nsbmd::parse(
        &toolbox_core::preview::plain(s.file(path).unwrap()).unwrap(),
    )
    .unwrap();
    if mode == "scale64" {
        for mesh in &mut m.models[0].meshes {
            for p in &mut mesh.positions {
                for v in p.iter_mut() {
                    *v *= 64.;
                }
            }
        }
    }
    let bytes = if mode == "raw" {
        toolbox_core::preview::plain(s.file(path).unwrap()).unwrap()
    } else {
        m.write().unwrap()
    };
    p.edit_many(BTreeMap::from([(path.to_string(), bytes)]), "test", &s)
        .unwrap();
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
    println!("installed");
}
