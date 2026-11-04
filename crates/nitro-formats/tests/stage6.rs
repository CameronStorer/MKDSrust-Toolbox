use nitro_formats::model::{decimate, validate_mesh, Mesh};

#[test]
fn dense_mesh_reduces_within_ten_seconds() {
    let mut mesh = Mesh::default();
    // 31,600 triangles, generated here, with a curved surface to test quadrics.
    for y in 0..=100 {
        for x in 0..=158 {
            mesh.positions
                .push([x as f32, ((x as f32) * 0.04).sin() * 2., y as f32]);
        }
    }
    for y in 0..100u32 {
        for x in 0..158u32 {
            let a = y * 159 + x;
            mesh.indices
                .extend([a, a + 159, a + 1, a + 1, a + 159, a + 160]);
        }
    }
    let start = std::time::Instant::now();
    decimate(&mut mesh, 500).unwrap();
    eprintln!(
        "31,600 → {} triangles in {:?}",
        mesh.indices.len() / 3,
        start.elapsed()
    );
    assert!(start.elapsed().as_secs_f32() < 10.);
    assert!(mesh.indices.len() / 3 <= 500);
    assert!(!mesh.indices.is_empty());
    validate_mesh(&mesh).unwrap();
}
