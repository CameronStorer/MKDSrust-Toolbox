//! Dev helper: apply recipe actions (JSON list) to the user's ROM and write a patched ROM for testing.
use std::collections::BTreeMap;
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
    wizard,
};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = RomSession::open(std::path::Path::new(&a[1])).unwrap();
    let mut p = Project::in_memory(&s, Manifest::default(), BTreeMap::new()).unwrap();
    let actions: Vec<wizard::Action> =
        serde_json::from_str(&std::fs::read_to_string(&a[2]).unwrap()).unwrap();
    for action in actions {
        let r = wizard::apply(&mut p, &s, action).unwrap();
        println!("files {:?} warnings {:?}", r.files, r.warnings);
    }
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    std::fs::write(&a[3], bytes).unwrap();
    println!("wrote {}", a[3]);
}
