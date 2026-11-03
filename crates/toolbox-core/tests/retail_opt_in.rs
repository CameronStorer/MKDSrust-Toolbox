//! Local opt-in only. Nothing derived from the user's ROM is written to disk.
use nitro_formats::{archive::Narc, bmg::Bmg, graphics::Nclr};
use std::io::{Cursor, Read};
use toolbox_core::{
    pack::{build, Entry, PackageManifest},
    project::{Manifest, Project},
    session::RomSession,
};
#[test]
fn user_rom_roundtrips_and_palette_patch() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM to test your own AMCE dump");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    assert!(!s.nodes.is_empty());
    let mut palette = None;
    for node in &s.nodes {
        let bytes = s.file(&node.path).unwrap();
        let plain = if bytes.first() == Some(&0x10) {
            nitro_formats::decompress(bytes).unwrap()
        } else {
            bytes.to_vec()
        };
        if plain.starts_with(b"NARC") {
            assert_eq!(Narc::parse(&plain).unwrap().write(), plain, "{}", node.path);
        }
        if plain.starts_with(b"MESGbmg1") {
            assert_eq!(Bmg::parse(&plain).unwrap().write(), plain, "{}", node.path);
        }
        if plain.starts_with(b"RLCN") {
            let p = Nclr::parse(&plain).unwrap();
            assert_eq!(p.write(), plain, "{}", node.path);
            if !p.colors.is_empty() && palette.is_none() {
                palette = Some((node.path.clone(), bytes.to_vec(), p));
            }
        }
    }
    let (path, original, mut colors) = palette.expect("ROM should contain an NCLR palette");
    colors.colors[0] ^= 1;
    let edited = if original.first() == Some(&0x10) {
        nitro_formats::recompress(&original, &colors.write())
    } else {
        colors.write()
    };
    // Project exists solely in memory; even edits/ is never written.
    let project = Project::in_memory(
        &s,
        Manifest::default(),
        std::collections::BTreeMap::from([(path.clone(), edited.clone())]),
    )
    .unwrap();
    let (zip, _) = build(&project, &s).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let m: PackageManifest = serde_json::from_reader(z.by_name("manifest.json").unwrap()).unwrap();
    let root = s.root_for(&path).unwrap();
    let Entry::Patch { patch } = &m.files[root] else {
        panic!("existing ROM file must be patched")
    };
    let mut patch_bytes = vec![];
    z.by_name(patch)
        .unwrap()
        .read_to_end(&mut patch_bytes)
        .unwrap();
    let mut result = vec![];
    qbsdiff::Bspatch::new(&patch_bytes)
        .unwrap()
        .apply(s.rom.file(root).unwrap(), Cursor::new(&mut result))
        .unwrap();
    let mut current = root.to_string();
    let mut bytes = result;
    while current != path {
        let plain = if bytes.first() == Some(&0x10) {
            nitro_formats::decompress(&bytes).unwrap()
        } else {
            bytes
        };
        let a = Narc::parse(&plain).unwrap();
        let entry = a
            .entries
            .into_iter()
            .find(|e| {
                let full = format!("{current}/{}", e.name);
                path == full || path.starts_with(&format!("{full}/"))
            })
            .expect("edited archive member");
        current = format!("{current}/{}", entry.name);
        bytes = entry.data;
    }
    assert_eq!(bytes, edited);
}
