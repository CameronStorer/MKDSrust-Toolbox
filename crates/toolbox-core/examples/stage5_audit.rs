//! Local structure-only diagnostics. Does not extract or persist game data.
use toolbox_core::{session::RomSession, Result};
fn main() -> Result<()> {
    let s = RomSession::open(std::path::Path::new(
        &std::env::var_os("MKDS_ROM").expect("set MKDS_ROM"),
    ))?;
    let a = s.file(nitro_formats::executable::ARM9)?;
    println!(
        "ARM9 stored={} loaded={} overlays={}",
        s.rom.arm9.len(),
        a.len(),
        s.rom.arm9_overlays.len()
    );
    for path in [
        "data/KartModelMainB.carc/character/heyho/P_HH.nsbmd",
        "data/KartModelSub.carc/character/mario/P_MR.nsbmd",
        "data/KartModelMainA.carc/kart/mario/kart_MR_b.nsbmd",
    ] {
        let m = nitro_formats::model::Nsbmd::parse(s.file(path)?)?;
        let m = &m.models[0];
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for p in m.meshes.iter().flat_map(|m| &m.positions) {
            for k in 0..3 {
                low[k] = low[k].min(p[k]);
                high[k] = high[k].max(p[k]);
            }
        }
        println!(
            "{path}: bounds {low:?}..{high:?}, bones={} materials={}",
            m.bones.len(),
            m.materials.len()
        );
    }
    for n in s
        .nodes
        .iter()
        .filter(|n| n.path.contains("Map2D/local") && n.path.contains("cross_courseTex"))
    {
        println!("{} {} bytes", n.path, n.size);
    }
    for n in s.nodes.iter().filter(|n| {
        n.path.contains("KartModelMenu") && (n.path.contains("HH") || n.path.contains("heyho"))
    }) {
        println!("{}", n.path);
    }
    Ok(())
}
