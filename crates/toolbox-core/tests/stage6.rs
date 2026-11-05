use toolbox_core::{
    audio, game,
    project::{Manifest, Project},
    session::RomSession,
    synthetic,
};

#[test]
fn friendly_roles_and_save_as_preserve_patches() {
    let temp = tempfile::tempdir().unwrap();
    let rom = temp.path().join("original.nds");
    std::fs::write(&rom, synthetic::wizard_rom()).unwrap();
    let s = RomSession::open(&rom).unwrap();
    let catalog = toolbox_core::tables::catalog(None, &s).unwrap();
    let entries = game::entries(&s.nodes, &catalog, &[]);
    assert!(entries
        .iter()
        .any(|e| e.label == "Mario race model" && e.location.contains("race")));
    assert!(entries
        .iter()
        .any(|e| e.label == "Mario far-away model" && e.location.contains("distance")));
    let mut p = Project::create(&temp.path().join("project"), &s, Manifest::default()).unwrap();
    toolbox_core::tables::set_cup(&mut p, &s, 0, [20, 20, 20, 20]).unwrap();
    let edits = p.edits.clone();
    let dest = temp.path().join("copy");
    p.save_as(&dest, &s).unwrap();
    assert_eq!(Project::open(&dest, &s).unwrap().edits, edits);
    assert!(!dest.join("original.nds").exists());
    assert!(p.save_as(&dest, &s).is_err());
}

#[test]
fn whole_song_uses_its_bank_and_stops_at_first_loop() {
    let sdat = nitro_formats::audio::Sdat::parse(&synthetic::sound_fixture()).unwrap();
    let pcm = audio::preview_sequence(&sdat, 0, 0.).unwrap();
    assert!(pcm.channels.iter().flatten().any(|&v| v != 0));
    assert!(pcm.channels[0].len() < pcm.rate as usize * 10);
    let events = [60, 127, 48, 0x94, 0, 0, 0];
    let timeline = nitro_formats::audio::timeline_once(&events, 48000).unwrap();
    assert_eq!(timeline.notes.len(), 1);
    assert_eq!(timeline.ticks, 48);
}

#[test]
fn swapping_embedded_texture_colours_preserves_geometry_and_undo() {
    use toolbox_core::service::{Request, Service};
    let s =
        RomSession::from_bytes(synthetic::wizard_rom(), "/external/synthetic.nds".into()).unwrap();
    let path = "data/MainRace.carc/Item/it_studio.nsbmd";
    let original = s.file(path).unwrap().to_vec();
    let p = Project::in_memory(&s, Manifest::default(), Default::default()).unwrap();
    let mut service = Service {
        session: Some(s),
        project: Some(p),
        persist_settings: false,
        ..Service::default()
    };
    service
        .dispatch(Request::TexturePalette {
            path: path.into(),
            texture: 0,
            bank: 0,
            indices: vec![1],
            colors: vec![31],
        })
        .unwrap();
    let bytes = service
        .project
        .as_ref()
        .unwrap()
        .bytes(service.session.as_ref().unwrap(), path)
        .unwrap();
    let before = nitro_formats::model::Nsbmd::parse(&original).unwrap();
    let after = nitro_formats::model::Nsbmd::parse(bytes).unwrap();
    assert_eq!(
        before.models[0].meshes[0].positions,
        after.models[0].meshes[0].positions
    );
    assert_eq!(after.textures.unwrap().palettes[0].colors[1], 31);
    service.dispatch(Request::Undo).unwrap();
    assert_eq!(
        service
            .project
            .as_ref()
            .unwrap()
            .bytes(service.session.as_ref().unwrap(), path)
            .unwrap(),
        original
    );
}

#[test]
fn owned_rom_song_preview_is_audible() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("skipped: MKDS_ROM is unset");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let sdat =
        nitro_formats::audio::Sdat::parse(s.file("data/Sound/sound_data.sdat").unwrap()).unwrap();
    for index in [1, 6, 16, 43, 66, 74, 75] {
        let pcm = audio::preview_sequence(&sdat, index, 2.)
            .unwrap_or_else(|e| panic!("sequence {index}: {e}"));
        assert!(
            pcm.channels.iter().flatten().any(|&v| v != 0),
            "sequence {index}"
        );
        eprintln!(
            "audible sequence {index}: {} frames (in memory only)",
            pcm.channels[0].len()
        );
    }
}

/// Six subdivided cuboids form a head, torso, arms and legs. A floor supplies
/// a continuous driving surface for the same fixture's track conversion.
fn dense_humanoid_glb() -> Vec<u8> {
    use nitro_formats::{
        math,
        model::{Bone, Mesh, Model, Nsbmd},
    };
    let mut mesh = Mesh::default();
    for (part, (center, size)) in [
        ([0., 4., 0.], [2., 2., 1.]),
        ([0., 6., 0.], [1.4, 1.4, 1.4]),
        ([-1.6, 4., 0.], [1., 2., 1.]),
        ([1.6, 4., 0.], [1., 2., 1.]),
        ([-0.6, 1.5, 0.], [0.9, 3., 1.]),
        ([0.6, 1.5, 0.], [0.9, 3., 1.]),
    ]
    .into_iter()
    .enumerate()
    {
        for axis in 0..3 {
            for sign in [-1., 1.] {
                let u = (axis + 1) % 3;
                let v = (axis + 2) % 3;
                let base = mesh.positions.len() as u32;
                for y in 0..=21 {
                    for x in 0..=21 {
                        let mut p = center;
                        p[axis] += size[axis] * sign * 0.5;
                        p[u] += size[u] * (x as f32 / 21. - 0.5);
                        p[v] += size[v] * (y as f32 / 21. - 0.5);
                        mesh.positions.push(p);
                        let mut n = [0.; 3];
                        n[axis] = sign;
                        mesh.normals.push(n);
                        mesh.uvs.push([x as f32 / 21., y as f32 / 21.]);
                        mesh.colors.push([0.7, 0.75, 0.6, 1.]);
                        mesh.joints.push([(part * 35) as u16, 0, 0, 0]);
                        mesh.weights.push([1., 0., 0., 0.]);
                    }
                }
                for y in 0..21u32 {
                    for x in 0..21u32 {
                        let a = base + y * 22 + x;
                        let mut t = [a, a + 1, a + 22, a + 1, a + 23, a + 22];
                        if sign < 0. {
                            t.swap(1, 2);
                            t.swap(4, 5);
                        }
                        mesh.indices.extend(t);
                    }
                }
            }
        }
    }
    let base = mesh.positions.len() as u32;
    mesh.positions
        .extend([[-3., 0., -3.], [3., 0., -3.], [3., 0., 3.], [-3., 0., 3.]]);
    mesh.normals.extend([[0., 1., 0.]; 4]);
    mesh.uvs.extend([[0., 0.]; 4]);
    mesh.colors.extend([[0.7; 4]; 4]);
    mesh.joints.extend([[0; 4]; 4]);
    mesh.weights.extend([[1., 0., 0., 0.]; 4]);
    mesh.indices
        .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    assert_eq!(mesh.indices.len() / 3, 31754);
    let bones = (0..220)
        .map(|i| Bone {
            name: format!("joint{i}"),
            parent: None,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        })
        .collect();
    let model = Nsbmd {
        models: vec![Model {
            name: "humanoid".into(),
            bones,
            matrix_count: 220,
            meshes: vec![mesh],
            materials: vec![toolbox_core::model_import::default_material(
                "surface".into(),
            )],
            warnings: vec![],
        }],
        textures: None,
    };
    let glb = toolbox_core::model::glb(Some(&model), None, &[], None).unwrap();
    let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let mut json: serde_json::Value = serde_json::from_slice(&glb[20..20 + len]).unwrap();
    for i in 0..110 {
        let node = json["nodes"][i].as_object_mut().unwrap();
        node.remove("translation");
        node.remove("rotation");
        node.remove("scale");
        node.insert(
            "matrix".into(),
            serde_json::json!(math::trs([0., 10., 0.], [0., 0., 0., 1.], [1.; 3])),
        );
    }
    let mut json = serde_json::to_vec(&json).unwrap();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend(((20 + json.len() + glb.len() - 20 - len) as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(json);
    out.extend(&glb[20 + len..]);
    out
}

#[test]
fn dense_matrix_skinned_download_runs_through_all_three_wizards() {
    use toolbox_core::wizard::{self, Action, CharacterOptions, KartOptions};
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("humanoid.glb");
    std::fs::write(&file, dense_humanoid_glb()).unwrap();
    let source = file.to_string_lossy().to_string();
    let imported = toolbox_core::model_import::import_static(&file).unwrap();
    assert_eq!(imported.models[0].bones.len(), 1);
    assert!(toolbox_core::model_import::import(&file).is_err()); // mismatched rest/bind is intentional
    let s =
        RomSession::from_bytes(synthetic::wizard_rom(), temp.path().join("synthetic.nds")).unwrap();
    for action in [
        Action::Character {
            options: CharacterOptions {
                slot: 0,
                base: 1,
                source: source.clone(),
                hips: 0.,
                rotation: [0., 0., 0., 1.],
            },
        },
        Action::Kart {
            options: KartOptions {
                slot: 1,
                base: 2,
                source: source.clone(),
                tire: "M".into(),
                wheels: None,
                seat: [0.; 3],
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: 1.,
            },
        },
        Action::Track {
            options: toolbox_core::track_wizard::TrackOptions {
                course: 20,
                source,
                line: vec![[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]],
                width: 100.,
                laps: 3,
                item_rows: 3,
            },
        },
    ] {
        let mut project = Project::in_memory(&s, Manifest::default(), Default::default()).unwrap();
        let start = std::time::Instant::now();
        let report = wizard::apply(&mut project, &s, action).unwrap();
        eprintln!("{} stress wizard in {:?}", report.polygons, start.elapsed());
        assert!(start.elapsed().as_secs_f32() < 30.);
        assert!(!report.files.is_empty());
        if let Some(path) = report
            .files
            .iter()
            .find(|p| p.ends_with("/Map2D/local.NCGR"))
        {
            if let Some(global) = report
                .files
                .iter()
                .find(|p| p.ends_with("/Map2D/global.NCGR"))
            {
                let image = toolbox_core::preview::image(
                    Some(&project),
                    &s,
                    global,
                    Some(&global.replace("NCGR", "NCLR")),
                    Some(&global.replace("global.NCGR", "global1.NSCR")),
                    0,
                    0,
                )
                .unwrap();
                assert!(image.rgba.chunks_exact(4).any(|p| p[3] != 0 && p[0] > 0));
            }
            let path = path.replace("Map2D/local.NCGR", "course_model.nsbtx");
            let textures =
                nitro_formats::texture::Nsbtx::parse(project.bytes(&s, &path).unwrap()).unwrap();
            assert!(!textures.textures.is_empty());
            assert!(textures
                .decode(0, 0)
                .unwrap()
                .rgba
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|&v| v > 0)));
        }
    }
}
