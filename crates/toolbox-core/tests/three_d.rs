use toolbox_core::{model, scene::CourseScene, session::RomSession, synthetic};
#[test]
fn course_glb_embeds_texture_skin_animation_and_collision() {
    let s = RomSession::from_bytes(synthetic::rom(), Default::default()).unwrap();
    let scene = CourseScene::load(None, &s, "data/Course/studio.carc/course_map.nkm").unwrap();
    assert_eq!(scene.info.budget.polygons, 4);
    assert_eq!(scene.info.budget.texture_bytes, 128);
    assert!(scene.info.issues.is_empty(), "{:?}", scene.info.issues);
    let glb = scene.glb(true).unwrap();
    assert_eq!(&glb[..4], b"glTF");
    assert_eq!(
        u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize,
        glb.len()
    );
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let j: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    assert_eq!(j["asset"]["version"], "2.0");
    assert_eq!(j["skins"].as_array().unwrap().len(), 1);
    assert_eq!(j["animations"][0]["name"], "Float");
    assert_eq!(j["images"][0]["mimeType"], "image/png");
    assert_eq!(j["meshes"].as_array().unwrap().len(), 3);
    let binary_start = 20 + json_len + 8;
    for view in j["bufferViews"].as_array().unwrap() {
        let at = view["byteOffset"].as_u64().unwrap() as usize;
        let len = view["byteLength"].as_u64().unwrap() as usize;
        assert_eq!(at % 4, 0);
        assert!(binary_start + at + len <= glb.len())
    }
    let obj = String::from_utf8(model::obj(scene.kcl.as_ref().unwrap()).unwrap()).unwrap();
    assert_eq!(obj.lines().filter(|l| l.starts_with("f ")).count(), 2);
    assert!(obj.contains("usemtl collision_00"));
    assert!(obj.contains("usemtl collision_01"));
}
