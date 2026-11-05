use nitro_formats::{
    archive::Narc,
    model::{Bone, Mesh, Nsbmd},
};
use std::collections::BTreeMap;
use toolbox_core::{
    engine::{MaterialAction, ModelAction},
    linked,
    mesh_edit::{self, Operation, Selection},
    model_import, preview,
    project::{Manifest, Project},
    scene::CourseScene,
    session::RomSession,
    synthetic,
};

fn quad() -> Nsbmd {
    model_import::obj("v 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nf 1 2 3 4\n").unwrap()
}
fn faces(ids: &[u32]) -> Selection {
    Selection {
        mode: "face".into(),
        vertices: vec![],
        edges: vec![],
        faces: ids.into(),
    }
}
fn load(p: &Project, s: &RomSession, path: &str) -> Nsbmd {
    Nsbmd::parse(&preview::plain(p.bytes(s, path).unwrap()).unwrap()).unwrap()
}
fn session() -> RomSession {
    RomSession::from_bytes(synthetic::wizard_rom(), "/external/synthetic.nds".into()).unwrap()
}
fn project(s: &RomSession) -> Project {
    Project::in_memory(s, Manifest::default(), BTreeMap::new()).unwrap()
}
fn transform(part: u32) -> ModelAction {
    ModelAction::Transform {
        part,
        translation: [1., 2., 3.],
        rotation: [0., 0., 0., 1.],
        scale: [1.; 3],
    }
}

#[test]
fn linked_runtime_variants_transform_and_undo_atomically() {
    let s = session();
    let mut p = project(&s);
    let path = "data/KartModelSub.carc/character/mario/P_MR.nsbmd";
    let copies = linked::copies(Some(&p), &s, path).unwrap();
    assert_eq!(copies.len(), 3);
    let before: Vec<_> = copies.iter().map(|c| load(&p, &s, &c.path)).collect();
    linked::edit(&mut p, &s, path, vec![transform(0)], true).unwrap();
    assert_eq!(p.history().len(), 1);
    assert_eq!(p.history()[0].paths.len(), 3);
    for (c, before) in copies.iter().zip(&before) {
        assert!(!c.path.contains("KartModelMenu.carc/"));
        assert!(!c.path.contains("KartModelMain.carc/"));
        let after = load(&p, &s, &c.path);
        for (&a, &b) in before.models[0].meshes[0]
            .indices
            .iter()
            .zip(&after.models[0].meshes[0].indices)
        {
            let a = before.models[0].meshes[0].positions[a as usize];
            let b = after.models[0].meshes[0].positions[b as usize];
            for k in 0..3 {
                assert!((b[k] - a[k] - (k + 1) as f32).abs() < 0.01);
            }
        }
    }
    p.undo().unwrap();
    assert!(p.edits.is_empty());
    p.redo().unwrap();
    assert_eq!(p.edits.len(), 3);
    let edits = p.edits.clone();
    let history = p.history().len();
    assert!(linked::edit(&mut p, &s, path, vec![transform(999)], true).is_err());
    assert_eq!(edits, p.edits);
    assert_eq!(history, p.history().len());
    linked::edit(&mut p, &s, path, vec![transform(0)], false).unwrap();
    assert_eq!(p.history().last().unwrap().paths, vec![path]);
}

#[test]
fn linked_import_uses_each_measured_budget_and_its_skeleton() {
    let mut pmodel = quad();
    pmodel.models[0].bones.push(Bone {
        name: "arm".into(),
        parent: Some(0),
        translation: [0., 5., 0.],
        rotation: [0., 0., 0., 1.],
        scale: [1.; 3],
    });
    let mut far = quad();
    far.models[0].meshes[0].indices.truncate(3);
    let mut files = synthetic::files();
    files.insert(
        "data/KartModelSub.carc".into(),
        Narc::new(BTreeMap::from([(
            "character/mario/P_MR.nsbmd".into(),
            pmodel.write().unwrap(),
        )]))
        .unwrap()
        .write(),
    );
    files.insert(
        "data/KartModelMainA.carc".into(),
        Narc::new(BTreeMap::from([(
            "character/mario/E_MR.nsbmd".into(),
            far.write().unwrap(),
        )]))
        .unwrap()
        .write(),
    );
    files.insert(
        "data/KartModelMenu/character/mario/P_MR.nsbmd".into(),
        pmodel.write().unwrap(),
    );
    let s = RomSession::from_bytes(synthetic::rom_with(files), "/external/synthetic.nds".into())
        .unwrap();
    let mut p = project(&s);
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("author.obj");
    std::fs::write(
        &source,
        "v 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nf 1 2 3 4\n",
    )
    .unwrap();
    let path = "data/KartModelSub.carc/character/mario/P_MR.nsbmd";
    linked::edit(
        &mut p,
        &s,
        path,
        vec![ModelAction::Import {
            source: source.to_string_lossy().into(),
        }],
        true,
    )
    .unwrap();
    assert_eq!(p.history().len(), 1);
    for c in linked::copies(Some(&p), &s, path).unwrap() {
        assert!(c.triangles <= c.budget);
        let m = load(&p, &s, &c.path);
        assert_eq!(
            m.models[0].bones.len(),
            if c.role == "Far-away driver" { 1 } else { 2 }
        );
    }
}

#[test]
fn extrude_and_inset_regions_have_only_boundary_walls_and_keep_skin() {
    for operation in [
        Operation::Extrude { amount: 2. },
        Operation::Inset { amount: 1. },
    ] {
        let mut model = quad();
        let m = &mut model.models[0].meshes[0];
        m.joints = vec![[0; 4]; m.positions.len()];
        m.weights = vec![[1., 0., 0., 0.]; m.positions.len()];
        mesh_edit::apply(m, &faces(&[0, 1]), &operation).unwrap();
        assert_eq!(m.indices.len() / 3, 10);
        assert_eq!(m.positions.len(), 8);
        assert_eq!(m.positions.len(), m.joints.len());
        assert!(m.weights.iter().all(|w| *w == [1., 0., 0., 0.]));
        match operation {
            Operation::Extrude { .. } => {
                assert_eq!(m.positions.iter().filter(|p| p[2] == 2.).count(), 4)
            }
            _ => assert_eq!(
                m.positions
                    .iter()
                    .filter(|p| p[0] >= 0.9 && p[0] <= 9.1 && p[1] >= 0.9 && p[1] <= 9.1)
                    .count(),
                4
            ),
        }
        let decoded = Nsbmd::parse(&model.write().unwrap()).unwrap();
        assert_eq!(decoded.models[0].meshes[0].indices.len() / 3, 10);
    }
}

#[test]
fn selected_transforms_deletes_edges_merge_and_invalid_selections() {
    let mut m = quad().models.remove(0).meshes.remove(0);
    let selection = Selection {
        mode: "vertex".into(),
        vertices: vec![0, 1],
        edges: vec![],
        faces: vec![],
    };
    mesh_edit::apply(
        &mut m,
        &selection,
        &Operation::Transform {
            translation: [0., 0., 2.],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            pivot: [5., 5., 0.],
        },
    )
    .unwrap();
    assert_eq!(m.positions[0][2], 2.);
    assert_eq!(m.positions[2][2], 0.);
    mesh_edit::apply(&mut m, &selection, &Operation::Merge).unwrap();
    assert_eq!(m.indices.len() / 3, 1);
    assert_eq!(m.positions.len(), 3);
    let mut m = quad().models.remove(0).meshes.remove(0);
    let edges = Selection {
        mode: "edge".into(),
        vertices: vec![],
        edges: vec![[0, 1]],
        faces: vec![],
    };
    mesh_edit::apply(&mut m, &edges, &Operation::Extrude { amount: -2. }).unwrap();
    assert_eq!(m.indices.len() / 3, 4);
    let mut m = quad().models.remove(0).meshes.remove(0);
    mesh_edit::apply(&mut m, &edges, &Operation::Delete).unwrap();
    assert_eq!(m.indices.len() / 3, 1);
    assert!(mesh_edit::apply(&mut m, &faces(&[999]), &Operation::Delete).is_err());
    assert!(
        mesh_edit::apply(&mut m, &faces(&[0]), &Operation::Inset { amount: f32::NAN }).is_err()
    );
}

#[test]
fn edited_models_reject_ds_budget_overflow_without_a_history_entry() {
    let mut model = quad();
    let m = &mut model.models[0].meshes[0];
    m.indices = m.indices.repeat(1024);
    let path = "data/author.nsbmd";
    let mut files = synthetic::files();
    files.insert(path.into(), model.write().unwrap());
    let s = RomSession::from_bytes(synthetic::rom_with(files), "/external/synthetic.nds".into())
        .unwrap();
    let mut p = project(&s);
    let original = load(&p, &s, path);
    let mesh = &original.models[0].meshes[0];
    let action = ModelAction::Edit {
        part: 0,
        selection: faces(&[0]),
        operation: Operation::Extrude { amount: 1. },
    };
    assert_eq!(mesh.indices.len() / 3, 2048);
    assert!(linked::edit(&mut p, &s, path, vec![action], false).is_err());
    assert!(p.edits.is_empty());
    assert!(p.history().is_empty());
}

#[test]
fn model_contents_bind_exact_resources_and_texture_edits_roundtrip_and_undo() {
    let s = session();
    let mut p = project(&s);
    let path = "data/KartModelSub.carc/character/mario/P_MR.nsbmd";
    let before = CourseScene::load(Some(&p), &s, path).unwrap();
    assert!(!before.info.bone_details.is_empty());
    let d = &before.info.material_details[0];
    assert!(d.path.as_ref().unwrap().ends_with("P_MR.nsbtx"));
    assert!(before.info.bone_details[0].vertices > 0);
    toolbox_core::engine::material(&mut p, &s, path, 0, MaterialAction::Tint { color: 0x1234 })
        .unwrap();
    let after = CourseScene::load(Some(&p), &s, path).unwrap();
    assert_eq!(
        after.textures.as_ref().unwrap().decode(0, 0).unwrap().rgba[0..3],
        nitro_formats::graphics::bgr555_to_rgba(0x1234)[0..3]
    );
    assert_eq!(p.history().len(), 1);
    p.undo().unwrap();
    assert!(p.edits.is_empty());
    // Embedded texture replacement also uses the shared model writer.
    let mut embedded = before.model.unwrap();
    embedded.textures = before.textures;
    let mut files = synthetic::files();
    files.insert("data/embedded.nsbmd".into(), embedded.write().unwrap());
    let s = RomSession::from_bytes(synthetic::rom_with(files), "/external/synthetic.nds".into())
        .unwrap();
    let mut p = project(&s);
    toolbox_core::engine::material(
        &mut p,
        &s,
        "data/embedded.nsbmd",
        0,
        MaterialAction::Tint { color: 0x2345 },
    )
    .unwrap();
    assert_eq!(
        load(&p, &s, "data/embedded.nsbmd")
            .textures
            .unwrap()
            .decode(0, 0)
            .unwrap()
            .rgba[0..3],
        nitro_formats::graphics::bgr555_to_rgba(0x2345)[0..3]
    );
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("authored.png");
    std::fs::write(
        &image,
        preview::png(&nitro_formats::graphics::Image {
            width: 13,
            height: 9,
            rgba: [40, 150, 210, 255].repeat(117),
        })
        .unwrap(),
    )
    .unwrap();
    toolbox_core::engine::material(
        &mut p,
        &s,
        "data/embedded.nsbmd",
        0,
        MaterialAction::Replace {
            source: image.to_string_lossy().into(),
        },
    )
    .unwrap();
    let out = load(&p, &s, "data/embedded.nsbmd");
    let t = &out.textures.as_ref().unwrap().textures[0];
    assert_eq!((t.width, t.height), (16, 16));
    assert_eq!(out.models[0].materials[0].width, 16);
}

#[test]
fn mapping_between_variants_does_not_reuse_vertex_indices() {
    let a = quad().models.remove(0).meshes.remove(0);
    let mut b: Mesh = a.clone();
    b.positions.reverse();
    let s = Selection {
        mode: "vertex".into(),
        vertices: vec![0],
        edges: vec![],
        faces: vec![],
    };
    let mapped = mesh_edit::map_selection(&a, &b, &s).unwrap();
    assert_eq!(mapped.vertices, vec![3]);
}

#[test]
fn selected_parts_and_indexed_palette_commands_are_atomic_and_preserve_other_parts() {
    let s = session();
    let mut p = project(&s);
    let path = "data/Studio/editing.nsbmd";
    let before = load(&p, &s, path);
    linked::edit(&mut p, &s, path, vec![transform(0), transform(1)], false).unwrap();
    assert_eq!(p.history().len(), 1);
    assert_eq!(p.history()[0].paths, vec![path]);
    p.undo().unwrap();
    assert!(p.edits.is_empty());
    linked::edit(&mut p, &s, path, vec![transform(0)], false).unwrap();
    let after = load(&p, &s, path);
    assert_eq!(
        before.models[0].meshes[1].positions,
        after.models[0].meshes[1].positions
    );
    let edits = p.edits.clone();
    assert!(linked::edit(&mut p, &s, path, vec![transform(0), transform(999)], false).is_err());
    assert_eq!(edits, p.edits);
    toolbox_core::engine::material(
        &mut p,
        &s,
        path,
        0,
        MaterialAction::Recolour {
            index: 1,
            color: 0x1234,
        },
    )
    .unwrap();
    assert_eq!(
        CourseScene::load(Some(&p), &s, path)
            .unwrap()
            .info
            .material_details[0]
            .colors[1],
        0x1234
    );
    let mut m = quad().models.remove(0).meshes.remove(0);
    mesh_edit::apply(
        &mut m,
        &Selection {
            mode: "vertex".into(),
            vertices: vec![1],
            edges: vec![],
            faces: vec![],
        },
        &Operation::Delete,
    )
    .unwrap();
    assert_eq!(m.indices.len() / 3, 1);
}

#[test]
fn linked_import_keeps_named_external_banks_and_clips() {
    let s = session();
    let mut p = project(&s);
    let path = "data/KartModelSub.carc/character/mario/P_MR.nsbmd";
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("authored.obj");
    std::fs::write(&file, "v 0 0 0\nv 10 0 0\nv 10 10 0\nf 1 2 3\n").unwrap();
    linked::edit(
        &mut p,
        &s,
        path,
        vec![ModelAction::Import {
            source: file.to_string_lossy().into(),
        }],
        true,
    )
    .unwrap();
    assert_eq!(p.history().len(), 1);
    assert_eq!(p.edits.len(), 6);
    for c in linked::copies(Some(&p), &s, path).unwrap() {
        let scene = CourseScene::load(Some(&p), &s, &c.path).unwrap();
        assert_eq!(scene.info.material_details[0].format, 3);
        assert!(scene.info.material_details[0].palette.is_some());
        assert!(scene.info.material_details[0]
            .path
            .as_ref()
            .unwrap()
            .ends_with(".nsbtx"));
        if c.role != "Far-away driver" {
            assert_eq!(scene.animations.len(), 4);
        }
    }
}

#[test]
fn owned_rom_linked_driver_edits_roundtrip_in_memory_only() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("MKDS_ROM unset; skipped");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let mut p = project(&s);
    let path = "data/KartModelSub.carc/character/mario/P_MR.nsbmd";
    let original = CourseScene::load(None, &s, path).unwrap();
    let actions = (0..original.info.parts.len())
        .map(|i| transform(i as u32))
        .collect();
    linked::edit(&mut p, &s, path, actions, true).unwrap();
    assert_eq!(p.history().len(), 1);
    for c in linked::copies(Some(&p), &s, path).unwrap() {
        let edited = CourseScene::load(Some(&p), &s, &c.path).unwrap();
        assert!(edited.info.budget.reasons.is_empty());
        assert!(!edited.glb(false).unwrap().is_empty());
    }
    p.undo().unwrap();
    assert!(p.edits.is_empty());
}
