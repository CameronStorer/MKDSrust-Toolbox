use std::collections::BTreeMap;
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
    track_wizard,
};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = RomSession::open(std::path::Path::new(&a[1])).unwrap();
    let mut p = Project::in_memory(&s, Manifest::default(), BTreeMap::new()).unwrap();
    let src = toolbox_core::model_import::import(std::path::Path::new(&a[2])).unwrap();
    let r = track_wizard::track_from(
        &mut p,
        &s,
        &track_wizard::TrackOptions {
            course: a[4].parse().unwrap(),
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
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    std::fs::write(&a[3], bytes).unwrap();
    println!("wrote {}", a[3]);
}
