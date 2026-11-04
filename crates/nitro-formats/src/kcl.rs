//! MKDS fixed-point KCL. Layout reference: Mario Kart Toolbox (MIT).
use crate::{
    error,
    math::{cross, dot},
    nkm::{fx, vec3},
    slice, u16le, u32le, Result,
};
#[derive(Clone, Debug)]
pub struct Prism {
    pub height: f32,
    pub position: u16,
    pub normals: [u16; 4],
    pub attribute: u16,
}
#[derive(Clone, Debug)]
pub struct OctreeNode {
    pub offset: u32,
    pub children: Vec<OctreeNode>,
    pub prisms: Vec<u16>,
}
#[derive(Clone, Debug)]
pub struct Kcl {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub prisms: Vec<Prism>,
    pub octree: Vec<OctreeNode>,
    pub minimum: [f32; 3],
    pub thickness: f32,
    pub radius: f32,
    raw: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Triangle {
    pub positions: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub attribute: u16,
}
impl Kcl {
    /// Generate fixed-point prisms and a conservative spatial octree. Lists
    /// include triangles whose expanded AABB touches the cell, so collision
    /// candidates are never lost at subdivision boundaries.
    pub fn from_triangles(triangles: &[Triangle]) -> Result<Self> {
        use crate::{g3d::align, math::normalize, put32};
        use std::collections::BTreeMap;
        if triangles.is_empty() || triangles.len() > 65534 {
            return Err(error(0, "KCL needs 1–65,534 non-degenerate triangles"));
        }
        let fx32 = |v: f32| -> Result<i32> {
            let q = (v * 4096.).round();
            if !q.is_finite() || !(-2147483648. ..2147483648.).contains(&q) {
                return Err(error(0, "collision coordinate exceeds fx32"));
            }
            Ok(q as i32)
        };
        let mut positions = BTreeMap::<[i32; 3], u16>::new();
        let mut normals = BTreeMap::<[i16; 3], u16>::new();
        let mut prism_bytes = vec![];
        let mut bounds = vec![];
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for (i, t) in triangles.iter().enumerate() {
            let [a, b, c] = t.positions;
            let ab = std::array::from_fn(|k| b[k] - a[k]);
            let ac = std::array::from_fn(|k| c[k] - a[k]);
            let f = normalize(cross(ab, ac));
            if dot(f, f) < 0.5 || t.positions.iter().flatten().any(|v| !v.is_finite()) {
                return Err(error(i, "degenerate/non-finite collision triangle"));
            }
            let edge1 = normalize(cross(f, ac));
            let edge2 = normalize(cross(ab, f));
            let edge3 = normalize(cross(f, std::array::from_fn(|k| b[k] - c[k])));
            let q = a.map(fx32);
            let p = [
                q[0].as_ref()
                    .copied()
                    .map_err(|_| error(i, "collision position exceeds fx32"))?,
                q[1].as_ref()
                    .copied()
                    .map_err(|_| error(i, "collision position exceeds fx32"))?,
                q[2].as_ref()
                    .copied()
                    .map_err(|_| error(i, "collision position exceeds fx32"))?,
            ];
            let next = positions.len();
            if next >= 65535 {
                return Err(error(i, "too many collision positions"));
            }
            let pos = *positions.entry(p).or_insert(next as u16);
            let mut ids = [0; 4];
            for (k, n) in [f, edge1, edge2, edge3].iter().enumerate() {
                let key = n.map(|v| (v * 4096.).round() as i16);
                let next = normals.len();
                if next >= 65535 {
                    return Err(error(i, "too many collision normals"));
                }
                ids[k] = *normals.entry(key).or_insert(next as u16);
            }
            let quantized = edge3.map(|v| (v * 4096.).round() / 4096.);
            let height = dot(ab, quantized);
            if height <= 0. {
                return Err(error(
                    i,
                    "collision triangle is too small for fixed-point normals",
                ));
            }
            prism_bytes.extend(fx32(height)?.to_le_bytes());
            prism_bytes.extend(pos.to_le_bytes());
            for id in ids {
                prism_bytes.extend(id.to_le_bytes());
            }
            prism_bytes.extend(t.attribute.to_le_bytes());
            let lo = std::array::from_fn(|k| {
                t.positions
                    .iter()
                    .map(|p| p[k])
                    .fold(f32::INFINITY, f32::min)
                    - 55.
            });
            let hi = std::array::from_fn(|k| {
                t.positions
                    .iter()
                    .map(|p| p[k])
                    .fold(f32::NEG_INFINITY, f32::max)
                    + 55.
            });
            for k in 0..3 {
                minimum[k] = minimum[k].min(lo[k].floor());
                maximum[k] = maximum[k].max(hi[k].ceil());
            }
            bounds.push((lo, hi));
        }
        let span = (0..3)
            .map(|k| (maximum[k] - minimum[k]).ceil() as u32)
            .max()
            .unwrap()
            .max(1);
        let cube = span
            .checked_next_power_of_two()
            .ok_or_else(|| error(0, "collision bounds overflow"))?;
        if cube > 524288 {
            return Err(error(0, "collision bounds exceed DS range"));
        }
        let mut b = vec![0; 60];
        put32(&mut b, 0, 60);
        for v in minimum {
            let _ = fx32(v)?;
        }
        let mut p = vec![[0; 3]; positions.len()];
        for (key, id) in positions {
            p[id as usize] = key;
        }
        for v in p {
            for q in v {
                b.extend(q.to_le_bytes());
            }
        }
        let n = b.len();
        put32(&mut b, 4, n as u32);
        let mut ns = vec![[0; 3]; normals.len()];
        for (key, id) in normals {
            ns[id as usize] = key;
        }
        for v in ns {
            for q in v {
                b.extend(q.to_le_bytes());
            }
        }
        align(&mut b, 4);
        let t = b.len();
        put32(&mut b, 8, (t - 16) as u32);
        b.extend(prism_bytes);
        let o = b.len();
        put32(&mut b, 12, o as u32);
        put32(&mut b, 16, 30 * 4096);
        put32(&mut b, 56, 25 * 4096);
        for (k, value) in minimum.iter().enumerate() {
            put32(&mut b, 20 + k * 4, fx32(*value)? as u32);
            put32(&mut b, 32 + k * 4, !(cube - 1));
        }
        put32(&mut b, 44, cube.ilog2());
        struct Tree<'a> {
            bytes: Vec<u8>,
            bounds: &'a [([f32; 3], [f32; 3])],
            work: usize,
        }
        impl Tree<'_> {
            fn node(
                &mut self,
                base: usize,
                at: usize,
                lo: [f32; 3],
                size: f32,
                ids: &[u16],
                depth: usize,
            ) -> Result<()> {
                if self.work > 500_000 {
                    return Err(error(0, "collision octree exceeds work limit"));
                }
                self.work += ids.len() + 1;
                if ids.len() <= 16 || size <= 32. || depth >= 8 {
                    let data = self.bytes.len();
                    crate::put32(&mut self.bytes, at, 0x80000000 | ((data - 2 - base) as u32));
                    for &id in ids {
                        self.bytes.extend((id + 1).to_le_bytes());
                    }
                    self.bytes.extend(0u16.to_le_bytes());
                    return Ok(());
                }
                let half = size / 2.;
                let children = (0..8)
                    .map(|i| {
                        let child = std::array::from_fn(|k| {
                            lo[k] + if i & (1 << k) != 0 { half } else { 0. }
                        });
                        let list = ids
                            .iter()
                            .copied()
                            .filter(|&id| {
                                let (min, max) = self.bounds[id as usize];
                                (0..3).all(|k| min[k] <= child[k] + half && max[k] >= child[k])
                            })
                            .collect::<Vec<_>>();
                        (child, list)
                    })
                    .collect::<Vec<_>>();
                if children.iter().map(|(_, ids)| ids.len()).sum::<usize>() >= ids.len() * 6 {
                    let data = self.bytes.len();
                    crate::put32(&mut self.bytes, at, 0x80000000 | ((data - 2 - base) as u32));
                    for &id in ids {
                        self.bytes.extend((id + 1).to_le_bytes());
                    }
                    self.bytes.extend(0u16.to_le_bytes());
                    return Ok(());
                }
                crate::g3d::align(&mut self.bytes, 4);
                let target = self.bytes.len();
                crate::put32(&mut self.bytes, at, (target - base) as u32);
                self.bytes.resize(target + 32, 0);
                for (i, (child, list)) in children.iter().enumerate() {
                    self.node(target, target + i * 4, *child, half, list, depth + 1)?;
                }
                Ok(())
            }
        }
        let mut tree = Tree {
            bytes: vec![0; 4],
            bounds: &bounds,
            work: 0,
        };
        tree.node(
            0,
            0,
            minimum,
            cube as f32,
            &(0..triangles.len() as u16).collect::<Vec<_>>(),
            0,
        )?;
        b.extend(tree.bytes);
        Self::parse(&b)
    }
    /// Attribute edits preserve geometry and the original spatial tree.
    pub fn repaint(&mut self, triangles: &[usize], attribute: u16) -> Result<()> {
        if triangles.iter().any(|&i| i >= self.prisms.len()) {
            return Err(error(0, "collision selection outside prism table"));
        }
        let start = u32le(&self.raw, 8)? as usize + 16;
        for &i in triangles {
            self.prisms[i].attribute = attribute;
            crate::put16(&mut self.raw, start + i * 16 + 14, attribute);
        }
        Ok(())
    }
    pub fn parse(b: &[u8]) -> Result<Self> {
        slice(b, 0, 60)?;
        let p = u32le(b, 0)? as usize;
        let n = u32le(b, 4)? as usize;
        let t = u32le(b, 8)? as usize + 16;
        let o = u32le(b, 12)? as usize;
        if !(60 <= p
            && p <= n
            && n <= t
            && t <= o
            && o <= b.len()
            && (n - p).is_multiple_of(12)
            && (o - t).is_multiple_of(16))
            || (t - n) % 6 > 2
        {
            return Err(error(0, "invalid KCL table offsets"));
        }
        let positions = (p..n)
            .step_by(12)
            .map(|a| vec3(b, a))
            .collect::<Result<Vec<_>>>()?;
        let normals = (0..(t - n) / 6)
            .map(|i| {
                let a = n + i * 6;
                Ok([
                    u16le(b, a)? as i16 as f32 / 4096.,
                    u16le(b, a + 2)? as i16 as f32 / 4096.,
                    u16le(b, a + 4)? as i16 as f32 / 4096.,
                ])
            })
            .collect::<Result<Vec<_>>>()?;
        let mut prisms = vec![];
        for at in (t..o).step_by(16) {
            let position = u16le(b, at + 4)?;
            let normals_idx = [
                u16le(b, at + 6)?,
                u16le(b, at + 8)?,
                u16le(b, at + 10)?,
                u16le(b, at + 12)?,
            ];
            if position as usize >= positions.len()
                || normals_idx.iter().any(|i| *i as usize >= normals.len())
            {
                return Err(error(at, "KCL prism index outside table"));
            }
            prisms.push(Prism {
                height: fx(b, at)?,
                position,
                normals: normals_idx,
                attribute: u16le(b, at + 14)?,
            });
        }
        let shift = u32le(b, 44)?;
        if shift > 31 {
            return Err(error(44, "invalid KCL block shift"));
        }
        let mut roots = 1usize;
        for a in [32, 36, 40] {
            let mask = u32le(b, a)?;
            let count = ((!mask as u64) >> shift) + 1;
            roots = roots
                .checked_mul(count as usize)
                .ok_or_else(|| error(a, "octree root count overflow"))?;
        }
        if roots > (b.len() - o) / 4 {
            return Err(error(o, "truncated octree roots"));
        }
        let tree = slice(b, o, b.len() - o)?;
        let mut budget = 1_000_000usize;
        fn node(
            b: &[u8],
            base: usize,
            at: usize,
            depth: usize,
            limit: usize,
            budget: &mut usize,
        ) -> Result<OctreeNode> {
            if depth > 32 || *budget == 0 {
                return Err(error(at, "KCL octree depth/work limit"));
            }
            *budget -= 1;
            let v = u32le(b, at)?;
            let target = base + (v & 0x7fffffff) as usize;
            let mut out = OctreeNode {
                offset: at as u32,
                children: vec![],
                prisms: vec![],
            };
            if v >> 31 != 0 {
                let mut p = target + 2;
                loop {
                    if *budget == 0 {
                        return Err(error(p, "KCL octree work limit"));
                    }
                    *budget -= 1;
                    let id = u16le(b, p)?;
                    p += 2;
                    if id == 0 {
                        break;
                    }
                    if id as usize > limit {
                        return Err(error(p - 2, "octree prism outside table"));
                    }
                    out.prisms.push(id - 1)
                }
            } else {
                if target <= base {
                    return Err(error(at, "cyclic KCL octree"));
                }
                slice(b, target, 32)?;
                for i in 0..8 {
                    out.children
                        .push(node(b, target, target + i * 4, depth + 1, limit, budget)?);
                }
            }
            Ok(out)
        }
        let octree = (0..roots)
            .map(|i| node(tree, 0, i * 4, 0, prisms.len(), &mut budget))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            positions,
            normals,
            prisms,
            octree,
            minimum: vec3(b, 20)?,
            thickness: fx(b, 16)?,
            radius: fx(b, 56)?,
            raw: b.to_vec(),
        })
    }
    pub fn write(&self) -> Vec<u8> {
        self.raw.clone()
    }
    pub fn triangles(&self) -> Result<Vec<Triangle>> {
        self.prisms
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let a = self.positions[p.position as usize];
                let [f, e1, e2, e3] = p.normals.map(|i| self.normals[i as usize]);
                let u = cross(e1, f);
                let v = cross(e2, f);
                let du = dot(u, e3);
                let dv = dot(v, e3);
                if du.abs() < 1e-10 || dv.abs() < 1e-10 {
                    return Err(error(i * 16, "degenerate KCL prism"));
                }
                let b = std::array::from_fn(|k| a[k] + v[k] * p.height / dv);
                let c = std::array::from_fn(|k| a[k] + u[k] * p.height / du);
                Ok(Triangle {
                    positions: [a, b, c],
                    normal: f,
                    attribute: p.attribute,
                })
            })
            .collect()
    }
}
