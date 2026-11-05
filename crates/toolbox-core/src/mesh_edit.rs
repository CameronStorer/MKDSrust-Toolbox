//! Selection-based mesh edits in bind space; all output goes through the Nitro model writer.
//! Independently implemented. UX follows Blender's default mesh keymap (see research notes).
use crate::{Error, Result};
use nitro_formats::{
    math,
    model::{self, Mesh},
};
use serde::Deserialize;
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Type)]
pub struct Selection {
    pub mode: String,
    pub vertices: Vec<u32>,
    pub edges: Vec<[u32; 2]>,
    pub faces: Vec<u32>,
}
#[derive(Clone, Debug, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum Operation {
    Transform {
        translation: [f32; 3],
        rotation: [f32; 4],
        scale: [f32; 3],
        pivot: [f32; 3],
    },
    Extrude {
        amount: f32,
    },
    Inset {
        amount: f32,
    },
    Delete,
    Merge,
}
impl Operation {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Transform { .. } => "Transform model",
            Self::Extrude { .. } => "Extrude selection",
            Self::Inset { .. } => "Inset selection",
            Self::Delete => "Delete model selection",
            Self::Merge => "Merge selected vertices",
        }
    }
}
fn invalid(msg: &str) -> Error {
    Error::Invalid(msg.into())
}
fn key(p: [f32; 3]) -> [u32; 3] {
    p.map(|v| if v == 0. { 0 } else { v.to_bits() })
}
fn logical(m: &Mesh) -> Vec<u32> {
    let mut ids = BTreeMap::new();
    m.positions
        .iter()
        .enumerate()
        .map(|(i, p)| *ids.entry(key(*p)).or_insert(i as u32))
        .collect()
}
pub fn vertices(m: &Mesh, s: &Selection) -> Result<BTreeSet<usize>> {
    model::validate_mesh(m)?;
    if !["vertex", "edge", "face"].contains(&s.mode.as_str()) {
        return Err(invalid("invalid selection mode"));
    }
    if s.vertices.iter().any(|&v| v as usize >= m.positions.len())
        || s.edges
            .iter()
            .flatten()
            .any(|&v| v as usize >= m.positions.len())
        || s.faces.iter().any(|&v| v as usize >= m.indices.len() / 3)
    {
        return Err(invalid(
            "selection outside mesh; select again after editing",
        ));
    }
    let ids: BTreeSet<_> = match s.mode.as_str() {
        "face" => s
            .faces
            .iter()
            .flat_map(|&i| {
                m.indices[i as usize * 3..i as usize * 3 + 3]
                    .iter()
                    .copied()
            })
            .collect(),
        "edge" => s.edges.iter().flatten().copied().collect(),
        _ => s.vertices.iter().copied().collect(),
    };
    let positions: BTreeSet<_> = ids.iter().map(|&i| key(m.positions[i as usize])).collect();
    Ok(m.positions
        .iter()
        .enumerate()
        .filter(|(_, p)| positions.contains(&key(**p)))
        .map(|(i, _)| i)
        .collect())
}
pub fn map_selection(src: &Mesh, dst: &Mesh, s: &Selection) -> Result<Selection> {
    let chosen = vertices(src, s)?;
    // Runtime variants may have different topology: nearest bind-space source vertex decides membership.
    let selected: BTreeSet<_> = dst
        .positions
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let nearest = src
                .positions
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let distance =
                        |q: &&[f32; 3]| (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f32>();
                    distance(a).total_cmp(&distance(b))
                })?
                .0;
            chosen.contains(&nearest).then_some(i as u32)
        })
        .collect();
    let faces = dst
        .indices
        .chunks_exact(3)
        .enumerate()
        .filter(|(_, t)| t.iter().all(|i| selected.contains(i)))
        .map(|(i, _)| i as u32)
        .collect();
    let edges: BTreeSet<_> = dst
        .indices
        .chunks_exact(3)
        .flat_map(|t| {
            (0..3).map(move |k| {
                let mut e = [t[k], t[(k + 1) % 3]];
                e.sort();
                e
            })
        })
        .filter(|e| e.iter().all(|i| selected.contains(i)))
        .collect();
    Ok(Selection {
        mode: s.mode.clone(),
        vertices: selected.into_iter().collect(),
        faces,
        edges: edges.into_iter().collect(),
    })
}
fn duplicate(m: &mut Mesh, i: usize) -> u32 {
    let id = m.positions.len() as u32;
    m.positions.push(m.positions[i]);
    macro_rules! copy {
        ($attr:ident) => {
            if !m.$attr.is_empty() {
                m.$attr.push(m.$attr[i]);
            }
        };
    }
    copy!(normals);
    copy!(uvs);
    copy!(colors);
    copy!(joints);
    copy!(weights);
    id
}
fn compact(m: &mut Mesh) {
    let used: BTreeSet<_> = m.indices.iter().copied().collect();
    let mut remap = BTreeMap::new();
    for (i, &v) in used.iter().enumerate() {
        remap.insert(v, i as u32);
    }
    macro_rules! pack {
        ($attr:ident) => {
            if !m.$attr.is_empty() {
                m.$attr = used.iter().map(|&i| m.$attr[i as usize]).collect();
            }
        };
    }
    pack!(positions);
    pack!(normals);
    pack!(uvs);
    pack!(colors);
    pack!(joints);
    pack!(weights);
    for i in &mut m.indices {
        *i = remap[i];
    }
}
pub fn apply(m: &mut Mesh, selection: &Selection, operation: &Operation) -> Result<()> {
    let chosen = vertices(m, selection)?;
    if chosen.is_empty() {
        return Err(invalid("select vertices, edges or faces first"));
    }
    let ids = logical(m);
    match operation {
        Operation::Transform {
            translation,
            rotation,
            scale,
            pivot,
        } => {
            let matrix = math::trs(*translation, *rotation, *scale);
            if math::inverse(matrix).is_none()
                || pivot.iter().chain(matrix.iter()).any(|v| !v.is_finite())
            {
                return Err(invalid("invalid or singular transform"));
            }
            for &i in &chosen {
                let local = std::array::from_fn(|k| m.positions[i][k] - pivot[k]);
                let p = math::point(matrix, local);
                m.positions[i] = std::array::from_fn(|k| p[k] + pivot[k]);
            }
        }
        Operation::Delete => {
            let faces: BTreeSet<_> = selection.faces.iter().map(|&i| i as usize).collect();
            let edges: BTreeSet<_> = selection
                .edges
                .iter()
                .map(|e| {
                    let mut e = e.map(|i| ids[i as usize]);
                    e.sort();
                    e
                })
                .collect();
            m.indices = m
                .indices
                .chunks_exact(3)
                .enumerate()
                .filter(|(i, t)| match selection.mode.as_str() {
                    "face" => !faces.contains(i),
                    "edge" => !(0..3).any(|k| {
                        let mut e = [ids[t[k] as usize], ids[t[(k + 1) % 3] as usize]];
                        e.sort();
                        edges.contains(&e)
                    }),
                    _ => !t.iter().any(|i| chosen.contains(&(*i as usize))),
                })
                .flat_map(|(_, t)| t.iter().copied())
                .collect();
            compact(m);
        }
        Operation::Merge => {
            let center: [f32; 3] = std::array::from_fn(|k| {
                chosen.iter().map(|&i| m.positions[i][k]).sum::<f32>() / chosen.len() as f32
            });
            let first = *chosen.first().unwrap() as u32;
            m.positions[first as usize] = center;
            for i in &mut m.indices {
                if chosen.contains(&(*i as usize)) {
                    *i = first;
                }
            }
            m.indices = m
                .indices
                .chunks_exact(3)
                .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
                .flatten()
                .copied()
                .collect();
            compact(m);
        }
        Operation::Extrude { amount } | Operation::Inset { amount } => {
            if !amount.is_finite() {
                return Err(invalid("enter a finite distance"));
            }
            let inset = matches!(operation, Operation::Inset { .. });
            if inset && (*amount < 0. || selection.mode != "face") {
                return Err(invalid("Inset needs faces and a nonnegative thickness"));
            }
            if !inset && selection.mode == "vertex" {
                return Err(invalid("Select faces or edges to extrude"));
            }
            let faces: BTreeSet<_> = selection.faces.iter().map(|&i| i as usize).collect();
            let mut normal = [0.; 3];
            let mut boundary = BTreeMap::<[u32; 2], (u32, u32, u32)>::new();
            let mut selected_vertices = BTreeSet::new();
            for (i, t) in m.indices.chunks_exact(3).enumerate() {
                if selection.mode != "face" || !faces.contains(&i) {
                    continue;
                }
                let a = m.positions[t[0] as usize];
                let b = m.positions[t[1] as usize];
                let c = m.positions[t[2] as usize];
                let n = math::cross(
                    std::array::from_fn(|k| b[k] - a[k]),
                    std::array::from_fn(|k| c[k] - a[k]),
                );
                for k in 0..3 {
                    normal[k] += n[k];
                }
                for k in 0..3 {
                    let a = t[k];
                    let b = t[(k + 1) % 3];
                    selected_vertices.insert(a);
                    selected_vertices.insert(b);
                    let mut e = [ids[a as usize], ids[b as usize]];
                    e.sort();
                    let v = boundary.entry(e).or_insert((a, b, 0));
                    v.2 += 1;
                }
            }
            if selection.mode == "edge" {
                for &[a, b] in &selection.edges {
                    let mut e = [ids[a as usize], ids[b as usize]];
                    e.sort();
                    boundary.insert(e, (a, b, 1));
                    selected_vertices.extend([a, b]);
                    for i in [a, b] {
                        if let Some(n) = m.normals.get(i as usize) {
                            for k in 0..3 {
                                normal[k] += n[k];
                            }
                        }
                    }
                }
            }
            normal = math::normalize(normal);
            if math::dot(normal, normal) < 0.5 {
                return Err(invalid("Selection has no usable extrusion normal"));
            }
            if boundary.values().any(|e| e.2 > 2) {
                return Err(invalid("Non-manifold selection; choose a manifold region"));
            }
            let boundary: Vec<_> = boundary.values().filter(|e| e.2 == 1).copied().collect();
            if boundary.is_empty() && inset {
                return Err(invalid("Inset needs a region with a boundary"));
            }
            let mut inward = BTreeMap::<u32, Vec<[f32; 3]>>::new();
            if inset {
                for &(a, b, _) in &boundary {
                    let d = std::array::from_fn(|k| {
                        m.positions[b as usize][k] - m.positions[a as usize][k]
                    });
                    let n = math::normalize(math::cross(normal, d));
                    inward.entry(ids[a as usize]).or_default().push(n);
                    inward.entry(ids[b as usize]).or_default().push(n);
                }
            }
            let mut remap = BTreeMap::new();
            for i in selected_vertices {
                let j = duplicate(m, i as usize);
                let delta = if inset {
                    if let Some(normals) = inward.get(&ids[i as usize]) {
                        let v: [f32; 3] =
                            std::array::from_fn(|k| normals.iter().map(|n| n[k]).sum());
                        let v = math::normalize(v);
                        let dot = math::dot(v, normals[0]);
                        if dot < 1e-4 {
                            return Err(invalid("Inset boundary is degenerate"));
                        }
                        v.map(|v| v * amount / dot)
                    } else {
                        [0.; 3]
                    }
                } else {
                    normal.map(|n| n * amount)
                };
                for (k, d) in delta.iter().enumerate() {
                    m.positions[j as usize][k] += d;
                }
                remap.insert(i, j);
            }
            for (i, t) in m.indices.chunks_exact_mut(3).enumerate() {
                if selection.mode == "face" && faces.contains(&i) {
                    for v in t {
                        *v = remap[v];
                    }
                }
            }
            for (a, b, _) in boundary {
                let c = remap[&b];
                let d = remap[&a];
                m.indices.extend([a, b, c, a, c, d]);
            }
            compact(m);
        }
    }
    model::recompute_normals(m)?;
    model::validate_mesh(m)?;
    Ok(())
}
