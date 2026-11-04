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
