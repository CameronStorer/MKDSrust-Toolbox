//! ROM rebuild and BPS export: synthetic ROMs always, the user's own ROM when `MKDS_ROM` is set.
use nitro_formats::{bps, rom::Rom, rom_build};
use std::collections::BTreeMap;
use toolbox_core::synthetic;

fn header_crc_ok(rom: &[u8]) -> bool {
    rom_build::crc16(&rom[..0x15E]) == u16::from_le_bytes([rom[0x15E], rom[0x15F]])
}

#[test]
fn crc16_matches_the_standard_check_value() {
    assert_eq!(rom_build::crc16(b"123456789"), 0x4B37);
}

#[test]
fn files_change_in_place_or_move_and_everything_else_stays() {
    let original = synthetic::rom();
    let rom = Rom::parse(&original).unwrap();
    let mut names: Vec<&String> = rom.files.keys().collect();
    names.sort_by_key(|n| rom.files[*n].len());
    let small = names[0].clone();
    let big = names[names.len() - 1].clone();
    // shrink one file (in place), grow another far past its slot (moved to the end)
    let mut changes = BTreeMap::new();
    changes.insert(small.clone(), vec![7u8; 1.min(rom.files[&small].len())]);
    changes.insert(big.clone(), vec![9u8; rom.files[&big].len() + 5000]);
    let (out, report) = rom_build::rebuild(&original, &changes).unwrap();
    assert!(
        report.in_place.contains(&small) && report.moved.contains(&big),
        "{report:?}"
    );
    assert!(header_crc_ok(&out));
    let back = Rom::parse(&out).unwrap();
    for (name, range) in &rom.files {
        let want = changes
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&original[range.clone()]);
        assert_eq!(back.file(name), Some(want), "{name}");
    }
    // the moved file is 0x200-aligned and the code sections didn't move
    assert_eq!(back.files[&big].start % 0x200, 0);
    assert_eq!(
        (back.arm9.clone(), back.arm7.clone()),
        (rom.arm9.clone(), rom.arm7.clone())
    );
    // a BPS of the build applies to the original and reproduces it exactly
    let patch = bps::create(&original, &out);
    assert_eq!(bps::apply(&original, &patch).unwrap(), out);
}

#[test]
fn an_unchanged_rebuild_only_fixes_the_header_crc() {
    let original = synthetic::rom();
    let (out, report) = rom_build::rebuild(&original, &BTreeMap::new()).unwrap();
    assert!(report.in_place.is_empty() && report.moved.is_empty());
    assert_eq!(out[..0x15E], original[..0x15E]);
    assert_eq!(out[0x160..], original[0x160..]);
}

#[test]
fn unknown_paths_are_refused() {
    let mut changes = BTreeMap::new();
    changes.insert("nope/missing.bin".to_string(), vec![1]);
    assert!(rom_build::rebuild(&synthetic::rom(), &changes).is_err());
}

/// The user's own ROM (never stored): a no-op rebuild is identical, and a changed file round-trips through a BPS.
#[test]
fn retail_rom_rebuilds_and_patches() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("MKDS_ROM not set: skipped");
        return;
    };
    let original = std::fs::read(path).unwrap();
    assert!(
        header_crc_ok(&original),
        "the retail header CRC should already be valid"
    );
    let (same, _) = rom_build::rebuild(&original, &BTreeMap::new()).unwrap();
    assert!(
        same == original,
        "a rebuild with no changes must reproduce the ROM byte for byte"
    );
    let rom = Rom::parse(&original).unwrap();
    // change a mid-sized file in place (same length, bytes flipped) and grow another so it moves
    let mut files: Vec<(&String, usize)> = rom
        .files
        .iter()
        .map(|(n, r)| (n, r.len()))
        .filter(|(_, l)| *l > 1024)
        .collect();
    files.sort_by_key(|(n, _)| n.to_string());
    let (a, alen) = files[files.len() / 2];
    let (b, blen) = files[files.len() / 3];
    let mut changes = BTreeMap::new();
    changes.insert(
        a.clone(),
        original[rom.files[a].clone()]
            .iter()
            .map(|x| !x)
            .collect::<Vec<u8>>(),
    );
    changes.insert(b.clone(), vec![0x5A; blen + 0x10000]);
    let (out, report) = rom_build::rebuild(&original, &changes).unwrap();
    assert!(
        report.in_place.contains(a) && report.moved.contains(b),
        "{report:?} ({alen})"
    );
    assert!(header_crc_ok(&out));
    let patch = bps::create(&original, &out);
    assert!(
        patch.len() < alen + blen + 0x10000 + 4096,
        "patch {} bytes",
        patch.len()
    );
    assert_eq!(bps::apply(&original, &patch).unwrap(), out);
}

/// End to end on the user's ROM: edit a palette inside a compressed archive, export a ROM and a BPS, and read the
/// edit back out of both.
#[test]
fn retail_project_exports_for_ds() {
    use toolbox_core::{
        export,
        project::{Manifest, Project},
        session::RomSession,
    };
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("MKDS_ROM not set: skipped");
        return;
    };
    let session = RomSession::open(std::path::Path::new(&path)).unwrap();
    let node = session
        .nodes
        .iter()
        .find(|n| n.path.contains(".carc/") && n.path.to_ascii_lowercase().ends_with(".nclr"))
        .expect("a palette inside a .carc");
    let mut bytes = session.file(&node.path).unwrap().to_vec();
    let last = bytes.len() - 2;
    bytes[last] ^= 0x1F; // one colour changes
    let mut edits = std::collections::BTreeMap::new();
    edits.insert(node.path.clone(), bytes.clone());
    let manifest = Manifest {
        id: "test.export".into(),
        name: "Export test".into(),
        version: "1.0.0".into(),
        author: "Test".into(),
        kind: "texture".into(),
        description: String::new(),
        platforms: vec!["pc".into(), "ds".into()],
    };
    let project = Project::in_memory(&session, manifest, edits).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (rom_out, bps_out) = (dir.path().join("hack.nds"), dir.path().join("hack.bps"));
    let r = export::export_ds(&project, &session, Some(&rom_out), Some(&bps_out)).unwrap();
    assert_eq!(r.changed.len(), 1, "{r:?}");
    let original = std::fs::read(&path).unwrap();
    let hacked = std::fs::read(&rom_out).unwrap();
    assert_eq!(
        nitro_formats::bps::apply(&original, &std::fs::read(&bps_out).unwrap()).unwrap(),
        hacked
    );
    let back = RomSession::from_bytes(hacked, rom_out.clone()).unwrap();
    assert_eq!(back.file(&node.path).unwrap(), bytes.as_slice());
    eprintln!(
        "exported {} ({} bytes patch): {r:?}",
        node.path,
        r.patch_size.unwrap()
    );
}
