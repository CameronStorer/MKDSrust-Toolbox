use nitro_formats::kcl::{Kcl, Triangle};
#[test]
fn prisms_octree_and_repaint_roundtrip() {
    let triangles = (0..40)
        .map(|i| {
            let x = i as f32 * 100.;
            Triangle {
                positions: [[x, 0., 0.], [x, 0., 50.], [x + 50., 0., 0.]],
                normal: [0., 1., 0.],
                attribute: i % 12,
            }
        })
        .collect::<Vec<_>>();
    let k = Kcl::from_triangles(&triangles).unwrap();
    assert_eq!(k.prisms.len(), 40);
    assert!(!k.octree[0].children.is_empty());
    let parsed = k.triangles().unwrap();
    for (a, b) in triangles.iter().zip(parsed) {
        assert_eq!(a.attribute, b.attribute);
        for (p, q) in a.positions.iter().zip(b.positions) {
            for (x, y) in p.iter().zip(q) {
                assert!((x - y).abs() < 0.1);
            }
        }
    }
    let mut k = Kcl::parse(&k.write()).unwrap();
    k.repaint(&[0, 39], 0x123).unwrap();
    let k = Kcl::parse(&k.write()).unwrap();
    assert_eq!(k.prisms[0].attribute, 0x123);
    assert_eq!(k.prisms[39].attribute, 0x123);
    assert!(k.repaint_clone_error());
}
trait BadSelection {
    fn repaint_clone_error(&self) -> bool;
}
impl BadSelection for Kcl {
    fn repaint_clone_error(&self) -> bool {
        self.clone().repaint(&[999], 0).is_err()
    }
}
#[test]
fn material_and_slope_classification() {
    let mut m = toolbox_core::model_import::obj("v 0 0 0\nv 100 0 0\nv 0 0 100\nf 1 3 2")
        .unwrap()
        .models
        .remove(0);
    let table = toolbox_core::collision::AttributeTable::default();
    m.materials[0].name = "grass".into();
    let k = toolbox_core::collision::generate(&m, &table).unwrap();
    assert_eq!(mkds_data::collision_type(k.prisms[0].attribute), 2);
    m.materials[0].name = "unknown".into();
    let k = toolbox_core::collision::generate(&m, &table).unwrap();
    assert_eq!(k.prisms[0].attribute, table.floor);
    assert!(Kcl::from_triangles(&[Triangle {
        positions: [[0.; 3]; 3],
        normal: [0.; 3],
        attribute: 0
    }])
    .is_err());
}
