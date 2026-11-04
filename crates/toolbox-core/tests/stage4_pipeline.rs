use nitro_formats::{audio::Sdat, bps, model::Nsbmd};
use toolbox_core::{
    engine::{ModelAction, SongOptions},
    project::{Manifest, Project},
    service::{BinaryRequest, Request, Service},
    session::RomSession,
    synthetic,
};
#[test]
fn every_engine_persists_undoes_repacks_and_exports_for_ds() {
    let dir = tempfile::tempdir().unwrap();
    let rom = dir.path().join("synthetic.nds");
    std::fs::write(&rom, synthetic::rom()).unwrap();
    let session = RomSession::open(&rom).unwrap();
    let project =
        Project::create(&dir.path().join("project"), &session, Manifest::default()).unwrap();
    let mut service = Service {
        session: Some(session),
        project: Some(project),
        persist_settings: false,
        ..Service::default()
    };
    let model = "data/Course/studio.carc/course_model.nsbmd";
    let kcl = "data/Course/studio.carc/course_collision.kcl";
    let sound = "data/Studio/demo.carc/audio/sound_data.sdat";
    let kcl = service
        .session
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.format == "kcl")
        .map(|n| n.path.clone())
        .unwrap_or_else(|| kcl.into());
    let source = dir.path().join("author.obj");
    std::fs::write(&source, "v 0 0 0\nv 100 0 0\nv 0 0 100\nf 1 3 2").unwrap();
    service
        .dispatch(Request::Model {
            path: model.into(),
            action: ModelAction::Import {
                source: source.to_string_lossy().into(),
            },
        })
        .unwrap();
    service
        .dispatch(Request::GenerateCollision {
            model: model.into(),
            path: kcl.clone(),
        })
        .unwrap();
    service
        .dispatch(Request::PaintCollision {
            path: kcl.clone(),
            triangles: vec![0],
            attribute: mkds_data::collision_attribute(3, false, true),
        })
        .unwrap();
    service.dispatch(Request::Undo).unwrap();
    service.dispatch(Request::Redo).unwrap();
    let source = dir.path().join("author.wav");
    let pcm = toolbox_core::audio::Pcm {
        rate: 16000,
        channels: vec![(0..16000)
            .map(|i| ((i as f32 / 30.).sin() * 10000.) as i16)
            .collect()],
    };
    std::fs::write(&source, toolbox_core::audio::wav(&pcm).unwrap()).unwrap();
    service
        .dispatch(Request::Select { path: sound.into() })
        .unwrap();
    let snapshot = service
        .dispatch(Request::ImportSong {
            path: sound.into(),
            source: source.to_string_lossy().into(),
            options: SongOptions {
                sequence: 0,
                bank: 0,
                route: "auto".into(),
                rate: 8000,
                stereo: false,
                encoding: 2,
                loop_start: Some(0),
                loop_end: None,
            },
        })
        .unwrap();
    assert_eq!(snapshot.song_report.unwrap().route, "sampled");
    assert_eq!(
        snapshot
            .audio_resources
            .iter()
            .filter(|r| r.kind == 2)
            .count(),
        2
    );
    let wav = service
        .binary(BinaryRequest {
            path: sound.into(),
            mode: "audio-wav".into(),
            texture: 0,
            bank: 0,
            start: 0,
            length: 1,
            palette: None,
            screen: None,
        })
        .unwrap();
    assert!(wav.starts_with(b"RIFF"));
    let p = service.project.as_ref().unwrap();
    let s = service.session.as_ref().unwrap();
    let issues = toolbox_core::validation::validate(p, s);
    assert!(issues.is_empty(), "{issues:?}");
    let reopened = Project::open(&p.root, s).unwrap();
    let m = Nsbmd::parse(&toolbox_core::preview::plain(reopened.bytes(s, model).unwrap()).unwrap())
        .unwrap();
    assert_eq!(m.models[0].meshes[0].indices.len(), 3);
    let sdat = Sdat::parse(reopened.bytes(s, sound).unwrap()).unwrap();
    assert_eq!(sdat.bank_for_sequence(0).unwrap(), 1);
    let (_, report) = toolbox_core::pack::build(&reopened, s).unwrap();
    assert!(report.is_empty());
    let out = dir.path().join("author.bps");
    service
        .dispatch(Request::ExportDs {
            rom: None,
            bps: Some(out.to_string_lossy().into()),
        })
        .unwrap();
    let rebuilt = bps::apply(&std::fs::read(&rom).unwrap(), &std::fs::read(out).unwrap()).unwrap();
    let changed = RomSession::from_bytes(rebuilt, rom.clone()).unwrap();
    let k = nitro_formats::kcl::Kcl::parse(changed.file(&kcl).unwrap()).unwrap();
    assert_eq!(mkds_data::collision_type(k.prisms[0].attribute), 3);
    assert!(std::fs::read(rom).unwrap() == synthetic::rom());
}
