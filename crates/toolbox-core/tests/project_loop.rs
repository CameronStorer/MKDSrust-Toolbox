use nitro_formats::{archive::Narc, bmg::Bmg, graphics::Nclr, texture::Nsbtx};
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};
use toolbox_core::{
    pack::{build, Entry, PackageManifest},
    preview::TextToken,
    project::{check_path, Manifest, Project},
    service::{Request, Service},
    session::RomSession,
    synthetic,
};
fn session() -> RomSession {
    RomSession::from_bytes(synthetic::rom(), PathBuf::from("/user/synthetic.nds")).unwrap()
}
fn patch_result(zip: &[u8], session: &RomSession) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let mut metadata = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut metadata)
        .unwrap();
    let manifest: PackageManifest = serde_json::from_str(&metadata).unwrap();
    assert!([1, 3].contains(&manifest.format));
    assert_eq!(manifest.game, "AMCE");
    let mut out = vec![];
    for (path, entry) in manifest.files {
        match entry {
            Entry::Patch { patch } => {
                let mut bytes = vec![];
                archive
                    .by_name(&patch)
                    .unwrap()
                    .read_to_end(&mut bytes)
                    .unwrap();
                assert!(bytes.starts_with(b"BSDIFF40"));
                let mut edited = vec![];
                qbsdiff::Bspatch::new(&bytes)
                    .unwrap()
                    .apply(session.rom.file(&path).unwrap(), Cursor::new(&mut edited))
                    .unwrap();
                out.push((path, edited));
            }
            Entry::Archive { archive: members } => {
                let payloads = members
                    .values()
                    .map(|member| {
                        let mut bytes = vec![];
                        archive
                            .by_name(&member.patch)
                            .unwrap()
                            .read_to_end(&mut bytes)
                            .unwrap();
                        (member.patch.clone(), bytes)
                    })
                    .collect();
                let edited = toolbox_core::archive_delta::apply(
                    session.rom.file(&path).unwrap(),
                    &members,
                    &payloads,
                )
                .unwrap();
                out.push((path, edited));
            }
            Entry::Add { .. } => panic!("unexpected add"),
        }
    }
    // Neither an entire retail file nor an unpatched archive may appear as any payload.
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut bytes = vec![];
        file.read_to_end(&mut bytes).unwrap();
        for node in &session.nodes {
            assert_ne!(&bytes, session.file(&node.path).unwrap());
        }
    }
    out
}
#[test]
fn nested_palette_text_edits_history_persistence_and_reproducible_package() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), &s, Manifest::default()).unwrap();
    let palette = "data/Studio/demo.carc/art/studio.nclr";
    let text = "data/Studio/demo.carc/language/en.bmg";
    let before = s.file(palette).unwrap().to_vec();
    let mut service = Service {
        session: Some(s),
        project: Some(project),
        persist_settings: false,
        ..Service::default()
    };
    service
        .dispatch(Request::Palette {
            path: palette.into(),
            indices: vec![0],
            colors: vec![32767],
        })
        .unwrap();
    let edited = service.project.as_ref().unwrap().edits[palette].clone();
    assert_ne!(edited, before);
    let source = std::fs::read(dir.path().join("edits").join(palette)).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&source).unwrap();
    assert_eq!(json["changes"]["kind"], "palette");
    assert_eq!(json["changes"]["colors"].as_object().unwrap().len(), 1);
    assert!(!source.starts_with(b"RLCN"));
    service.dispatch(Request::Undo).unwrap();
    assert!(service.project.as_ref().unwrap().edits.is_empty());
    service.dispatch(Request::Redo).unwrap();
    assert_eq!(service.project.as_ref().unwrap().edits[palette], edited);
    let tokens = vec![TextToken::Text("My own synthetic message".into())];
    service
        .dispatch(Request::Text {
            path: text.into(),
            index: 0,
            tokens,
        })
        .unwrap();
    let s = service.session.as_ref().unwrap();
    let p = service.project.as_ref().unwrap();
    assert_eq!(p.history().len(), 2);
    let reopened = Project::open(dir.path(), s).unwrap();
    assert_eq!(p.edits, reopened.edits);
    let source = std::fs::read_to_string(dir.path().join("edits").join(text)).unwrap();
    assert!(source.contains("My own synthetic message"));
    assert!(!source.contains("Made by you."));
    let (zip, warnings) = build(&reopened, s).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(build(&reopened, s).unwrap().0, zip);
    let outputs = patch_result(&zip, s);
    assert_eq!(outputs.len(), 1);
    let plain = nitro_formats::decompress(&outputs[0].1).unwrap();
    let archive = Narc::parse(&plain).unwrap();
    assert_eq!(
        archive
            .entries
            .iter()
            .find(|e| e.name == "art/studio.nclr")
            .unwrap()
            .data,
        edited
    );
    let text = archive
        .entries
        .iter()
        .find(|e| e.name == "language/en.bmg")
        .unwrap();
    let b = Bmg::parse(&text.data).unwrap();
    assert_eq!(
        nitro_formats::bmg::text(&b.messages[0].tokens),
        "My own synthetic message"
    );
    // Never extracted a retail file into the project: only explicitly edited leaves exist.
    assert!(!dir.path().join("edits/data/Studio/texture.nsbtx").exists());
}
#[test]
fn path_hash_manifest_and_add_safety() {
    for p in [
        "../file",
        "/abs",
        "C:/rom",
        "a\\b",
        "a//b",
        "a/./b",
        "con/file",
        "ok/aux.nclr",
        "a:ads",
        "a/.",
    ] {
        assert!(check_path(p).is_err(), "{p}");
    }
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Project::create(dir.path(), &s, Manifest::default()).unwrap();
    assert!(p.edit("../oops", vec![0], "bad", &s).is_err());
    let retail = s.file("data/Studio/readout.bmg").unwrap().to_vec();
    p.edit("data/new.bmg", retail, "add", &s).unwrap();
    assert!(build(&p, &s).unwrap_err().to_string().contains("retail"));
    p.revert("data/new.bmg").unwrap();
    p.edit("data/a.exe", b"MZ".to_vec(), "add", &s).unwrap();
    assert!(build(&p, &s).is_err());
    p.revert("data/a.exe").unwrap();
    p.edit(
        "data/new.nclr",
        Nclr::new(&[0x7222; 16], false, false).write(),
        "own palette",
        &s,
    )
    .unwrap();
    let (bytes, _) = build(&p, &s).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert!(z.by_name("files/data/new.nclr").is_ok());
    let mut config = p.config.clone();
    config.rom_sha256 = "different".into();
    std::fs::write(
        dir.path().join("project.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    assert!(Project::open(dir.path(), &s).is_err());
}
#[test]
fn synthetic_rom_header_banner_and_texture_dictionary() {
    let s = session();
    assert_eq!(s.rom.bytes(), synthetic::rom());
    assert_eq!(s.rom.game_code, "AMCE");
    assert_eq!(s.rom.arm9.len(), 4);
    assert_eq!(s.rom.arm7.len(), 4);
    let b = s.rom.banner.as_ref().unwrap();
    assert_eq!(b.titles.len(), 6);
    assert_eq!(b.icon.len(), 512);
    let bytes = synthetic::texture_fixture();
    let texture = Nsbtx::parse(&bytes).unwrap();
    assert_eq!(texture.write(), bytes);
    let image = texture.decode(0, 0).unwrap();
    assert_eq!(image.width, 8);
    assert_eq!(image.rgba[3], 255);
    for i in 0..bytes.len() {
        let _ = Nsbtx::parse(&bytes[..i]);
    }
    for i in 0..bytes.len() {
        let mut b = bytes.clone();
        b[i] = 255;
        let _ = Nsbtx::parse(&b);
    }
    for i in 0..s.rom.bytes().len() {
        if i < 512 || i % 128 == 0 {
            let _ = nitro_formats::rom::Rom::parse(&s.rom.bytes()[..i]);
        }
    }
}

#[test]
fn nsbtx_dictionaries_roundtrip_every_encoded_texture_format() {
    let read = |b: &[u8], p: usize| u32::from_le_bytes(b[p..p + 4].try_into().unwrap()) as usize;
    let put = |b: &mut [u8], p: usize, n: u32| b[p..p + 4].copy_from_slice(&n.to_le_bytes());
    for format in 1..=7 {
        for mode in 0..4u16 {
            let mut b = synthetic::texture_fixture();
            let section = 20;
            let data = read(&b, section + 20);
            let palette = read(&b, section + 56);
            // Retain valid real dictionaries while generating separate pixel/descriptor arrays.
            b.resize(section + palette + 512, 0);
            b[section + 48..section + 50].copy_from_slice(&64u16.to_le_bytes());
            let dict =
                u16::from_le_bytes(b[section + 14..section + 16].try_into().unwrap()) as usize;
            put(&mut b, section + dict + 20, (format << 26) | (1 << 29));
            b[section + data..section + data + 128].fill(0);
            if format == 5 {
                b[section + 28..section + 30].copy_from_slice(&2u16.to_le_bytes());
                put(&mut b, section + 40, (data + 16) as u32);
                for descriptor in b[section + data + 16..section + data + 24].chunks_exact_mut(2) {
                    descriptor.copy_from_slice(&(mode << 14).to_le_bytes());
                }
            }
            for (i, c) in b[section + palette..].chunks_exact_mut(2).enumerate() {
                c.copy_from_slice(&(i as u16).to_le_bytes());
            }
            let len = b.len();
            put(&mut b, 8, len as u32);
            put(&mut b, section + 4, (len - section) as u32);
            let parsed = Nsbtx::parse(&b).unwrap();
            assert_eq!(parsed.write(), b);
            assert_eq!(parsed.textures[0].format, format as u8);
            let image = parsed.decode(0, 0).unwrap();
            assert_eq!(image.rgba.len(), 8 * 8 * 4);
            assert_eq!(image.rgba[3], if format == 5 { 255 } else { 0 });
        }
    }
}
#[test]
fn new_edits_discard_redo_and_revert_is_undoable() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Project::create(dir.path(), &s, Manifest::default()).unwrap();
    let path = "data/Studio/readout.bmg";
    p.edit(path, Bmg::new(&["First"]).write(), "first", &s)
        .unwrap();
    p.edit(path, Bmg::new(&["Second"]).write(), "second", &s)
        .unwrap();
    p.undo().unwrap();
    p.edit(path, Bmg::new(&["Third"]).write(), "third", &s)
        .unwrap();
    p.redo().unwrap();
    assert_eq!(p.history().len(), 2);
    assert_eq!(p.history()[1].title, "third");
    p.revert(path).unwrap();
    assert!(p.edits.is_empty());
    p.undo().unwrap();
    assert!(!p.edits.is_empty());
}

#[test]
fn deeply_nested_archive_edit_rebuilds_each_container() {
    let files = synthetic::files();
    let outer = Narc::new(std::collections::BTreeMap::from([(
        "sub/inner.carc".into(),
        files["data/Studio/demo.carc"].clone(),
    )]))
    .unwrap()
    .write();
    let session = RomSession::from_bytes(
        synthetic::rom_with(std::collections::BTreeMap::from([(
            "data/outer.narc".into(),
            outer,
        )])),
        PathBuf::new(),
    )
    .unwrap();
    let path = "data/outer.narc/sub/inner.carc/art/studio.nclr";
    let dir = tempfile::tempdir().unwrap();
    let mut project = Project::create(dir.path(), &session, Manifest::default()).unwrap();
    let mut colors = Nclr::parse(session.file(path).unwrap()).unwrap();
    colors.colors[0] ^= 31;
    let edited = colors.write();
    project
        .edit(path, edited.clone(), "nested colour", &session)
        .unwrap();
    let (zip, _) = build(&project, &session).unwrap();
    let rebuilt = patch_result(&zip, &session);
    let outer = Narc::parse(&rebuilt[0].1).unwrap();
    let inner = Narc::parse(&nitro_formats::decompress(&outer.entries[0].data).unwrap()).unwrap();
    assert_eq!(
        inner
            .entries
            .iter()
            .find(|e| e.name == "art/studio.nclr")
            .unwrap()
            .data,
        edited
    );
}
#[test]
fn unsupported_rom_warns_but_never_packs() {
    let mut b = synthetic::rom();
    b[12..16].copy_from_slice(b"TEST");
    let s = RomSession::from_bytes(b, PathBuf::new()).unwrap();
    assert!(!s.warnings.is_empty());
    let dir = tempfile::tempdir().unwrap();
    let p = Project::create(dir.path(), &s, Manifest::default()).unwrap();
    assert!(build(&p, &s).is_err());
}

#[test]
fn conflicting_container_edits_are_rejected_instead_of_losing_work() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Project::create(dir.path(), &s, Manifest::default()).unwrap();
    let leaf = "data/Studio/demo.carc/art/studio.nclr";
    let root = "data/Studio/demo.carc";
    let mut palette = Nclr::parse(s.file(leaf).unwrap()).unwrap();
    palette.colors[0] ^= 1;
    p.edit(leaf, palette.write(), "colour", &s).unwrap();
    assert!(p
        .edit(root, s.file(root).unwrap().to_vec(), "container", &s)
        .is_err());
    // The packer also checks edits loaded externally from a project folder.
    p.edits.insert(root.into(), s.file(root).unwrap().to_vec());
    assert!(build(&p, &s)
        .unwrap_err()
        .to_string()
        .contains("conflicting edits"));
}
#[test]
fn validation_visits_compressed_members_and_reports_bad_formats() {
    use toolbox_core::validation::{Check, Severity, Stage1Check};
    let palette = Nclr::new(&[0; 17], false, false).write();
    let archive = Narc::new(std::collections::BTreeMap::from([(
        "sub/invalid.nclr".into(),
        nitro_formats::compress(&palette),
    )]))
    .unwrap()
    .write();
    let issues = Stage1Check.check("outer.carc", &nitro_formats::compress(&archive));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].location, "outer.carc/sub/invalid.nclr");
    assert!(issues[0].message.contains("17 colours"));
    assert!(matches!(
        Stage1Check.check("bad.bmg", b"MESGbmg1")[0].severity,
        Severity::Error
    ));
    let text = Bmg::new(&[&"a".repeat(mkds_data::MAX_MESSAGE_UNITS + 1)]).write();
    assert!(!Stage1Check.check("long.bmg", &text).is_empty());
}
#[test]
fn edits_update_the_same_rendering_pipeline_used_for_png_export() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Project::create(dir.path(), &s, Manifest::default()).unwrap();
    let palette = "data/Studio/demo.carc/art/studio.nclr";
    let tiles = "data/Studio/demo.carc/art/studio.ncgr";
    let before =
        toolbox_core::preview::image(Some(&p), &s, tiles, Some(palette), None, 0, 0).unwrap();
    let mut colors = Nclr::parse(s.file(palette).unwrap()).unwrap();
    colors.colors[1] ^= 0x7fff;
    p.edit(palette, colors.write(), "colour", &s).unwrap();
    let after =
        toolbox_core::preview::image(Some(&p), &s, tiles, Some(palette), None, 0, 0).unwrap();
    assert_ne!(before.rgba, after.rgba);
    assert_eq!(
        &after.rgba[..4],
        &nitro_formats::graphics::bgr555_to_rgba(colors.colors[1])
    );
    assert!(toolbox_core::preview::png(&after)
        .unwrap()
        .starts_with(b"\x89PNG"));
    p.undo().unwrap();
    assert_eq!(
        toolbox_core::preview::image(Some(&p), &s, tiles, Some(palette), None, 0, 0).unwrap(),
        before
    );
}

#[test]
fn packaging_warnings_and_portable_target_restrictions_match_the_installer() {
    let mut seed = 12345u32;
    let original: Vec<_> = (0..70000)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect();
    let edited: Vec<_> = (0..70000)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect();
    let s = RomSession::from_bytes(
        synthetic::rom_with(std::collections::BTreeMap::from([
            ("data/original.bin".into(), original),
            ("data/blocked.exe".into(), vec![1; 100]),
        ])),
        PathBuf::new(),
    )
    .unwrap();
    let p = Project::in_memory(
        &s,
        Manifest {
            platforms: vec!["pc".into(), "ds".into()],
            ..Manifest::default()
        },
        std::collections::BTreeMap::from([("data/original.bin".into(), edited)]),
    )
    .unwrap();
    let (zip, warnings) = build(&p, &s).unwrap();
    assert!(warnings.iter().any(|w| w.contains("60%")));
    let mut z = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let json: serde_json::Value =
        serde_json::from_reader(z.by_name("manifest.json").unwrap()).unwrap();
    assert!(
        json.get("platforms").is_none(),
        "default PC packages remain compatible with older v1 launchers"
    );
    let manifest: PackageManifest = serde_json::from_value(json).unwrap();
    assert_eq!(manifest.info.platforms, ["pc"]);
    for path in ["data/blocked.exe", "data/ORIGINAL.bin/new.nclr", "data"] {
        let p = Project::in_memory(
            &s,
            Manifest::default(),
            std::collections::BTreeMap::from([(
                path.into(),
                Nclr::new(&[31; 16], false, false).write(),
            )]),
        )
        .unwrap();
        assert!(build(&p, &s).is_err(), "{path}");
    }
}

#[test]
fn rom_overlay_tables_remain_read_only_and_validate_file_ids() {
    let mut bytes = synthetic::rom();
    let table = bytes.len();
    bytes.extend_from_slice(&[0; 32]);
    let put = |b: &mut [u8], at: usize, n: u32| b[at..at + 4].copy_from_slice(&n.to_le_bytes());
    put(&mut bytes, 0x50, table as u32);
    put(&mut bytes, 0x54, 32);
    put(&mut bytes, table, 7);
    put(&mut bytes, table + 4, 0x02000000);
    put(&mut bytes, table + 8, 4096);
    put(&mut bytes, table + 24, 0);
    let rom = nitro_formats::rom::Rom::parse(&bytes).unwrap();
    assert_eq!(rom.arm9_overlays[0].id, 7);
    assert_eq!(rom.arm9_overlays[0].ram_address, 0x02000000);
    assert_eq!(rom.bytes(), bytes);
    put(&mut bytes, table + 24, 99999);
    assert!(nitro_formats::rom::Rom::parse(&bytes).is_err());
}
