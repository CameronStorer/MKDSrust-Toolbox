use nitro_formats::{animation, kcl::Kcl, model::Nsbmd, nkm::Nkm, synthetic};
#[test]
fn nkm_every_section_roundtrips_and_writes_records() {
    let b = synthetic::nkm();
    let mut n = Nkm::parse(&b).unwrap();
    assert_eq!(n.sections.len(), 19);
    assert_eq!(n.write().unwrap(), b);
    n.sections[0].entries[0][36] = 99;
    let edited = n.write().unwrap();
    assert_ne!(edited, b);
    assert_eq!(Nkm::parse(&edited).unwrap().sections[0].entries[0][36], 99);
    for end in 0..b.len() {
        let _ = Nkm::parse(&b[..end]);
    }
}
#[test]
fn kcl_roundtrip_prisms_and_octree() {
    let b = synthetic::kcl();
    let k = Kcl::parse(&b).unwrap();
    assert_eq!(k.write(), b);
    assert_eq!(k.octree[0].prisms, [0, 1]);
    let t = k.triangles().unwrap();
    assert_eq!(t.len(), 2);
    assert_eq!(t[1].attribute, 0x8100);
    assert!(t
        .iter()
        .flat_map(|t| t.positions)
        .flatten()
        .all(f32::is_finite));
    for end in 0..b.len() {
        let _ = Kcl::parse(&b[..end]);
    }
    let mut cyclic = b;
    cyclic[164..168].copy_from_slice(&0u32.to_le_bytes());
    assert!(Kcl::parse(&cyclic).is_err());
}
#[test]
fn gx_model_and_joint_frames() {
    let b = synthetic::nsbmd();
    let n = Nsbmd::parse(&b).unwrap();
    let m = &n.models[0];
    assert_eq!(m.bones[0].name, "root");
    assert_eq!(m.materials[0].texture.as_deref(), Some("studio"));
    assert_eq!(m.meshes[0].positions.len(), 8);
    assert_eq!(m.meshes[0].indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    assert_eq!(m.meshes[0].positions[0], [-384., 0., -384.]);
    assert_eq!(m.meshes[0].weights[0], [1., 0., 0., 0.]);
    let a = animation::parse(&synthetic::nsbca()).unwrap();
    assert_eq!(a[0].frames, 60);
    assert!((a[0].joints[0].translations.as_ref().unwrap()[15][0] - 1.).abs() < 0.001);
    assert_eq!(
        a[0].joints[0].rotations.as_ref().unwrap()[0],
        [0., 0., 0., 1.]
    );
    assert_eq!(
        animation::list(&synthetic::nsbca()).unwrap()[0].name,
        "Float"
    );
    for end in 0..b.len() {
        let _ = Nsbmd::parse(&b[..end]);
    }
}

#[test]
fn texture_matrix_roundtrips_and_maps_uvs() {
    let mut n = Nsbmd::parse(&synthetic::nsbmd()).unwrap();
    {
        let m = &mut n.models[0].materials[0];
        m.tex_mode = 1;
        m.tex_scale = [1., 7.];
        m.tex_translate = [0.25, 0.];
    }
    let back = Nsbmd::parse(&n.write().unwrap()).unwrap();
    let m = &back.models[0].materials[0];
    assert_eq!((m.tex_mode, m.tex_scale, m.tex_rotate, m.tex_translate), (1, [1., 7.], [0., 1.], [0.25, 0.]));
    // Maya rule: V scales about the bottom edge, so t=1 (bottom) stays put.
    assert_eq!(m.transform_uv([0., 1.]), [0.25, 1.]);
    assert_eq!(m.transform_uv([0.5, 0.]), [0.75, -6.]);
    let mut none = back.models[0].materials[0].clone();
    none.tex_mode = 0;
    assert_eq!(none.transform_uv([0.3, 0.4]), [0.3, 0.4]);
}
