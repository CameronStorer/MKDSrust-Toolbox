use nitro_formats::{blz, executable, model::Nsbmd, nkm::Nkm, rom::Rom};
use std::collections::BTreeMap;
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
    tables, track_wizard, wizard,
};
#[test]
fn backwards_lz_bounds_and_executable_modes() {
    let mut image = vec![0; 0x18000];
    for (i, v) in image.iter_mut().enumerate().take(0x4000) {
        *v = (i.wrapping_mul(17) % 251) as u8;
    }
    let p = 0x100;
    image[p + 28..p + 36].copy_from_slice(&[0x21, 0x06, 0xc0, 0xde, 0xde, 0xc0, 0x06, 0x21]);
    let compressed = blz::compress(&image, 0x4000).expect("repetitive suffix compresses");
    assert!(compressed.len() < image.len() / 2);
    assert_eq!(blz::decompress(&compressed).unwrap(), image);
    let stored = executable::encode_arm9(&image, 0x02000000, image.len(), false).unwrap();
    assert!(stored.len() < image.len());
    let decoded = executable::decode_arm9(&stored).unwrap();
    assert_eq!(
        u32::from_le_bytes(decoded[p + 20..p + 24].try_into().unwrap()),
        0x02000000 + stored.len() as u32
    );
    let plain = executable::encode_arm9(&decoded, 0x02000000, 1, false).unwrap();
    assert_eq!(&plain[p + 20..p + 24], &[0; 4]);
    assert_eq!(executable::decode_arm9(&plain).unwrap(), plain);
    for end in 0..compressed.len().min(100) {
        let _ = blz::decompress(&compressed[..end]);
    }
    assert!(blz::decompress(&[255; 8]).is_err());
}
#[test]
fn closed_routes_are_complete_and_bad_lines_rejected() {
    let line = [
        [0., 0., 0.],
        [1000., 0., 0.],
        [1000., 0., 1000.],
        [0., 0., 1000.],
    ];
    let n = track_wizard::generate_nkm(22, 3, &line, 100., 3).unwrap();
    assert!(track_wizard::validate(&n).is_empty());
    let n = Nkm::parse(&n.write().unwrap()).unwrap();
    assert_eq!(n.section("CPOI").unwrap().entries.len(), 64);
    assert_eq!(n.section("OBJI").unwrap().entries.len(), 9);
    assert!(track_wizard::generate_nkm(22, 0, &line, 100., 3).is_err());
    assert!(track_wizard::resample(&[[0.; 3]; 3], 64).is_err());
    let mut data = vec![0; 8];
    assert!(tables::write(&mut data, 100, 99, &[tables::Value::U32(1)]).is_err());
    assert!(tables::write(&mut data, 100, 100, &[tables::Value::Fx32(f32::NAN)]).is_err());
}
#[test]
fn synthetic_recipes_persist_as_one_command_and_reproduce_the_rom() {
    let temp = tempfile::tempdir().unwrap();
    let rom = temp.path().join("original.nds");
    std::fs::write(&rom, toolbox_core::synthetic::wizard_rom()).unwrap();
    let s = RomSession::open(&rom).unwrap();
    let root = temp.path().join("project");
    let mut p = Project::create(&root, &s, Manifest::default()).unwrap();
    let obj = temp.path().join("author.obj");
    std::fs::write(&obj,"v -1 0 -1\nv 1 0 -1\nv 1 0 1\nv -1 0 1\nv 0 2 0\nf 1 3 2\nf 1 4 3\nf 1 2 5\nf 2 3 5\nf 3 4 5\nf 4 1 5\n").unwrap();
    let source = obj.to_string_lossy().to_string();
    let wav = temp.path().join("tone.wav");
    std::fs::write(
        &wav,
        toolbox_core::audio::wav(&toolbox_core::audio::Pcm {
            rate: 16000,
            channels: vec![(0..1600)
                .map(|i| ((i as f32 / 9.).sin() * 6000.) as i16)
                .collect()],
        })
        .unwrap(),
    )
    .unwrap();
    let actions = vec![
        wizard::Action::Character {
            options: wizard::CharacterOptions {
                slot: 0,
                base: 1,
                source: source.clone(),
                hips: 0.,
                rotation: [0., 0., 0., 1.],
            },
        },
        wizard::Action::Kart {
            options: wizard::KartOptions {
                slot: 1,
                base: 2,
                source: source.clone(),
                tire: "S".into(),
                wheels: None,
                seat: [0.; 3],
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: 1.,
            },
        },
        wizard::Action::Track {
            options: track_wizard::TrackOptions {
                course: 20,
                source,
                line: vec![[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]],
                width: 150.,
                laps: 3,
                item_rows: 3,
            },
        },
        wizard::Action::Music {
            course: 22,
            source: wav.to_string_lossy().into(),
            options: toolbox_core::engine::SongOptions {
                sequence: 0,
                bank: 0,
                route: "auto".into(),
                rate: 16000,
                stereo: false,
                encoding: 2,
                loop_start: Some(0),
                loop_end: None,
            },
        },
        wizard::Action::Cup {
            slot: 0,
            courses: [22, 20, 22, 20],
        },
        wizard::Action::Name {
            path: "data/Main2D_us.carc/common.bmg".into(),
            index: 37,
            text: "Studio Cup".into(),
        },
    ];
    for action in actions {
        let before = p.edits.clone();
        let history = p.history().len();
        let report = wizard::apply(&mut p, &s, action).unwrap();
        assert!(!report.files.is_empty());
        assert_eq!(p.history().len(), history + 1);
        let after = p.edits.clone();
        p.undo().unwrap();
        assert_eq!(p.edits, before);
        p.redo().unwrap();
        assert_eq!(p.edits, after);
        let reopened = Project::open(&root, &s).unwrap();
        assert_eq!(reopened.edits, p.edits);
    }
    let (image, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    let patch = nitro_formats::bps::create(s.rom.bytes(), &image);
    assert_eq!(
        nitro_formats::bps::apply(s.rom.bytes(), &patch).unwrap(),
        image
    );
    let built = RomSession::from_bytes(image, s.path.clone()).unwrap();
    for (path, bytes) in &p.edits {
        assert_eq!(built.file(path).unwrap(), bytes, "{path}");
    }
    let model = "data/KartModelSub.carc/character/mario/P_MR.nsbmd";
    let m = Nsbmd::parse(built.file(model).unwrap()).unwrap();
    assert!(m.models[0]
        .meshes
        .iter()
        .all(|m| m.weights.iter().all(|w| *w == [1., 0., 0., 0.])));
    let n = Nkm::parse(
        built
            .file("data/Course/studio20D.carc/course_map.nkm")
            .unwrap(),
    )
    .unwrap();
    assert!(track_wizard::validate(&n).is_empty());
    assert!(toolbox_core::pack::build(&p, &s)
        .unwrap_err()
        .to_string()
        .contains("BPS"));
}
#[test]
fn owned_rom_wizards_export_and_reopen_without_writing_game_bytes() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM for Stage 5 local verification");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let original_hash = toolbox_core::session::hash(s.rom.bytes());
    let new_project = || Project::in_memory(&s, Manifest::default(), BTreeMap::new()).unwrap();
    let source = || Nsbmd::parse(&nitro_formats::synthetic::nsbmd()).unwrap();
    let mut p = new_project();
    let c = tables::catalog(Some(&p), &s).unwrap();
    assert_eq!(c.cups.len(), 8);
    assert!(c
        .courses
        .iter()
        .find(|c| c.id == 22)
        .unwrap()
        .sequence
        .is_some());
    let row = [c.cups[0][1], c.cups[0][0], c.cups[0][2], c.cups[0][3]];
    tables::set_cup(&mut p, &s, 0, row).unwrap();
    tables::assign_music(&mut p, &s, 22, 15).unwrap();
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    let r = Rom::parse(&bytes).unwrap();
    assert_eq!(r.arm9.start, s.rom.arm9.start);
    let arm = executable::decode_arm9(&bytes[r.arm9.clone()]).unwrap();
    assert_eq!(tables::word(&arm, mkds_data::CUP_TABLE).unwrap(), row[0]);
    assert_eq!(
        arm[(mkds_data::MUSIC_TABLE - mkds_data::ARM9_BASE + 11 * 4 + 1) as usize],
        15
    );
    println!(
        "cups and course music: executable export/readback passed ({} bytes)",
        bytes.len()
    );
    drop(bytes);
    let mut p = new_project();
    let report = wizard::character_from(
        &mut p,
        &s,
        &wizard::CharacterOptions {
            slot: 12,
            base: 0,
            source: String::new(),
            hips: 0.,
            rotation: [0., 0., 0., 1.],
        },
        source(),
    )
    .unwrap();
    for path in &report.files {
        if path.ends_with(".nsbmd") {
            Nsbmd::parse(p.bytes(&s, path).unwrap()).unwrap();
        }
    }
    let edits = p.edits.clone();
    p.undo().unwrap();
    assert!(p.edits.is_empty());
    p.redo().unwrap();
    assert_eq!(p.edits, edits);
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    let r = RomSession::from_bytes(bytes, s.path.clone()).unwrap();
    for path in &report.files {
        assert_eq!(r.file(path).unwrap(), p.bytes(&s, path).unwrap(), "{path}");
    }
    println!(
        "character: {} related files export/readback passed",
        report.files.len()
    );
    drop(r);
    let mut p = new_project();
    let report = wizard::kart_from(
        &mut p,
        &s,
        &wizard::KartOptions {
            slot: 1,
            base: 2,
            source: String::new(),
            tire: "M".into(),
            wheels: Some([[1., 1., 1.], [-1., 1., 1.], [1., 1., -1.], [-1., 1., -1.]]),
            seat: [0.; 3],
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: 1.,
        },
        source(),
    )
    .unwrap();
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    let r = RomSession::from_bytes(bytes, s.path.clone()).unwrap();
    for path in &report.files {
        assert_eq!(r.file(path).unwrap(), p.bytes(&s, path).unwrap(), "{path}");
    }
    println!(
        "kart: {} related files export/readback passed",
        report.files.len()
    );
    drop(r);
    let mut p = new_project();
    let report = track_wizard::track_from(
        &mut p,
        &s,
        &track_wizard::TrackOptions {
            course: 20,
            source: String::new(),
            line: vec![[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]],
            width: 100.,
            laps: 3,
            item_rows: 3,
        },
        source(),
    )
    .unwrap();
    let limits = track_wizard::file_budgets(&s).unwrap();
    for path in &report.files {
        track_wizard::check_file_size(path, p.bytes(&s, path).unwrap(), &limits).unwrap();
        if path.ends_with(".kcl") {
            assert!(p.bytes(&s, path).unwrap().len() <= nitro_formats::kcl::MAX_BYTES);
        }
    }
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    let r = RomSession::from_bytes(bytes, s.path.clone()).unwrap();
    for path in &report.files {
        assert_eq!(r.file(path).unwrap(), p.bytes(&s, path).unwrap(), "{path}");
    }
    println!(
        "track: {} related files export/readback passed",
        report.files.len()
    );
    assert_eq!(
        original_hash,
        toolbox_core::session::hash(&std::fs::read(&path).unwrap())
    );
}
