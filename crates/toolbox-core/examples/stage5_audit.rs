//! Local structure-only diagnostics. Does not extract or persist game data.
use toolbox_core::{session::RomSession, Result};
fn main() -> Result<()> {
    let s = RomSession::open(std::path::Path::new(
        &std::env::var_os("MKDS_ROM").expect("set MKDS_ROM"),
    ))?;
    let a = s.file(nitro_formats::executable::ARM9)?;
    let kart = nitro_formats::model::Nsbmd::parse(
        s.file("data/KartModelMainA.carc/kart/mario/kart_MR_a.nsbmd")?,
    )?;
    let m = &kart.models[0];
    let wheels: Vec<_> = m
        .meshes
        .iter()
        .filter(|mesh| m.materials[mesh.material].name == "kart_tire")
        .flat_map(|mesh| &mesh.positions)
        .copied()
        .collect();
    let offset = s.file("data/KartModelMainA.carc/kartoffsetdata.bin")?;
    for (i, (xsign, zsign)) in [(1., 1.), (-1., 1.), (1., -1.), (-1., -1.)]
        .iter()
        .enumerate()
    {
        let points: Vec<_> = wheels
            .iter()
            .filter(|p| p[0] * xsign > 0. && p[2] * zsign > 0.)
            .collect();
        let center: [f32; 3] = std::array::from_fn(|k| {
            (points.iter().map(|p| p[k]).fold(f32::INFINITY, f32::min)
                + points
                    .iter()
                    .map(|p| p[k])
                    .fold(f32::NEG_INFINITY, f32::max))
                / 2.
        });
        let raw = nitro_formats::nkm::vec3(offset, 20 + i * 12)?;
        println!(
            "wheel {i}: baked center {center:?}, table {raw:?}, X/Z ratios {:?}",
            [raw[0] / center[0], raw[2] / center[2]]
        );
    }
    for n in s.nodes.iter().filter(|n| {
        n.path.contains("player_character")
            && (n.path.contains("Main2D_us") || n.path.contains("Main2D.carc"))
    }) {
        println!("portrait resource {} {} bytes", n.path, n.size);
    }
    let sound = nitro_formats::audio::Sdat::parse(s.file("data/Sound/sound_data.sdat")?)?;
    let voices = nitro_formats::audio::Ssar::parse(&sound.files[sound.resource_file(1, 2)?])?;
    println!("voice archive entries {}", voices.entries.len());
    for (character, entries) in voices.entries.chunks(14).enumerate() {
        println!(
            "voice character {character} banks {:?}",
            entries
                .iter()
                .map(|e| e.bank)
                .collect::<std::collections::BTreeSet<_>>()
        );
    }
    for archive in ["KartModelMainA", "KartModelMenu"] {
        let b = s.file(&format!("data/{archive}.carc/kartphysicalparam.bin"))?;
        println!(
            "{archive} first physical fields: {:?}",
            b[..32]
                .chunks_exact(4)
                .map(|q| i32::from_le_bytes(q.try_into().unwrap()) as f32 / 4096.)
                .collect::<Vec<_>>()
        );
    }
    let mut sizes = std::collections::BTreeMap::<String, usize>::new();
    for n in s
        .nodes
        .iter()
        .filter(|n| n.path.starts_with("data/Course/"))
    {
        if let Some((_, file)) = n.path.split_once(".carc/") {
            let bytes = toolbox_core::preview::plain(s.file(&n.path)?)?;
            let size = sizes.entry(file.to_ascii_lowercase()).or_default();
            *size = (*size).max(bytes.len());
        }
    }
    for (file, size) in sizes {
        if file.starts_with("course_") || file.starts_with("map2d/local") {
            println!("course budget {file}: {size} bytes");
        }
    }
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
