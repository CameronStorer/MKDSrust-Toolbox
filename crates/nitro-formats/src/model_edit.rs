//! Mesh operations in bind/model space. All edits retain per-vertex attributes.
use crate::{
    error,
    math::{self, cross, dot, normalize, Mat4},
    model::Mesh,
    Result,
};
pub fn validate_mesh(m: &Mesh) -> Result<()> {
    let n = m.positions.len();
    if n > 1_000_000
        || !m.indices.len().is_multiple_of(3)
        || m.indices.iter().any(|&i| i as usize >= n)
        || m.positions
            .iter()
            .flatten()
            .chain(m.normals.iter().flatten())
            .chain(m.uvs.iter().flatten())
            .chain(m.colors.iter().flatten())
            .any(|v| !v.is_finite())
        || [
            m.normals.len(),
            m.uvs.len(),
            m.colors.len(),
            m.joints.len(),
            m.weights.len(),
        ]
        .iter()
        .any(|&l| l != 0 && l != n)
        || m.joints.len() != m.weights.len()
    {
        return Err(error(0, "invalid mesh indices or vertex attributes"));
    }
    Ok(())
}
pub fn transform(m: &mut Mesh, matrix: Mat4) -> Result<()> {
    validate_mesh(m)?;
    let inv = math::inverse(matrix).ok_or_else(|| error(0, "singular transform"))?;
    if matrix.iter().any(|v| !v.is_finite()) {
        return Err(error(0, "non-finite transform"));
    }
    for p in &mut m.positions {
        *p = math::point(matrix, *p);
    }
    for n in &mut m.normals {
        let old = *n;
        *n = normalize(std::array::from_fn(|r| {
            (0..3).map(|c| inv[r * 4 + c] * old[c]).sum()
        }));
    }
    let determinant = dot(
        cross(
            [matrix[0], matrix[1], matrix[2]],
            [matrix[4], matrix[5], matrix[6]],
        ),
        [matrix[8], matrix[9], matrix[10]],
    );
    if determinant < 0. {
        for t in m.indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
    }
    Ok(())
}
pub fn recompute_normals(m: &mut Mesh) -> Result<()> {
    validate_mesh(m)?;
    m.normals = vec![[0.; 3]; m.positions.len()];
    for t in m.indices.chunks_exact(3) {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        let u = std::array::from_fn(|k| m.positions[b][k] - m.positions[a][k]);
        let v = std::array::from_fn(|k| m.positions[c][k] - m.positions[a][k]);
        let n = cross(u, v);
        for i in [a, b, c] {
            for (x, y) in m.normals[i].iter_mut().zip(n) {
                *x += y;
            }
        }
    }
    for n in &mut m.normals {
        *n = normalize(*n);
    }
    Ok(())
}
fn vertex(out: &mut Mesh, src: &Mesh, a: usize, b: usize, t: f32) -> u32 {
    let i = out.positions.len() as u32;
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    out.positions.push(std::array::from_fn(|k| {
        lerp(src.positions[a][k], src.positions[b][k])
    }));
    if !src.normals.is_empty() {
        out.normals.push(normalize(std::array::from_fn(|k| {
            lerp(src.normals[a][k], src.normals[b][k])
        })));
    }
    if !src.uvs.is_empty() {
        out.uvs
            .push(std::array::from_fn(|k| lerp(src.uvs[a][k], src.uvs[b][k])));
    }
    if !src.colors.is_empty() {
        out.colors.push(std::array::from_fn(|k| {
            lerp(src.colors[a][k], src.colors[b][k])
        }));
    }
    if !src.joints.is_empty() {
        let mut influences = std::collections::BTreeMap::<u16, f32>::new();
        for (id, factor) in [(a, 1. - t), (b, t)] {
            for (&j, &w) in src.joints[id].iter().zip(&src.weights[id]) {
                *influences.entry(j).or_default() += w * factor;
            }
        }
        let mut v = influences
            .into_iter()
            .filter(|(_, w)| *w > 0.)
            .collect::<Vec<_>>();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        let sum = v.iter().take(4).map(|(_, w)| *w).sum::<f32>();
        let mut js = [0; 4];
        let mut ws = [0.; 4];
        for (k, (j, w)) in v.into_iter().take(4).enumerate() {
            js[k] = j;
            ws[k] = w / sum.max(f32::EPSILON);
        }
        out.joints.push(js);
        out.weights.push(ws);
    }
    i
}
pub fn cut_plane(m: &mut Mesh, normal: [f32; 3], distance: f32, keep_positive: bool) -> Result<()> {
    validate_mesh(m)?;
    if dot(normal, normal) < 1e-12 || !distance.is_finite() || normal.iter().any(|v| !v.is_finite())
    {
        return Err(error(0, "invalid cut plane"));
    }
    let mut out = Mesh {
        name: m.name.clone(),
        material: m.material,
        ..Mesh::default()
    };
    for tri in m.indices.chunks_exact(3) {
        let mut polygon = vec![];
        for edge in 0..3 {
            let a = tri[edge] as usize;
            let b = tri[(edge + 1) % 3] as usize;
            let sign = if keep_positive { 1. } else { -1. };
            let da = (dot(m.positions[a], normal) - distance) * sign;
            let db = (dot(m.positions[b], normal) - distance) * sign;
            if da >= 0. {
                polygon.push(vertex(&mut out, m, a, a, 0.));
            }
            if (da < 0.) != (db < 0.) {
                polygon.push(vertex(&mut out, m, a, b, da / (da - db)));
            }
        }
        for i in 1..polygon.len().saturating_sub(1) {
            out.indices.extend([polygon[0], polygon[i], polygon[i + 1]]);
        }
    }
    *m = out;
    Ok(())
}
pub fn delete_triangles(m: &mut Mesh, selected: &[usize]) -> Result<()> {
    validate_mesh(m)?;
    if selected.iter().any(|i| *i >= m.indices.len() / 3) {
        return Err(error(0, "triangle selection outside mesh"));
    }
    let selected = selected
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    m.indices = m
        .indices
        .chunks_exact(3)
        .enumerate()
        .filter(|(i, _)| !selected.contains(i))
        .flat_map(|(_, t)| t.iter().copied())
        .collect();
    compact(m);
    Ok(())
}
pub fn select_box(m: &Mesh, min: [f32; 3], max: [f32; 3]) -> Vec<usize> {
    m.indices
        .chunks_exact(3)
        .enumerate()
        .filter(|(_, t)| {
            t.iter().all(|&i| {
                (0..3).all(|k| {
                    m.positions[i as usize][k] >= min[k] && m.positions[i as usize][k] <= max[k]
                })
            })
        })
        .map(|(i, _)| i)
        .collect()
}
pub fn connected_part(m: &Mesh, triangle: usize) -> Result<Vec<usize>> {
    validate_mesh(m)?;
    if triangle >= m.indices.len() / 3 {
        return Err(error(0, "triangle selection outside mesh"));
    }
    // Display lists may repeat vertices at UV/normal seams. Connectivity is
    // geometric, so coincident corners still belong to the same part.
    let key = |i: u32| m.positions[i as usize].map(|v| if v == 0. { 0 } else { v.to_bits() });
    let mut vertices = std::collections::BTreeSet::new();
    vertices.extend(
        m.indices[triangle * 3..triangle * 3 + 3]
            .iter()
            .map(|&i| key(i)),
    );
    let mut seen = std::collections::BTreeSet::from([triangle]);
    loop {
        let old = seen.len();
        for (i, t) in m.indices.chunks_exact(3).enumerate() {
            if t.iter().any(|&v| vertices.contains(&key(v))) {
                seen.insert(i);
                vertices.extend(t.iter().map(|&i| key(i)));
            }
        }
        if seen.len() == old {
            break;
        }
    }
    Ok(seen.into_iter().collect())
}
fn compact(m: &mut Mesh) {
    let mut out = Mesh {
        name: m.name.clone(),
        material: m.material,
        ..Mesh::default()
    };
    let mut map = std::collections::BTreeMap::new();
    for &i in &m.indices {
        let index = *map
            .entry(i)
            .or_insert_with(|| vertex(&mut out, m, i as usize, i as usize, 0.));
        out.indices.push(index);
    }
    *m = out;
}
pub fn merge_vertices(m: &mut Mesh, tolerance: f32) -> Result<()> {
    validate_mesh(m)?;
    if !tolerance.is_finite() || tolerance < 0. {
        return Err(error(0, "invalid merge distance"));
    }
    let mut map = vec![];
    let mut exact = std::collections::HashMap::<Vec<u32>, u32>::new();
    for (i, p) in m.positions.iter().enumerate() {
        if tolerance == 0. {
            let bits = |v: f32| if v == 0. { 0 } else { v.to_bits() };
            let mut key = p.iter().copied().map(bits).collect::<Vec<_>>();
            if !m.normals.is_empty() {
                key.extend(m.normals[i].iter().copied().map(bits));
            }
            if !m.uvs.is_empty() {
                key.extend(m.uvs[i].iter().copied().map(bits));
            }
            if !m.colors.is_empty() {
                key.extend(m.colors[i].iter().copied().map(bits));
            }
            if !m.joints.is_empty() {
                key.extend(m.joints[i].iter().map(|&v| v as u32));
                key.extend(m.weights[i].iter().copied().map(bits));
            }
            map.push(*exact.entry(key).or_insert(i as u32));
            continue;
        }
        let found = (0..i).find(|&j| {
            p.iter()
                .zip(m.positions[j])
                .all(|(a, b)| (*a - b).abs() <= tolerance)
                && (m.uvs.is_empty() || m.uvs[i] == m.uvs[j])
                && (m.normals.is_empty() || m.normals[i] == m.normals[j])
                && (m.colors.is_empty() || m.colors[i] == m.colors[j])
                && (m.joints.is_empty()
                    || (m.joints[i] == m.joints[j] && m.weights[i] == m.weights[j]))
        });
        map.push(found.map_or(i as u32, |j| map[j]));
    }
    for i in &mut m.indices {
        *i = map[*i as usize];
    }
    m.indices = m
        .indices
        .chunks_exact(3)
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
        .flatten()
        .copied()
        .collect();
    compact(m);
    Ok(())
}
/// Garland–Heckbert plane quadrics with conservative edge collapse. UV/skin
/// seams are preserved; a target can be unreachable without destroying them.
pub fn decimate(m: &mut Mesh, target: usize) -> Result<()> {
    decimate_impl(m, target, 2)
}
/// LOD reduction may collapse a two-triangle patch to one, but never erase an isolated triangle.
/// Scene reduction retains the stricter default so small road surfaces survive.
pub fn decimate_lod(m: &mut Mesh, target: usize) -> Result<()> {
    decimate_impl(m, target, 1)
}
fn decimate_impl(m: &mut Mesh, target: usize, remaining: usize) -> Result<()> {
    validate_mesh(m)?;
    merge_vertices(m, 0.)?;
    if target == 0 {
        m.indices.clear();
        compact(m);
        return Ok(());
    }
    while m.indices.len() / 3 > target {
        let mut q = vec![[0f64; 16]; m.positions.len()];
        let mut edges = std::collections::BTreeSet::new();
        let mut adjacent = vec![Vec::new(); m.positions.len()];
        for (id, t) in m.indices.chunks_exact(3).enumerate() {
            for &v in t {
                adjacent[v as usize].push(id);
            }
            let p = t
                .iter()
                .map(|&i| m.positions[i as usize])
                .collect::<Vec<_>>();
            let n = normalize(cross(
                std::array::from_fn(|k| p[1][k] - p[0][k]),
                std::array::from_fn(|k| p[2][k] - p[0][k]),
            ));
            let plane = [n[0] as f64, n[1] as f64, n[2] as f64, -dot(n, p[0]) as f64];
            for &i in t {
                for a in 0..4 {
                    for b in 0..4 {
                        q[i as usize][a * 4 + b] += plane[a] * plane[b];
                    }
                }
            }
            for k in 0..3 {
                let a = t[k] as usize;
                let b = t[(k + 1) % 3] as usize;
                edges.insert((a.min(b), a.max(b)));
            }
        }
        let mut candidates = edges
            .into_iter()
            .filter_map(|(a, b)| {
                if !m.joints.is_empty()
                    && (m.joints[a] != m.joints[b] || m.weights[a] != m.weights[b])
                {
                    return None;
                }
                let shared = adjacent[a]
                    .iter()
                    .filter(|&&id| m.indices[id * 3..id * 3 + 3].contains(&(b as u32)))
                    .count();
                // Do not erase a small disconnected piece (for example a map floor).
                if adjacent[a].len() + adjacent[b].len() - shared * 2 < remaining {
                    return None;
                }
                let quad: [f64; 16] = std::array::from_fn(|i| q[a][i] + q[b][i]);
                let mut system = math::IDENTITY;
                for r in 0..3 {
                    for c in 0..3 {
                        system[c * 4 + r] = quad[r * 4 + c] as f32;
                    }
                }
                let rhs = [-quad[3] as f32, -quad[7] as f32, -quad[11] as f32];
                let midpoint =
                    std::array::from_fn(|k| (m.positions[a][k] + m.positions[b][k]) / 2.);
                let p = math::inverse(system).map_or(midpoint, |inv| math::vector(inv, rhs));
                if p.iter().any(|v| !v.is_finite()) {
                    return None;
                }
                // Do not invert remaining faces.
                for id in adjacent[a].iter().chain(&adjacent[b]) {
                    let t = &m.indices[*id * 3..*id * 3 + 3];
                    if t.contains(&(a as u32)) && t.contains(&(b as u32)) {
                        continue;
                    }
                    let old = t
                        .iter()
                        .map(|&i| m.positions[i as usize])
                        .collect::<Vec<_>>();
                    let new = t
                        .iter()
                        .map(|&i| {
                            if i as usize == a || i as usize == b {
                                p
                            } else {
                                m.positions[i as usize]
                            }
                        })
                        .collect::<Vec<_>>();
                    let normal = |v: &Vec<[f32; 3]>| {
                        cross(
                            std::array::from_fn(|k| v[1][k] - v[0][k]),
                            std::array::from_fn(|k| v[2][k] - v[0][k]),
                        )
                    };
                    if dot(normal(&old), normal(&new)) <= 0. {
                        return None;
                    }
                }
                let v = [p[0] as f64, p[1] as f64, p[2] as f64, 1.];
                let cost = (0..4)
                    .flat_map(|r| (0..4).map(move |c| v[r] * quad[r * 4 + c] * v[c]))
                    .sum::<f64>();
                Some((cost, a, b, p))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        let mut touched = vec![false; m.positions.len()];
        let mut remap: Vec<u32> = (0..m.positions.len() as u32).collect();
        let mut removed = 0;
        let mut collapsed = false;
        for (_, a, b, p) in candidates {
            if touched[a] || touched[b] {
                continue;
            }
            let shared = adjacent[a]
                .iter()
                .filter(|id| m.indices[**id * 3..**id * 3 + 3].contains(&(b as u32)))
                .count();
            if m.indices.len() / 3 <= target + removed {
                break;
            }
            // Lock the entire one-ring so batched collapses cannot invert each other's faces.
            for id in adjacent[a].iter().chain(&adjacent[b]) {
                for &v in &m.indices[*id * 3..*id * 3 + 3] {
                    touched[v as usize] = true;
                }
            }
            remap[b] = a as u32;
            removed += shared;
            collapsed = true;
            m.positions[a] = p;
            if !m.uvs.is_empty() {
                m.uvs[a] = std::array::from_fn(|k| (m.uvs[a][k] + m.uvs[b][k]) * 0.5);
            }
            if !m.colors.is_empty() {
                m.colors[a] = std::array::from_fn(|k| (m.colors[a][k] + m.colors[b][k]) * 0.5);
            }
        }
        if !collapsed {
            break;
        }
        for i in &mut m.indices {
            *i = remap[*i as usize];
        }
        m.indices = m
            .indices
            .chunks_exact(3)
            .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
            .flatten()
            .copied()
            .collect();
    }
    compact(m);
    recompute_normals(m)
}
