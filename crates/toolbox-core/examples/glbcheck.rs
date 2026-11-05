//! Dev helper: export every model in the ROM to glb and report how many materials end up without a texture.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = toolbox_core::session::RomSession::open(std::path::Path::new(&a[1])).unwrap();
    let filter = a.get(2).cloned().unwrap_or_default();
    let paths: Vec<String> = s
        .nodes
        .iter()
        .filter(|n| n.format == "nsbmd" && n.path.contains(&filter))
        .map(|n| n.path.clone())
        .collect();
    for p in paths {
        let r = toolbox_core::scene::CourseScene::load(None, &s, &p).and_then(|c| c.glb(false));
        match r {
            Ok(b) => {
                let len = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
                let j: serde_json::Value = serde_json::from_slice(&b[20..20 + len]).unwrap();
                let mats = j["materials"].as_array().map_or(0, |m| m.len());
                let textured = j["materials"].as_array().map_or(0, |m| {
                    m.iter()
                        .filter(|m| !m["pbrMetallicRoughness"]["baseColorTexture"].is_null())
                        .count()
                });
                println!("{textured:3}/{mats:3} {p}");
            }
            Err(e) => println!("ERR {p}: {e}"),
        }
    }
}
