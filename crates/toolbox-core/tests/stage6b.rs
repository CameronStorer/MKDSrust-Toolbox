use nitro_formats::{
    archive::Narc,
    graphics::{Ncgr, Nclr, Nscr},
};
use std::collections::BTreeMap;
use toolbox_core::{associations, scene::CourseScene, session::RomSession, synthetic};

#[test]
fn pairing_merges_languages_and_checks_image_sizes_without_random_fallbacks() {
    let mut files = synthetic::files();
    let tiles = Ncgr::new(1, 1, 3, false, vec![1; 64]).unwrap().write();
    files.insert(
        "data/Scene/Race.carc".into(),
        Narc::new(BTreeMap::from([
            (
                "race_s_o.NCLR".into(),
                Nclr::new(&[0; 16], false, false).write(),
            ),
            ("race_s.NCER".into(), nitro_formats::synthetic::ncer()),
            ("bad.NCLR".into(), Nclr::new(&[0; 16], false, false).write()),
            ("local2.NSCR".into(), Nscr::new(8, 8, &[2]).unwrap().write()),
            ("local.NCGR".into(), tiles.clone()),
        ]))
        .unwrap()
        .write(),
    );
    files.insert(
        "data/Scene/Race_us.carc".into(),
        Narc::new(BTreeMap::from([
            ("race_s_o.NCGR".into(), tiles),
            (
                "bad.NCGR".into(),
                Ncgr::new(1, 1, 4, false, vec![17; 64]).unwrap().write(),
            ),
        ]))
        .unwrap()
        .write(),
    );
    let rom = synthetic::rom_with(files);
    let s = RomSession::from_bytes(rom, "/external/synthetic.nds".into()).unwrap();
    let p = s
        .pairings
        .iter()
        .find(|p| p.path.ends_with("Race_us.carc/race_s_o.NCGR"))
        .unwrap();
    assert_eq!(
        p.palette.as_deref(),
        Some("data/Scene/Race.carc/race_s_o.NCLR")
    );
    assert_eq!(p.cells.as_deref(), Some("data/Scene/Race.carc/race_s.NCER"));
    assert_eq!(
        associations::companion(&s.nodes, "data/Scene/Race.carc/race_s_o.NCLR", "ncgr").as_deref(),
        Some("data/Scene/Race_us.carc/race_s_o.NCGR")
    );
    assert!(s
        .pairings
        .iter()
        .find(|p| p.path.ends_with("bad.NCGR"))
        .unwrap()
        .palette
        .is_none());
    assert!(s
        .pairings
        .iter()
        .find(|p| p.path.ends_with("local.NCGR"))
        .unwrap()
        .screen
        .is_none());
    assert!(
        associations::companion(&s.nodes, "data/Scene/Race_us.carc/missing.NCGR", "nclr").is_none()
    );
}

#[test]
fn model_loads_only_its_own_texture_and_clips() {
    let s =
        RomSession::from_bytes(synthetic::wizard_rom(), "/external/synthetic.nds".into()).unwrap();
    let scene = CourseScene::load(
        None,
        &s,
        "data/KartModelSub.carc/character/mario/P_MR.nsbmd",
    )
    .unwrap();
    assert_eq!(scene.animations.len(), 4);
    assert!(scene.animations.iter().all(|a| a.name.starts_with("P_MR_")));
    assert!(scene.textures.is_some());
    let scene = CourseScene::load(
        None,
        &s,
        "data/KartModelMainA.carc/kart/shadow/sh_MR_a.nsbmd",
    )
    .unwrap();
    assert!(scene.textures.is_none());
    assert!(scene.animations.is_empty());
}

#[test]
fn related_pages_use_live_menu_files_voice_banks_and_wheel_offsets() {
    let s =
        RomSession::from_bytes(synthetic::wizard_rom(), "/external/synthetic.nds".into()).unwrap();
    let catalog = toolbox_core::tables::catalog(None, &s).unwrap();
    let pages = toolbox_core::related::pages(None, &s, &catalog, &[]);
    let mario = pages.iter().find(|p| p.id == "character:0").unwrap();
    assert!(mario
        .assets
        .iter()
        .find(|a| a.label == "Menu driver")
        .unwrap()
        .path
        .as_deref()
        .unwrap()
        .starts_with("data/KartModelMenu/"));
    assert!(mario
        .assets
        .iter()
        .find(|a| a.label == "Voice")
        .unwrap()
        .path
        .is_some());
    assert_eq!(mario.links.len(), 3);
    let kart = pages.iter().find(|p| p.id == "kart:1").unwrap();
    let body = kart.assets.iter().find(|a| a.label == "Race body").unwrap();
    assert_eq!(body.attachments.len(), 4);
    assert_eq!(body.attachments[0].position, [-1., 0., 1.]);
    assert!(body.attachments[0].path.ends_with("kart_tire_S.nsbmd"));
    let track = pages.iter().find(|p| p.id == "track:20").unwrap();
    assert!(track
        .assets
        .iter()
        .any(|a| a.mode == "routes" && a.path.is_some()));
    let voice = mario.assets.iter().find(|a| a.label == "Voice").unwrap();
    let wav = toolbox_core::engine::preview_audio(
        s.file(voice.path.as_ref().unwrap()).unwrap(),
        voice.audio_kind,
        voice.index,
        0,
        1,
    )
    .unwrap();
    assert!(wav.starts_with(b"RIFF"));
}

#[test]
fn member_packages_are_small_nested_hash_checked_and_combine_disjoint_members() {
    use std::io::{Cursor, Read};
    use toolbox_core::{
        archive_delta,
        pack::{self, Entry, PackageManifest},
        project::{Manifest, Project},
    };
    let mut seed = 71u32;
    let mut leaf: Vec<_> = (0..131072)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect();
    leaf[0] = 0;
    let inner = nitro_formats::compress(
        &Narc::new(BTreeMap::from([
            ("body.bin".into(), leaf.clone()),
            ("other.bin".into(), vec![8; 100]),
        ]))
        .unwrap()
        .write(),
    );
    let original = nitro_formats::compress(
        &Narc::new(BTreeMap::from([("sub/inner.carc".into(), inner)]))
            .unwrap()
            .write(),
    );
    let s = RomSession::from_bytes(
        synthetic::rom_with(BTreeMap::from([(
            "data/models.carc".into(),
            original.clone(),
        )])),
        "/external/synthetic.nds".into(),
    )
    .unwrap();
    leaf[11000] ^= 1;
    let p = Project::in_memory(
        &s,
        Manifest::default(),
        BTreeMap::from([(
            "data/models.carc/sub/inner.carc/body.bin".into(),
            leaf.clone(),
        )]),
    )
    .unwrap();
    let (bytes, warnings) = pack::build(&p, &s).unwrap();
    assert!(warnings.is_empty());
    assert!(
        bytes.len() < 2048,
        "package should contain only one tiny member patch: {}",
        bytes.len()
    );
    assert_eq!(pack::build(&p, &s).unwrap().0, bytes);
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert_eq!(zip.len(), 2);
    let manifest: PackageManifest =
        serde_json::from_reader(zip.by_name("manifest.json").unwrap()).unwrap();
    assert_eq!(manifest.format, 3);
    let Entry::Archive { archive: members } = &manifest.files["data/models.carc"] else {
        panic!("expected member patches")
    };
    assert_eq!(members.len(), 1);
    assert!(members.contains_key("sub/inner.carc/body.bin"));
    let payloads = members
        .values()
        .map(|m| {
            let mut b = vec![];
            zip.by_name(&m.patch).unwrap().read_to_end(&mut b).unwrap();
            (m.patch.clone(), b)
        })
        .collect();
    let result = archive_delta::apply(&original, members, &payloads).unwrap();
    assert_eq!(
        result,
        pack::rebuild_file("data/models.carc", &original, &p.edits).unwrap()
    );
    let other = BTreeMap::from([(
        "data/models.carc/sub/inner.carc/other.bin".into(),
        vec![9; 100],
    )]);
    let with_other = pack::rebuild_file("data/models.carc", &original, &other).unwrap();
    assert!(archive_delta::apply(&with_other, members, &payloads).is_ok());
    assert!(archive_delta::apply(&result, members, &payloads)
        .unwrap_err()
        .to_string()
        .contains("hash mismatch"));
    let mut invalid = members.clone();
    invalid.insert("../escape".into(), members.values().next().unwrap().clone());
    assert!(archive_delta::apply(&original, &invalid, &payloads).is_err());
    let mut missing = members.clone();
    missing.insert(
        "absent.bin".into(),
        members.values().next().unwrap().clone(),
    );
    assert!(archive_delta::apply(&original, &missing, &payloads).is_err());
    let legacy = pack::build_legacy(&p, &s).unwrap().0;
    let mut legacy = zip::ZipArchive::new(Cursor::new(legacy)).unwrap();
    let legacy: PackageManifest =
        serde_json::from_reader(legacy.by_name("manifest.json").unwrap()).unwrap();
    assert_eq!(legacy.format, 1);
}

#[test]
fn owned_rom_pairs_pages_and_menu_model_member_patch_stay_in_memory() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("skipped: MKDS_ROM is unset");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let catalog = toolbox_core::tables::catalog(None, &s).unwrap();
    let pages = toolbox_core::related::pages(None, &s, &catalog, &[]);
    assert_eq!(pages.iter().filter(|p| p.kind == "Character").count(), 13);
    assert!(pages.iter().filter(|p| p.kind == "Character").all(|p| p
        .assets
        .iter()
        .find(|a| a.label == "Voice")
        .unwrap()
        .path
        .is_some()));
    let root = "data/KartModelMenu.carc";
    let path = "data/KartModelMenu.carc/character/mario/P_MR.nsbmd";
    let mut model = nitro_formats::model::Nsbmd::parse(s.file(path).unwrap()).unwrap();
    model.models[0].meshes[0].positions[0][0] += 0.001;
    let target = model.write().unwrap();
    let edits = BTreeMap::from([(path.into(), target)]);
    let rebuilt = toolbox_core::pack::rebuild_file(root, s.file(root).unwrap(), &edits).unwrap();
    let mut payloads = BTreeMap::new();
    let members = toolbox_core::archive_delta::make(
        root,
        s.file(root).unwrap(),
        &rebuilt,
        &mut payloads,
        &mut vec![],
    )
    .unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(
        toolbox_core::archive_delta::apply(s.file(root).unwrap(), &members, &payloads).unwrap(),
        rebuilt
    );
    eprintln!(
        "menu member patch: {} bytes; compressed archive {} bytes (in memory only)",
        payloads.values().map(Vec::len).sum::<usize>(),
        s.file(root).unwrap().len()
    );
}
