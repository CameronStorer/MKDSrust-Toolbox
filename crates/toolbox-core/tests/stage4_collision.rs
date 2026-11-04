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
    assert!(k.octree.len() > 1);
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

fn dense_map(side: usize) -> Vec<Triangle> {
    let mut triangles = vec![];
    for z in 0..side {
        for x in 0..side {
            let step = 2000. / side as f32;
            let a = [x as f32 * step, 0., z as f32 * step];
            let b = [a[0] + step, 0., a[2]];
            let c = [a[0], 0., a[2] + step];
            let d = [b[0], 0., c[2]];
            for positions in [[a, c, b], [b, c, d]] {
                triangles.push(Triangle {
                    positions,
                    normal: [0., 1., 0.],
                    attribute: 0x8000,
                });
            }
        }
    }
    triangles
}

#[test]
fn dense_collision_has_a_root_grid_and_stays_in_the_course_heap() {
    use nitro_formats::kcl::{MAX_BYTES, WARN_BYTES};
    let triangles = dense_map(56); // 6,272 prisms on a 2,000-unit square.
    let k = Kcl::from_triangles(&triangles).unwrap();
    let bytes = k.write();
    println!(
        "dense KCL: {} bytes, {} prisms",
        bytes.len(),
        k.prisms.len()
    );
    assert!(bytes.len() <= MAX_BYTES);
    assert_eq!(k.budget_warning().is_some(), bytes.len() > WARN_BYTES);
    let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(word(44), 10);
    assert!(word(48) > 0 && word(52) >= word(48));
    let counts: [usize; 3] =
        std::array::from_fn(|axis| (((!word(32 + axis * 4)) >> 10) + 1) as usize);
    assert_eq!(k.octree.len(), counts.iter().product::<usize>());
    // Emulate the game's root addressing and traversal. Include vertices and
    // edges as well as centroids, including boundaries between root cubes.
    for (id, triangle) in triangles.iter().enumerate() {
        let center = std::array::from_fn(|axis| {
            triangle.positions.iter().map(|p| p[axis]).sum::<f32>() / 3.
        });
        for p in triangle.positions.into_iter().chain([center]) {
            let mut local: [f32; 3] = std::array::from_fn(|axis| p[axis] - k.minimum[axis]);
            let cell = local.map(|v| (v / 1024.) as usize);
            let root = cell[0] | (cell[1] << word(48)) | (cell[2] << word(52));
            let mut node = &k.octree[root];
            for axis in 0..3 {
                local[axis] -= cell[axis] as f32 * 1024.;
            }
            let mut size = 1024.;
            while !node.children.is_empty() {
                size /= 2.;
                let mut index = 0;
                for (axis, value) in local.iter_mut().enumerate() {
                    if *value >= size {
                        index |= 1 << axis;
                        *value -= size;
                    }
                }
                node = &node.children[index];
            }
            assert!(
                node.prisms.contains(&(id as u16)),
                "missing prism {id} at {p:?}"
            );
        }
    }
    let error = Kcl::from_triangles(&dense_map(100))
        .unwrap_err()
        .to_string();
    assert!(error.contains("215 KB"), "{error}");
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
