use nitro_formats::{
    math,
    model::{self, Bone, Nsbmd},
    texture::Nsbtx,
};
use toolbox_core::{model as core_model, model_import};
fn model() -> Nsbmd {
    model_import::obj("v 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nf 1 2 3 4\n").unwrap()
}
fn geometry_equal(a: &Nsbmd, b: &Nsbmd) {
    for (a, b) in a.models[0].meshes.iter().zip(&b.models[0].meshes) {
        assert_eq!(a.indices.len(), b.indices.len());
        for (&i, &j) in a.indices.iter().zip(&b.indices) {
            for (x, y) in a.positions[i as usize].iter().zip(b.positions[j as usize]) {
                assert!((x - y).abs() < 0.01, "{x} != {y}");
            }
        }
    }
}
#[test]
fn authored_textured_skinned_nsbmd_and_glb_roundtrip() {
    let mut a = model();
    let image = nitro_formats::graphics::Image {
        width: 13,
        height: 11,
        rgba: [80, 180, 220, 255].repeat(143),
    };
    let (t, p) = core_model::convert_texture(&image, "paint", 3, 128).unwrap();
    assert_eq!((t.width, t.height), (16, 16));
    a.textures = Some(Nsbtx::new(vec![t], vec![p]).unwrap());
    let m = &mut a.models[0];
    m.materials[0].texture = Some("paint".into());
    m.materials[0].palette = Some("paint".into());
    m.materials[0].width = 16;
    m.materials[0].height = 16;
    m.bones.push(Bone {
        name: "arm".into(),
        parent: Some(0),
        translation: [1., 0., 0.],
        rotation: [0., 0., 0., 1.],
        scale: [1.; 3],
    });
    let mesh = &mut m.meshes[0];
    mesh.joints = vec![[0, 1, 0, 0]; 4];
    mesh.weights = vec![[0.5, 0.5, 0., 0.]; 4];
    let bytes = a.write().unwrap();
    let b = Nsbmd::parse(&bytes).unwrap();
    geometry_equal(&a, &b);
    assert_eq!(b.models[0].bones.len(), 2);
    assert_eq!(b.models[0].meshes[0].weights[0], [0.5, 0.5, 0., 0.]);
    assert_eq!(
        b.textures.as_ref().unwrap().textures[0].data,
        a.textures.as_ref().unwrap().textures[0].data
    );
    let glb = core_model::glb(Some(&b), None, &[], None).unwrap();
    let c = model_import::gltf(&glb, std::path::Path::new(".")).unwrap();
    geometry_equal(&b, &c);
    assert!(c.textures.is_some());
    assert_eq!(core_model::budget(&b, None).polygons, 2);
}
#[test]
fn rigid_bones_transforms_and_mesh_operations() {
    let mut a = model();
    a.models[0].bones[0].translation = [2., 3., 0.];
    let b = Nsbmd::parse(&a.write().unwrap()).unwrap();
    geometry_equal(&a, &b);
    let m = &mut a.models[0].meshes[0];
    model::transform(m, math::trs([1., 2., 3.], [0., 0., 0., 1.], [2.; 3])).unwrap();
    assert_eq!(m.positions[0], [1., 2., 3.]);
    model::cut_plane(m, [0., 1., 0.], 10., true).unwrap();
    assert!(m.positions.iter().all(|p| p[1] >= 10. - 0.0001));
    assert!(!m.indices.is_empty());
    model::merge_vertices(m, 0.001).unwrap();
    let before = m.indices.len() / 3;
    model::decimate(m, 1).unwrap();
    assert!(m.indices.len() / 3 < before);
    assert_eq!(m.normals.len(), m.positions.len());
    let part = model::connected_part(m, 0).unwrap();
    model::delete_triangles(m, &part).unwrap();
    assert!(m.indices.is_empty());
}
#[test]
fn malformed_imports_and_bad_skin_are_errors() {
    for text in ["f 1 2 3", "v NaN 0 0\nf 1 1 1"] {
        assert!(model_import::obj(text).is_err());
    }
    let mut a = model();
    a.models[0].meshes[0].indices[0] = 999;
    assert!(a.write().is_err());
    let mut a = model();
    a.models[0].bones[0].parent = Some(0);
    assert!(a.write().is_err());
    assert!(model_import::gltf(b"{}", std::path::Path::new(".")).is_err());
}

#[test]
fn strip_display_lists_preserve_winding_and_share_vertices() {
    let original = model();
    let decoded = Nsbmd::parse(&original.write_with_strips(true).unwrap()).unwrap();
    let a = &original.models[0].meshes[0];
    let b = &decoded.models[0].meshes[0];
    assert_eq!(b.positions.len(), 4);
    assert_eq!(b.indices.len(), a.indices.len());
    for (source, target) in a.indices.chunks_exact(3).zip(b.indices.chunks_exact(3)) {
        assert!(
            (0..3).any(|rotation| (0..3).all(|k| a.positions[source[k] as usize]
                .iter()
                .zip(b.positions[target[(k + rotation) % 3] as usize])
                .all(|(a, b)| (a - b).abs() < 0.01)))
        );
    }
}

#[test]
fn original_constant_jpeg_is_decoded_and_converted() {
    // An authored baseline 8x8 grey JPEG: DC zero, EOB, all quantizers one.
    // This fixture is constructed from the format, never from an image file.
    fn marker(out: &mut Vec<u8>, id: u8, payload: &[u8]) {
        out.extend([0xff, id]);
        out.extend(((payload.len() + 2) as u16).to_be_bytes());
        out.extend(payload);
    }
    let mut b = vec![0xff, 0xd8];
    let mut quant = vec![0];
    quant.extend([1; 64]);
    marker(&mut b, 0xdb, &quant);
    marker(&mut b, 0xc0, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0]);
    let mut huffman = vec![0; 18];
    huffman[1] = 1;
    marker(&mut b, 0xc4, &huffman);
    huffman[0] = 0x10;
    marker(&mut b, 0xc4, &huffman);
    marker(&mut b, 0xda, &[1, 1, 0, 0, 63, 0]);
    b.extend([0x3f, 0xff, 0xd9]);
    let image = toolbox_core::graphic::read_image_data(&b).unwrap();
    assert_eq!((image.width, image.height), (8, 8));
    assert!(image
        .rgba
        .chunks_exact(4)
        .all(|c| c == [128, 128, 128, 255]));
    let (texture, palette) = core_model::convert_texture(&image, "jpeg", 3, 256).unwrap();
    let bank = Nsbtx::new(vec![texture], vec![palette]).unwrap();
    assert_eq!(bank.decode(0, 0).unwrap().width, 8);
}
