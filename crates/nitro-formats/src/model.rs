//! BMD0/MDL0 and GX decoding, independently implemented from apicula's 0BSD layout documentation.
pub use crate::model_edit::*;
use crate::{
    error,
    math::{self, Mat4, IDENTITY},
    nkm::{fx, vec3},
    slice,
    texture::{dictionary, Nsbtx},
    u16le, u32le, Result,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}
impl Bone {
    pub fn matrix(&self) -> Mat4 {
        math::trs(self.translation, self.rotation, self.scale)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub texture: Option<String>,
    pub palette: Option<String>,
    pub diffuse: [f32; 4],
    pub width: u16,
    pub height: u16,
    pub repeat: [bool; 2],
    pub mirror: [bool; 2],
    pub double_sided: bool,
    /// Texture matrix (NitroSystem, Maya rules): applied by the DS only when `tex_mode` is 1
    /// (texcoord source). Retail tracks use the scale to tile road and wall textures.
    #[serde(default)]
    pub tex_mode: u8,
    #[serde(default = "one2")]
    pub tex_scale: [f32; 2],
    /// sin, cos
    #[serde(default = "rot0")]
    pub tex_rotate: [f32; 2],
    #[serde(default)]
    pub tex_translate: [f32; 2],
}
fn one2() -> [f32; 2] {
    [1., 1.]
}
fn rot0() -> [f32; 2] {
    [0., 1.]
}
impl Material {
    /// True when the DS transforms this material's texture coordinates.
    pub fn has_tex_matrix(&self) -> bool {
        self.tex_mode == 1
            && (self.tex_scale != [1., 1.]
                || self.tex_rotate != [0., 1.]
                || self.tex_translate != [0., 0.])
    }
    /// Maps a normalised (s/width, t/height) coordinate the way the DS texture matrix does.
    /// Maya-style SRT: Maya's V axis runs bottom-up, the DS T axis top-down.
    pub fn transform_uv(&self, uv: [f32; 2]) -> [f32; 2] {
        if !self.has_tex_matrix() {
            return uv;
        }
        let [sx, sy] = self.tex_scale;
        let [sin, cos] = self.tex_rotate;
        let [tx, ty] = self.tex_translate;
        let (u, v) = (uv[0], 1. - uv[1]);
        let (u, v) = (u * sx, v * sy);
        let (u, v) = (u * cos - v * sin, u * sin + v * cos);
        [u + tx, 1. - (v + ty)]
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub material: usize,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
}
#[derive(Clone, Debug)]
pub struct Model {
    pub name: String,
    pub bones: Vec<Bone>,
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
    pub matrix_count: usize,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Nsbmd {
    pub models: Vec<Model>,
    pub textures: Option<Nsbtx>,
}
pub(crate) fn blocks<'a>(b: &'a [u8], magic: &[u8; 4]) -> Result<Vec<&'a [u8]>> {
    if slice(b, 0, 4)? != magic || u16le(b, 4)? != 0xfeff {
        return Err(error(0, "invalid Nitro 3D header"));
    }
    let b = slice(b, 0, u32le(b, 8)? as usize)?;
    let n = u16le(b, 14)? as usize;
    if u16le(b, 12)? < 16 {
        return Err(error(12, "invalid Nitro header length"));
    }
    slice(b, 16, n * 4)?;
    (0..n)
        .map(|i| {
            let at = u32le(b, 16 + i * 4)? as usize;
            slice(b, at, u32le(b, at + 4)? as usize)
        })
        .collect()
}
pub(crate) fn pivot(select: u16, neg: u16, a: f32, b: f32) -> Result<[f32; 9]> {
    if select > 8 {
        return Err(error(0, "invalid rotation pivot"));
    }
    let o = if neg & 1 == 0 { 1. } else { -1. };
    let c = if neg & 2 == 0 { b } else { -b };
    let d = if neg & 4 == 0 { a } else { -a };
    Ok(match select {
        0 => [o, 0., 0., 0., a, b, 0., c, d],
        1 => [0., o, 0., a, 0., b, c, 0., d],
        2 => [0., 0., o, a, b, 0., c, d, 0.],
        3 => [0., a, b, o, 0., 0., 0., c, d],
        4 => [a, 0., b, 0., o, 0., c, 0., d],
        5 => [a, b, 0., 0., 0., o, c, d, 0.],
        6 => [0., a, b, 0., c, d, o, 0., 0.],
        7 => [a, 0., b, c, 0., d, 0., o, 0.],
        _ => [a, b, 0., c, d, 0., 0., 0., o],
    })
}
fn bone(b: &[u8], name: String) -> Result<Bone> {
    let flags = u16le(b, 0)?;
    let mut at = 4;
    let mut translation = [0.; 3];
    let mut scale = [1.; 3];
    let mut r = [1., 0., 0., 0., 1., 0., 0., 0., 1.];
    if flags & 1 == 0 {
        translation = vec3(b, at)?;
        at += 12
    }
    let f16 = |p| -> Result<f32> { Ok(u16le(b, p)? as i16 as f32 / 4096.) };
    if flags & 8 != 0 {
        r = pivot((flags >> 4) & 15, flags >> 8, f16(at)?, f16(at + 2)?)?;
        at += 4
    } else if flags & 2 == 0 {
        r[0] = f16(2)?;
        for (i, v) in r.iter_mut().enumerate().skip(1) {
            *v = f16(at + (i - 1) * 2)?
        }
        at += 16
    }
    if flags & 4 == 0 {
        scale = vec3(b, at)?
    }
    Ok(Bone {
        name,
        parent: None,
        translation,
        rotation: math::quaternion(r),
        scale,
    })
}
impl Nsbmd {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let sections = blocks(b, b"BMD0")?;
        let mdl = sections
            .iter()
            .find(|b| b.starts_with(b"MDL0"))
            .ok_or_else(|| error(16, "missing MDL0"))?;
        let mut models = vec![];
        for (name, d) in dictionary(mdl, 8, 4)? {
            let at = u32le(&d, 0)? as usize;
            let m = slice(mdl, at, u32le(mdl, at)? as usize)?;
            models.push(parse_model(m, name)?)
        }
        let textures = if sections.iter().any(|b| b.starts_with(b"TEX0")) {
            Some(Nsbtx::parse(b)?)
        } else {
            None
        };
        Ok(Self { models, textures })
    }
}
#[derive(Clone)]
struct Matrix {
    m: Mat4,
    joints: [u16; 4],
    weights: [f32; 4],
}
impl Default for Matrix {
    fn default() -> Self {
        Self {
            m: IDENTITY,
            joints: [0; 4],
            weights: [0.; 4],
        }
    }
}
fn parse_model(b: &[u8], name: String) -> Result<Model> {
    slice(b, 0, 64)?;
    let sbc = u32le(b, 4)? as usize;
    let mats = u32le(b, 8)? as usize;
    let shapes = u32le(b, 12)? as usize;
    let binds = u32le(b, 16)? as usize;
    if !(64 <= sbc && sbc <= mats && mats <= shapes && shapes <= b.len()) {
        return Err(error(4, "invalid model offsets"));
    }
    let mut bones = vec![];
    for (name, d) in dictionary(b, 64, 4)? {
        let at = 64 + u32le(&d, 0)? as usize;
        bones.push(bone(slice(b, at, b.len().saturating_sub(at))?, name)?)
    }
    let mut materials = vec![];
    for (name, d) in dictionary(b, mats + 4, 4)? {
        let p = mats + u32le(&d, 0)? as usize;
        slice(b, p, 44)?;
        let color = u32le(b, p + 4)?;
        let attr = u32le(b, p + 12)?;
        let tex = u32le(b, p + 20)?;
        // Matrix parts beyond the record's own size are treated as absent.
        let size = (u16le(b, p + 2)? as usize).max(44);
        let mut flags = u16le(b, p + 30)?;
        let need = |f: u16| {
            44 + if f & 2 == 0 { 8 } else { 0 }
                + if f & 4 == 0 { 4 } else { 0 }
                + if f & 8 == 0 { 8 } else { 0 }
        };
        if need(flags) > size {
            flags |= 14
        }
        let mut extra = p + 44;
        let mut fx32 = || -> Result<f32> {
            let v = u32le(b, extra)? as i32 as f32 / 4096.;
            extra += 4;
            Ok(v)
        };
        let tex_scale = if flags & 2 == 0 {
            [fx32()?, fx32()?]
        } else {
            [1., 1.]
        };
        let tex_rotate = if flags & 4 == 0 {
            let v = u32le(b, extra)?;
            extra += 4;
            [
                (v & 0xffff) as u16 as i16 as f32 / 4096.,
                (v >> 16) as u16 as i16 as f32 / 4096.,
            ]
        } else {
            [0., 1.]
        };
        let mut fx32 = || -> Result<f32> {
            let v = u32le(b, extra)? as i32 as f32 / 4096.;
            extra += 4;
            Ok(v)
        };
        let tex_translate = if flags & 8 == 0 {
            [fx32()?, fx32()?]
        } else {
            [0., 0.]
        };
        materials.push(Material {
            name,
            texture: None,
            palette: None,
            diffuse: [
                (color & 31) as f32 / 31.,
                ((color >> 5) & 31) as f32 / 31.,
                ((color >> 10) & 31) as f32 / 31.,
                ((attr >> 16) & 31) as f32 / 31.,
            ],
            width: u16le(b, p + 32)?,
            height: u16le(b, p + 34)?,
            repeat: [tex & (1 << 16) != 0, tex & (1 << 17) != 0],
            mirror: [tex & (1 << 18) != 0, tex & (1 << 19) != 0],
            double_sided: attr & 0xc0 == 0xc0,
            tex_mode: (tex >> 30) as u8,
            tex_scale,
            tex_rotate,
            tex_translate,
        });
    }
    for (off, is_texture) in [(0, true), (2, false)] {
        let p = mats + u16le(b, mats + off)? as usize;
        for (name, d) in dictionary(b, p, 4)? {
            let at = mats + u16le(&d, 0)? as usize;
            for id in slice(b, at, d[2] as usize)? {
                let material = materials
                    .get_mut(*id as usize)
                    .ok_or_else(|| error(at, "material pairing index out of range"))?;
                if is_texture {
                    material.texture = Some(name.clone())
                } else {
                    material.palette = Some(name.clone())
                }
            }
        }
    }
    let mut pieces = vec![];
    for (name, d) in dictionary(b, shapes, 4)? {
        let p = shapes + u32le(&d, 0)? as usize;
        if u16le(b, p + 2)? != 16 {
            return Err(error(p, "invalid shape header"));
        }
        pieces.push((
            name,
            slice(b, p + u32le(b, p + 8)? as usize, u32le(b, p + 12)? as usize)?,
        ));
    }
    let mut inverses = vec![];
    if binds > 0 && binds <= b.len() {
        for i in 0..bones.len() {
            let at = binds + i * 84;
            if at + 84 > b.len() {
                break;
            }
            let mut m = IDENTITY;
            for c in 0..4 {
                for r in 0..3 {
                    m[c * 4 + r] = fx(b, at + (c * 3 + r) * 4)?
                }
            }
            inverses.push(m)
        }
    }
    let mut stack = vec![Matrix::default(); 32];
    let mut current = Matrix::default();
    let mut material = 0;
    let mut meshes = vec![];
    let mut at = sbc;
    let mut matrix_count = 0;
    let mut warnings = vec![];
    while at < mats {
        let op = b[at];
        at += 1;
        let n = match op {
            0 | 1 | 0x0b | 0x2b | 0x40 | 0x80 => 0,
            2 | 0x0c | 0x0d | 0x47 => 2,
            3..=5 | 7 | 8 | 0x24 | 0x44 => 1,
            6 => 3,
            0x26 | 0x46 => 4,
            0x66 => 5,
            9 => 2 + 3 * slice(b, at + 1, 1)?[0] as usize,
            _ => return Err(error(at - 1, format!("unknown SBC command {op:02x}"))),
        };
        let p = slice(b, at, n)?;
        at += n;
        match op {
            1 => break,
            3 => {
                current = stack
                    .get(p[0] as usize)
                    .ok_or_else(|| error(at, "matrix slot out of range"))?
                    .clone();
                matrix_count = matrix_count.max(p[0] as usize + 1)
            }
            4 | 0x24 | 0x44 => {
                material = p[0] as usize;
                if material >= materials.len() {
                    return Err(error(at, "material out of range"));
                }
            }
            5 => {
                let (name, cmds) = pieces
                    .get(p[0] as usize)
                    .ok_or_else(|| error(at, "shape out of range"))?;
                let mut mesh = decode_gx(cmds, current.clone(), &mut stack, &mut matrix_count)?;
                mesh.name = name.clone();
                mesh.material = material;
                if let Some(m) = materials.get(material) {
                    for uv in &mut mesh.uvs {
                        uv[0] /= m.width.max(1) as f32;
                        uv[1] /= m.height.max(1) as f32;
                    }
                }
                meshes.push(mesh)
            }
            6 | 0x26 | 0x46 | 0x66 => {
                if matches!(op, 0x46 | 0x66) {
                    let i = p[if op == 0x66 { 4 } else { 3 }] as usize;
                    current = stack
                        .get(i)
                        .ok_or_else(|| error(at, "matrix slot out of range"))?
                        .clone();
                }
                let i = p[0] as usize;
                let parent = p[1] as usize;
                if i >= bones.len() {
                    return Err(error(at, "node out of range"));
                }
                if parent != i && parent < bones.len() {
                    bones[i].parent = Some(parent)
                }
                current.m = math::mul(current.m, bones[i].matrix());
                current.joints = [i as u16, 0, 0, 0];
                current.weights = [1., 0., 0., 0.];
                if matches!(op, 0x26 | 0x66) {
                    let slot = p[3] as usize;
                    *stack
                        .get_mut(slot)
                        .ok_or_else(|| error(at, "matrix slot out of range"))? = current.clone();
                    matrix_count = matrix_count.max(slot + 1)
                }
            }
            9 => {
                let mut m = [0.; 16];
                let mut joints = [0; 4];
                let mut weights = [0.; 4];
                if p[1] > 4 {
                    warnings.push("More than four skin influences; strongest four exported".into())
                }
                let mut terms = vec![];
                for term in p[2..].chunks_exact(3) {
                    let slot = stack
                        .get(term[0] as usize)
                        .ok_or_else(|| error(at, "skin slot out of range"))?;
                    let inv = *inverses
                        .get(term[1] as usize)
                        .ok_or_else(|| error(at, "missing inverse bind matrix"))?;
                    let weight = term[2] as f32 / 256.;
                    let product = math::mul(slot.m, inv);
                    for k in 0..16 {
                        m[k] += product[k] * weight
                    }
                    terms.push((term[1] as u16, weight));
                }
                terms.sort_by(|a, b| b.1.total_cmp(&a.1));
                let sum: f32 = terms.iter().take(4).map(|t| t.1).sum();
                for (i, (j, w)) in terms.iter().take(4).enumerate() {
                    joints[i] = *j;
                    weights[i] = *w / sum.max(f32::EPSILON)
                }
                current = Matrix { m, joints, weights };
                let slot = p[0] as usize;
                *stack
                    .get_mut(slot)
                    .ok_or_else(|| error(at, "skin destination out of range"))? = current.clone();
                matrix_count = matrix_count.max(slot + 1);
            }
            0x0b | 0x2b => {
                let s = fx(b, if op == 0x0b { 32 } else { 36 })?;
                current.m = math::mul(current.m, math::trs([0.; 3], [0., 0., 0., 1.], [s; 3]));
            }
            7 | 8 | 0x47 => warnings.push("Billboard uses its rest orientation".into()),
            _ => {}
        }
    }
    // Reject cyclic parent graphs before exporters or animation evaluation visit them.
    for i in 0..bones.len() {
        let mut p = Some(i);
        for _ in 0..=bones.len() {
            p = p.and_then(|j| bones[j].parent);
            if p == Some(i) {
                return Err(error(sbc, "cyclic bone hierarchy"));
            }
            if p.is_none() {
                break;
            }
        }
    }
    warnings.sort();
    warnings.dedup();
    Ok(Model {
        name,
        bones,
        materials,
        meshes,
        matrix_count,
        warnings,
    })
}
fn signed(v: u32, bits: u32) -> f32 {
    ((v << (32 - bits)) as i32 >> (32 - bits)) as f32
}
fn decode_gx(
    b: &[u8],
    mut current: Matrix,
    stack: &mut [Matrix],
    matrix_count: &mut usize,
) -> Result<Mesh> {
    let mut out = Mesh::default();
    let mut at = 0;
    let mut position = [0.; 3];
    let mut normal = [0., 1., 0.];
    let mut uv = [0.; 2];
    let mut color = [1.; 4];
    let mut mode = 0;
    let mut primitive = vec![];
    let mut active = false;
    while at < b.len() {
        let ops = slice(b, at, 4)?;
        at += 4;
        for &op in ops {
            let count = match op {
                0 | 0x11 | 0x15 | 0x41 => 0,
                0x10
                | 0x12..=0x14
                | 0x20..=0x22
                | 0x24..=0x2b
                | 0x30..=0x34
                | 0x40
                | 0x50
                | 0x60
                | 0x72 => 1,
                0x23 | 0x71 => 2,
                0x1b | 0x1c | 0x70 => 3,
                0x1a => 9,
                0x17 | 0x19 => 12,
                0x16 | 0x18 => 16,
                _ => return Err(error(at - 4, format!("unknown GX command {op:02x}"))),
            };
            let p = slice(b, at, count * 4)?;
            at += count * 4;
            let word = if count > 0 { u32le(p, 0)? } else { 0 };
            match op {
                0x14 => {
                    let slot = (word & 31) as usize;
                    current = stack[slot].clone();
                    *matrix_count = (*matrix_count).max(slot + 1)
                }
                0x13 => {
                    let slot = (word & 31) as usize;
                    stack[slot] = current.clone();
                    *matrix_count = (*matrix_count).max(slot + 1)
                }
                0x15 => current.m = IDENTITY,
                0x16..=0x1a => {
                    let mut m = IDENTITY;
                    let rows = if matches!(op, 0x17 | 0x19 | 0x1a) {
                        3
                    } else {
                        4
                    };
                    for i in 0..count {
                        m[i / rows * 4 + i % rows] = fx(p, i * 4)?
                    }
                    current.m = if op <= 0x17 {
                        m
                    } else {
                        math::mul(current.m, m)
                    }
                }
                0x1b | 0x1c => {
                    let v = vec3(p, 0)?;
                    let m = math::trs(
                        if op == 0x1c { v } else { [0.; 3] },
                        [0., 0., 0., 1.],
                        if op == 0x1b { v } else { [1.; 3] },
                    );
                    current.m = math::mul(current.m, m)
                }
                0x20 => {
                    color = [
                        (word & 31) as f32 / 31.,
                        ((word >> 5) & 31) as f32 / 31.,
                        ((word >> 10) & 31) as f32 / 31.,
                        1.,
                    ]
                }
                0x21 => {
                    normal = [
                        signed(word, 10) / 512.,
                        signed(word >> 10, 10) / 512.,
                        signed(word >> 20, 10) / 512.,
                    ]
                }
                0x22 => {
                    uv = [
                        word as u16 as i16 as f32 / 16.,
                        (word >> 16) as u16 as i16 as f32 / 16.,
                    ]
                }
                0x40 => {
                    mode = word & 3;
                    primitive.clear();
                    active = true
                }
                0x41 => active = false,
                0x23..=0x28 => {
                    match op {
                        0x23 => {
                            position = [
                                word as u16 as i16 as f32 / 4096.,
                                (word >> 16) as u16 as i16 as f32 / 4096.,
                                u16le(p, 4)? as i16 as f32 / 4096.,
                            ]
                        }
                        0x24 => {
                            position = [
                                signed(word, 10) / 64.,
                                signed(word >> 10, 10) / 64.,
                                signed(word >> 20, 10) / 64.,
                            ]
                        }
                        0x25 => {
                            position[0] = word as u16 as i16 as f32 / 4096.;
                            position[1] = (word >> 16) as u16 as i16 as f32 / 4096.
                        }
                        0x26 => {
                            position[0] = word as u16 as i16 as f32 / 4096.;
                            position[2] = (word >> 16) as u16 as i16 as f32 / 4096.
                        }
                        0x27 => {
                            position[1] = word as u16 as i16 as f32 / 4096.;
                            position[2] = (word >> 16) as u16 as i16 as f32 / 4096.
                        }
                        _ => {
                            for (i, v) in position.iter_mut().enumerate() {
                                *v += signed(word >> (i * 10), 10) / 4096.
                            }
                        }
                    }
                    if !active {
                        return Err(error(at, "GX vertex outside primitive"));
                    }
                    if out.positions.len() >= 1_000_000 {
                        return Err(error(at, "model vertex limit"));
                    }
                    let id = out.positions.len() as u32;
                    out.positions.push(math::point(current.m, position));
                    out.normals
                        .push(math::normalize(math::vector(current.m, normal)));
                    out.uvs.push(uv);
                    out.colors.push(color);
                    out.joints.push(current.joints);
                    out.weights.push(current.weights);
                    primitive.push(id);
                    let n = primitive.len();
                    let tri = match mode {
                        0 if n % 3 == 0 => vec![primitive[n - 3], primitive[n - 2], id],
                        1 if n % 4 == 0 => vec![
                            primitive[n - 4],
                            primitive[n - 3],
                            primitive[n - 2],
                            primitive[n - 4],
                            primitive[n - 2],
                            id,
                        ],
                        2 if n >= 3 => {
                            if n % 2 == 1 {
                                vec![primitive[n - 3], primitive[n - 2], id]
                            } else {
                                vec![primitive[n - 2], primitive[n - 3], id]
                            }
                        }
                        3 if n >= 4 && n % 2 == 0 => vec![
                            primitive[n - 4],
                            primitive[n - 3],
                            id,
                            primitive[n - 4],
                            id,
                            primitive[n - 2],
                        ],
                        _ => vec![],
                    };
                    out.indices.extend(tri);
                }
                _ => {}
            }
        }
    }
    Ok(out)
}
