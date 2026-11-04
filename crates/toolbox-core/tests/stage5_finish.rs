//! Authored inputs only; optional retail tests keep every rebuilt image in RAM.
use nitro_formats::{
    audio::{Sdat, Ssar, Swar},
    graphics::Image,
    model::Nsbmd,
    nkm::Nkm,
};
use std::collections::BTreeMap;
use toolbox_core::{
    engine, graphic,
    project::{Manifest, Project},
    session::RomSession,
    track_wizard, wizard,
};

fn project(s: &RomSession) -> Project {
    Project::in_memory(s, Manifest::default(), BTreeMap::new()).unwrap()
}
fn readback(p: &Project, s: &RomSession) {
    let (bytes, _, _) = toolbox_core::export::build_rom(p, s).unwrap();
    let built = RomSession::from_bytes(bytes, s.path.clone()).unwrap();
    for (path, bytes) in &p.edits {
        if path.ends_with(".uncompressed") {
            continue;
        }
        assert_eq!(built.file(path).unwrap(), bytes, "{path}");
    }
}
fn image(width: u32, height: u32) -> Image {
    Image {
        width,
        height,
        rgba: (0..width * height)
            .flat_map(|i| [((i * 17) % 256) as u8, 120, 230, 255])
            .collect(),
    }
}
fn author_wav(path: &std::path::Path) {
    std::fs::write(
        path,
        toolbox_core::audio::wav(&toolbox_core::audio::Pcm {
            rate: 16000,
            channels: vec![(0..800)
                .map(|i| ((i as f32 / 13.).sin() * 8000.) as i16)
                .collect()],
        })
        .unwrap(),
    )
    .unwrap();
}
fn art_audio_names(s: &RomSession, item: &str, sequence: u32) {
    let temp = tempfile::tempdir().unwrap();
    let wav = temp.path().join("author.wav");
    author_wav(&wav);
    let png = temp.path().join("author.png");
    std::fs::write(&png, toolbox_core::preview::png(&image(32, 32)).unwrap()).unwrap();
    let mut p = project(s);
    let report = wizard::apply(
        &mut p,
        s,
        wizard::Action::Music {
            course: 22,
            source: wav.to_string_lossy().into(),
            options: engine::SongOptions {
                sequence,
                bank: 0,
                route: "sampled".into(),
                rate: 16000,
                stereo: false,
                encoding: 2,
                loop_start: Some(0),
                loop_end: None,
            },
        },
    )
    .unwrap();
    assert!(report.files.iter().any(|p| p.ends_with(".sdat")));
    let music = Sdat::parse(p.bytes(s, "data/Sound/sound_data.sdat").unwrap()).unwrap();
    music.validate_files().unwrap();
    assert_eq!(tables_sequence(&p, s, 22), sequence);
    readback(&p, s);
    let mut p = project(s);
    let original = Sdat::parse(s.file("data/Sound/sound_data.sdat").unwrap()).unwrap();
    let original_wave = Swar::parse(&original.files[original.resource_file(3, 0).unwrap()])
        .unwrap()
        .waves[0]
        .clone();
    engine::replace_effect(
        &mut p,
        s,
        "data/Sound/sound_data.sdat",
        &wav,
        engine::EffectOptions {
            archive: 0,
            sample: 0,
            rate: 16000,
            encoding: 2,
        },
    )
    .unwrap();
    let sound = Sdat::parse(p.bytes(s, "data/Sound/sound_data.sdat").unwrap()).unwrap();
    sound.validate_files().unwrap();
    let replacement = Swar::parse(&sound.files[sound.resource_file(3, 0).unwrap()])
        .unwrap()
        .waves[0]
        .clone();
    assert!(replacement.data.len() <= original_wave.data.len());
    assert_ne!(replacement, original_wave);
    readback(&p, s);
    let mut p = project(s);
    wizard::apply(
        &mut p,
        s,
        wizard::Action::Icon {
            tiles: "data/Scene/Race_us.carc/race_s_o.NCGR".into(),
            palette: "data/Scene/Race.carc/race_s_o.NCLR".into(),
            cells: "data/Scene/Race.carc/race_s.NCER".into(),
            index: 0,
            source: png.to_string_lossy().into(),
        },
    )
    .unwrap();
    assert_ne!(
        p.bytes(s, "data/Scene/Race_us.carc/race_s_o.NCGR").unwrap(),
        s.file("data/Scene/Race_us.carc/race_s_o.NCGR").unwrap()
    );
    readback(&p, s);
    let mut p = project(s);
    let model = Nsbmd::parse(s.file(item).unwrap()).unwrap();
    let texture = &model.textures.as_ref().unwrap().textures[0];
    std::fs::write(
        &png,
        toolbox_core::preview::png(&image(texture.width, texture.height)).unwrap(),
    )
    .unwrap();
    let draft = graphic::import(&p, s, item, &png, None, 0, 0, 3, true).unwrap();
    p.edit_many(draft.edits, "Retexture item", s).unwrap();
    let result = Nsbmd::parse(p.bytes(s, item).unwrap()).unwrap();
    assert_eq!(
        result.models[0].meshes[0].positions,
        model.models[0].meshes[0].positions
    );
    assert_ne!(
        result.textures.unwrap().write(),
        model.textures.unwrap().write()
    );
    readback(&p, s);
    let mut p = project(s);
    wizard::apply(
        &mut p,
        s,
        wizard::Action::Name {
            path: "data/Main2D_us.carc/common.bmg".into(),
            index: 37,
            text: "Studio Cup".into(),
        },
    )
    .unwrap();
    assert!(toolbox_core::names::list(Some(&p), s)
        .unwrap()
        .iter()
        .any(|n| n.index == 37 && n.text == "Studio Cup"));
    readback(&p, s);
}
fn tables_sequence(p: &Project, s: &RomSession, course: u32) -> u32 {
    toolbox_core::tables::catalog(Some(p), s)
        .unwrap()
        .courses
        .iter()
        .find(|c| c.id == course)
        .unwrap()
        .sequence
        .unwrap()
}
#[test]
fn every_art_audio_and_names_recipe_exports_with_authored_inputs() {
    let s = RomSession::from_bytes(
        toolbox_core::synthetic::wizard_rom(),
        "synthetic.nds".into(),
    )
    .unwrap();
    art_audio_names(&s, "data/MainRace.carc/Item/it_studio.nsbmd", 0);
    let mut p = project(&s);
    wizard::apply(
        &mut p,
        &s,
        wizard::Action::KartStats {
            slot: 1,
            speed: 8.,
            acceleration: 0.08,
        },
    )
    .unwrap();
    assert_eq!(
        toolbox_core::tables::catalog(Some(&p), &s).unwrap().karts[1].speed,
        8.
    );
    readback(&p, &s);
    let mut sound = Sdat::parse(s.file("data/Sound/sound_data.sdat").unwrap()).unwrap();
    let file = sound.resource_file(1, 2).unwrap();
    let before = Ssar::parse(&sound.files[file]).unwrap();
    wizard::copy_voice(&mut sound, 12, 0).unwrap();
    let after = Ssar::parse(&sound.files[file]).unwrap();
    assert_eq!(after.entries[168].offset, before.entries[0].offset);
    assert_eq!(after.entries[168].bank, before.entries[168].bank);
    assert_eq!(after.entries[..168], before.entries[..168]);
    let mut p = project(&s);
    let path = "data/KartModelMainA.carc/kartoffsetdata.bin";
    let seats = s.file(path).unwrap()[224 + 68..448].to_vec();
    wizard::apply(
        &mut p,
        &s,
        wizard::Action::SetWheels {
            slot: 1,
            wheels: [[4., 2., 6.], [-4., 2., 6.], [5., 2., -4.], [-5., 2., -4.]],
        },
    )
    .unwrap();
    assert_eq!(&p.bytes(&s, path).unwrap()[224 + 68..448], seats);
    assert_eq!(
        nitro_formats::nkm::vec3(p.bytes(&s, path).unwrap(), 224 + 20).unwrap(),
        [4., 2., 6.]
    );
    let path = "data/Course/studio20.carc/course_map.nkm";
    let count = Nkm::parse(p.bytes(&s, path).unwrap())
        .unwrap()
        .section("OBJI")
        .unwrap()
        .entries
        .len();
    wizard::apply(
        &mut p,
        &s,
        wizard::Action::PlaceObject {
            path: path.into(),
            donor: None,
            index: 0,
            position: [100., 12., 100.],
        },
    )
    .unwrap();
    assert_eq!(
        Nkm::parse(p.bytes(&s, path).unwrap())
            .unwrap()
            .section("OBJI")
            .unwrap()
            .entries
            .len(),
        count + 1
    );
    wizard::apply(
        &mut p,
        &s,
        wizard::Action::Portrait {
            model: "data/KartModelSub.carc/character/mario/P_MR.nsbmd".into(),
            tiles: "data/Main2D_us.carc/player_character_L.nce.ncgr".into(),
            palette: "data/Main2D_us.carc/player_character_L_o.NCLR".into(),
            cells: "data/Main2D_us.carc/player_character_L.nce.ncer".into(),
            index: 0,
        },
    )
    .unwrap();
    readback(&p, &s);
    let mut offsets = p
        .bytes(&s, "data/KartModelMainA.carc/kartoffsetdata.bin")
        .unwrap()
        .to_vec();
    toolbox_core::tables::write(
        &mut offsets,
        0,
        68,
        &[
            toolbox_core::tables::Value::Fx32(0.1),
            toolbox_core::tables::Value::Fx32(0.2),
            toolbox_core::tables::Value::Fx32(-0.1),
        ],
    )
    .unwrap();
    p.edit(
        "data/KartModelMainA.carc/kartoffsetdata.bin",
        offsets,
        "Seat placement",
        &s,
    )
    .unwrap();
    let service = toolbox_core::service::Service {
        session: Some(s),
        project: Some(p),
        persist_settings: false,
        ..Default::default()
    };
    let glb = service
        .binary(toolbox_core::service::BinaryRequest {
            path: "data/KartModelSub.carc/character/mario/P_MR.nsbmd".into(),
            mode: "seated-reference".into(),
            start: 0,
            length: 0,
            palette: None,
            screen: None,
            texture: 0,
            bank: 0,
        })
        .unwrap();
    assert_eq!(&glb[..4], b"glTF");
    let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + length]).unwrap();
    assert!((json["nodes"][0]["translation"][1].as_f64().unwrap() - 0.2).abs() < 0.001);
}
#[test]
fn repairs_preserve_authored_objects_and_budgets_refuse_growth() {
    let line = [
        [0., 0., 0.],
        [1000., 0., 0.],
        [1000., 0., 1000.],
        [0., 0., 1000.],
    ];
    let mut n = track_wizard::generate_nkm(20, 3, &line, 150., 3).unwrap();
    let objects = n.section("OBJI").unwrap().entries.clone();
    n.sections.retain(|s| s.kind != "KTPJ" && s.kind != "CAME");
    n.sections
        .iter_mut()
        .find(|s| s.kind == "CPOI")
        .unwrap()
        .entries
        .truncate(4);
    n.sections
        .iter_mut()
        .find(|s| s.kind == "EPAT")
        .unwrap()
        .entries[0][4] = 255;
    assert!(!track_wizard::validate(&n).is_empty());
    track_wizard::repair(&mut n).unwrap();
    assert!(track_wizard::validate(&Nkm::parse(&n.write().unwrap()).unwrap()).is_empty());
    assert_eq!(objects, n.section("OBJI").unwrap().entries);
    let limits = BTreeMap::from([("course_model.nsbmd".into(), 100)]);
    assert!(track_wizard::check_file_size(
        "data/Course/a.carc/course_model.nsbmd",
        &[0; 101],
        &limits
    )
    .unwrap_err()
    .to_string()
    .contains("largest retail"));
    let model = Nsbmd::parse(&nitro_formats::synthetic::nsbmd()).unwrap();
    let portrait = toolbox_core::icon::portrait(&model.models[0]).unwrap();
    assert!(portrait.rgba.chunks_exact(4).any(|p| p[3] == 255));
    assert!(portrait.rgba.chunks_exact(4).any(|p| p[3] == 0));
    let s = RomSession::from_bytes(
        toolbox_core::synthetic::wizard_rom(),
        "synthetic.nds".into(),
    )
    .unwrap();
    let mut p = project(&s);
    let path = "data/Course/studio20.carc/course_collision.kcl";
    let mut oversized = s.file(path).unwrap().to_vec();
    oversized.resize(nitro_formats::kcl::MAX_BYTES + 1, 0);
    p.edit(path, oversized.clone(), "Legacy large tree", &s)
        .unwrap();
    assert!(toolbox_core::validation::validate(&p, &s)
        .iter()
        .any(|i| i.fix.as_deref() == Some("collision-budget")));
    wizard::apply(
        &mut p,
        &s,
        wizard::Action::CompactCollision { path: path.into() },
    )
    .unwrap();
    assert!(p.bytes(&s, path).unwrap().len() < nitro_formats::kcl::WARN_BYTES);
    p.undo().unwrap();
    assert_eq!(p.bytes(&s, path).unwrap(), oversized);
}
#[test]
fn owned_rom_art_audio_and_names_read_back_in_memory() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: MKDS_ROM not set");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let hash = toolbox_core::session::hash(s.rom.bytes());
    let item = s
        .nodes
        .iter()
        .find(|n| n.path.starts_with("data/MainRace.carc/Item/it_") && n.format == "nsbmd")
        .unwrap()
        .path
        .clone();
    art_audio_names(&s, &item, 16);
    let overlay = &s.rom.arm9_overlays[0];
    let mut p = project(&s);
    let overlay_path = nitro_formats::executable::overlay_path(overlay.id);
    let word = u32::from_le_bytes(s.file(&overlay_path).unwrap()[..4].try_into().unwrap()) ^ 1;
    wizard::apply(
        &mut p,
        &s,
        wizard::Action::Table {
            overlay: Some(overlay.id),
            address: overlay.ram_address,
            values: vec![toolbox_core::tables::Value::U32(word)],
            uncompressed: true,
        },
    )
    .unwrap();
    readback(&p, &s);
    let (bytes, _, _) = toolbox_core::export::build_rom(&p, &s).unwrap();
    let rom = nitro_formats::rom::Rom::parse(&bytes).unwrap();
    let rebuilt = &rom.arm9_overlays[0];
    assert_eq!(rebuilt.flags & 0x0300_0000, 0);
    assert_eq!(
        rom.file_by_id(rebuilt.file_id).unwrap(),
        p.bytes(&s, &overlay_path).unwrap()
    );
    assert_eq!(
        hash,
        toolbox_core::session::hash(&std::fs::read(path).unwrap())
    );
}
