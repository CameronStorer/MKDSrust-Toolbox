//! NSBMD authoring. Original encoder using the public Nitro/GX layouts.
use crate::{
    error,
    g3d::{align, container, dictionary},
    math::{self, IDENTITY},
    model::{Model, Nsbmd},
    put16, put32, Result,
};
fn offset_dictionary(names: &[String], bodies: &[Vec<u8>], base: usize) -> Result<Vec<u8>> {
    let dummy: Vec<_> = names.iter().map(|n| (n.clone(), vec![0; 4])).collect();
    let mut at = base + dictionary(&dummy, 4)?.len();
    let entries = names
        .iter()
        .zip(bodies)
        .map(|(n, b)| {
            let off = at;
            at += b.len();
            (n.clone(), (off as u32).to_le_bytes().to_vec())
        })
        .collect::<Vec<_>>();
    let mut b = dictionary(&entries, 4)?;
    for body in bodies {
        b.extend(body);
    }
    Ok(b)
}
fn fixed(x: f32) -> Result<u32> {
    let x = (x * 4096.).round();
    if !x.is_finite() || x < i32::MIN as f32 || x >= 2147483648. {
        return Err(error(0, "model coordinate exceeds fx32"));
    }
    Ok(x as i32 as u32)
}
fn gx(commands: &[(u8, Vec<u32>)]) -> Vec<u8> {
    let mut b = vec![];
    for batch in commands.chunks(4) {
        let mut ops = [0; 4];
        for (i, (op, _)) in batch.iter().enumerate() {
            ops[i] = *op;
        }
        b.extend(ops);
        for (_, args) in batch {
            for a in args {
                b.extend(a.to_le_bytes());
            }
        }
    }
    b
}
impl Nsbmd {
    pub fn write(&self) -> Result<Vec<u8>> {
        self.write_with_strips(false)
    }
    /// Emit connected triangle strips where the source topology permits them.
    pub fn write_with_strips(&self, strips: bool) -> Result<Vec<u8>> {
        let mut mdl = b"MDL0\0\0\0\0".to_vec();
        let bodies = self
            .models
            .iter()
            .map(|m| encode(m, self.textures.as_ref(), strips))
            .collect::<Result<Vec<_>>>()?;
        mdl.extend(offset_dictionary(
            &self
                .models
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<_>>(),
            &bodies,
            8,
        )?);
        let n = mdl.len();
        put32(&mut mdl, 4, n as u32);
        let mut blocks = vec![mdl];
        if let Some(textures) = &self.textures {
            for s in crate::model::blocks(&textures.write(), b"BTX0")? {
                if s.starts_with(b"TEX0") {
                    blocks.push(s.to_vec());
                }
            }
        }
        Ok(container(b"BMD0", &blocks))
    }
}
fn encode(
    model: &Model,
    textures: Option<&crate::texture::Nsbtx>,
    strips: bool,
) -> Result<Vec<u8>> {
    if model.bones.is_empty()
        || model.bones.len() > 31
        || model.meshes.len() > 255
        || model.materials.is_empty()
        || model.materials.len() > 255
    {
        return Err(error(
            0,
            "model requires 1–31 bones, 1–255 materials and at most 255 meshes",
        ));
    }
    let mut world = vec![];
    let mut nodes = vec![];
    for (i, bone) in model.bones.iter().enumerate() {
        if bone.parent.is_some_and(|p| p >= i) {
            return Err(error(i, "bones must be ordered parent before child"));
        }
        let local = bone.matrix();
        let m = math::mul(bone.parent.map_or(IDENTITY, |p| world[p]), local);
        world.push(m);
        let r = math::trs([0.; 3], bone.rotation, [1.; 3]);
        let mut n = vec![0; 4];
        put16(&mut n, 2, (r[0] * 4096.).round() as i16 as u16);
        for t in bone.translation {
            n.extend(fixed(t)?.to_le_bytes());
        }
        for k in [1, 2, 4, 5, 6, 8, 9, 10] {
            n.extend(((r[k] * 4096.).round() as i16).to_le_bytes());
        }
        for s in bone.scale {
            if s.abs() < 1e-8 {
                return Err(error(i, "zero bone scale"));
            }
            n.extend(fixed(s)?.to_le_bytes());
        }
        for s in bone.scale {
            n.extend(fixed(1. / s)?.to_le_bytes());
        }
        align(&mut n, 4);
        nodes.push(n);
    }
    let inverses = world
        .iter()
        .map(|m| math::inverse(*m).ok_or_else(|| error(0, "singular skeleton")))
        .collect::<Result<Vec<_>>>()?;
    let mut b = vec![0; 64];
    b.extend(offset_dictionary(
        &model
            .bones
            .iter()
            .map(|n| n.name.clone())
            .collect::<Vec<_>>(),
        &nodes,
        0,
    )?);
    let sbc = b.len();
    let mut script = vec![];
    for (i, bone) in model.bones.iter().enumerate() {
        if let Some(parent) = bone.parent {
            script.extend([0x66, i as u8, parent as u8, 0, i as u8, parent as u8]);
        } else {
            script.extend([0x26, i as u8, i as u8, 0, i as u8]);
        }
    }
    let mut shapes = vec![];
    let mut shape_names = vec![];
    let mut matrix_count = model.bones.len();
    // Position scale keeps VTX_16 inside its signed 4.12 range.
    let max = model
        .meshes
        .iter()
        .flat_map(|m| &m.positions)
        .flat_map(|p| inverses.iter().flat_map(move |inv| math::point(*inv, *p)))
        .map(|x| x.abs())
        .fold(1f32, f32::max);
    let scale = (max / 6.).max(1.).log2().ceil().exp2();
    for (mi, mesh) in model.meshes.iter().enumerate() {
        super::model::validate_mesh(mesh)?;
        let material = model
            .materials
            .get(mesh.material)
            .ok_or_else(|| error(mi, "mesh material out of range"))?;
        let mut combinations = Vec::<([u16; 4], [u16; 4])>::new();
        let mut cmds = vec![];
        let primitives = if strips {
            stripify(&mesh.indices)
        } else {
            vec![mesh.indices.clone()]
        };
        for primitive in primitives {
            cmds.push((0x40, vec![if strips { 2 } else { 0 }]));
            for &id in &primitive {
                let i = id as usize;
                let js = mesh.joints.get(i).copied().unwrap_or([0; 4]);
                let ws = mesh.weights.get(i).copied().unwrap_or([1., 0., 0., 0.]);
                if js
                    .iter()
                    .zip(ws)
                    .any(|(&j, w)| w > 0. && j as usize >= world.len())
                {
                    return Err(error(i, "skin joint out of range"));
                }
                let sum = ws.iter().sum::<f32>();
                if sum <= 0. || !sum.is_finite() || ws.iter().any(|w| !w.is_finite() || *w < 0.) {
                    return Err(error(i, "invalid skin weights"));
                }
                let mut weights = ws.map(|w| (w / sum * 256.).round() as u16);
                let total: i32 = weights.iter().map(|w| *w as i32).sum();
                let largest = weights
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, w)| *w)
                    .unwrap()
                    .0;
                weights[largest] = (weights[largest] as i32 + 256 - total) as u16;
                let rigid = weights.iter().position(|w| *w == 256);
                let (slot, local) = if let Some(k) = rigid {
                    let j = js[k] as usize;
                    (j, math::point(inverses[j], mesh.positions[i]))
                } else {
                    let key = (js, weights);
                    let index = if let Some(i) = combinations.iter().position(|v| *v == key) {
                        i
                    } else {
                        combinations.push(key);
                        combinations.len() - 1
                    };
                    let slot = world.len() + index;
                    if slot >= 31 {
                        return Err(error(
                            i,
                            "skin exceeds 31 usable matrix slots; split the mesh",
                        ));
                    }
                    (slot, mesh.positions[i])
                };
                matrix_count = matrix_count.max(slot + 1);
                cmds.push((0x14, vec![slot as u32]));
                if scale != 1. {
                    cmds.push((0x1b, vec![fixed(scale)?; 3]));
                }
                let n = mesh.normals.get(i).copied().unwrap_or([0., 1., 0.]);
                let n = if let Some(k) = rigid {
                    math::normalize(math::vector(inverses[js[k] as usize], n))
                } else {
                    n
                };
                let packed =
                    n.map(|v| ((v * 512.).round().clamp(-512., 511.) as i32 as u32) & 1023);
                cmds.push((0x21, vec![packed[0] | packed[1] << 10 | packed[2] << 20]));
                let c = mesh.colors.get(i).copied().unwrap_or([1.; 4]);
                let c = c.map(|v| (v.clamp(0., 1.) * 31.).round() as u32);
                cmds.push((0x20, vec![c[0] | c[1] << 5 | c[2] << 10]));
                let uv = mesh.uvs.get(i).copied().unwrap_or([0.; 2]);
                let u = (uv[0] * material.width as f32 * 16.).round();
                let v = (uv[1] * material.height as f32 * 16.).round();
                if [u, v]
                    .iter()
                    .any(|x| !x.is_finite() || *x < -32768. || *x > 32767.)
                {
                    return Err(error(i, "texture coordinates exceed GX range"));
                }
                cmds.push((
                    0x22,
                    vec![u as i16 as u16 as u32 | (v as i16 as u16 as u32) << 16],
                ));
                let p = local.map(|v| v / scale * 4096.);
                if p.iter()
                    .any(|v| !v.is_finite() || *v < -32768. || *v > 32767.)
                {
                    return Err(error(i, "vertex exceeds GX range"));
                }
                cmds.push((
                    0x23,
                    vec![
                        p[0].round() as i16 as u16 as u32
                            | (p[1].round() as i16 as u16 as u32) << 16,
                        p[2].round() as i16 as u16 as u32,
                    ],
                ));
            }
            cmds.push((0x41, vec![]));
        }
        for (j, (joints, weights)) in combinations.iter().enumerate() {
            let terms = weights.iter().filter(|&&w| w > 0).count();
            script.extend([9, (world.len() + j) as u8, terms as u8]);
            for (&joint, &w) in joints.iter().zip(weights).filter(|(_, w)| **w > 0) {
                script.extend([joint as u8, joint as u8, w as u8]);
            }
        }
        script.extend([4, mesh.material as u8, 5, mi as u8]);
        let dl = gx(&cmds);
        let mut shape = vec![0; 16];
        put16(&mut shape, 2, 16);
        put32(&mut shape, 4, 3);
        put32(&mut shape, 8, 16);
        put32(&mut shape, 12, dl.len() as u32);
        shape.extend(dl);
        shapes.push(shape);
        shape_names.push(format!("shape{mi}"));
    }
    script.push(1);
    b.extend(script);
    align(&mut b, 4);
    let mats = b.len();
    b.extend([0; 4]);
    let mut material_data = vec![];
    for m in &model.materials {
        let mut d = vec![0; 44];
        put16(&mut d, 2, 44);
        let c = m.diffuse.map(|v| (v.clamp(0., 1.) * 31.).round() as u32);
        put32(&mut d, 4, c[0] | c[1] << 5 | c[2] << 10 | 1 << 15);
        put32(
            &mut d,
            12,
            c[3] << 16 | if m.double_sided { 0xc0 } else { 0x80 },
        );
        put32(&mut d, 16, 0x3f1fffff);
        let param = textures
            .and_then(|t| {
                t.textures
                    .iter()
                    .find(|t| Some(&t.name) == m.texture.as_ref())
            })
            .map_or(0, |t| {
                ((t.width.ilog2() - 3) << 20)
                    | ((t.height.ilog2() - 3) << 23)
                    | ((t.format as u32) << 26)
                    | u32::from(t.transparent_zero) << 29
            });
        put32(
            &mut d,
            20,
            param
                | u32::from(m.repeat[0]) << 16
                | u32::from(m.repeat[1]) << 17
                | u32::from(m.mirror[0]) << 18
                | u32::from(m.mirror[1]) << 19
                | u32::from(m.tex_mode & 3) << 30,
        );
        put32(&mut d, 24, 0xffffffff);
        put16(&mut d, 32, m.width);
        put16(&mut d, 34, m.height);
        put32(&mut d, 36, 4096);
        put32(&mut d, 40, 4096);
        // Texture matrix: 0x2 scale is one, 0x4 no rotation, 0x8 no translation.
        let fx = |v: f32| ((v * 4096.).round() as i32) as u32;
        let mut flags = u16::from(m.tex_scale != [1., 1.] || m.tex_rotate != [0., 1.] || m.tex_translate != [0., 0.]);
        if m.tex_scale == [1., 1.] {
            flags |= 2
        } else {
            d.extend(fx(m.tex_scale[0]).to_le_bytes());
            d.extend(fx(m.tex_scale[1]).to_le_bytes());
        }
        if m.tex_rotate == [0., 1.] {
            flags |= 4
        } else {
            let h = |v: f32| (v * 4096.).round() as i16 as u16 as u32;
            d.extend((h(m.tex_rotate[0]) | h(m.tex_rotate[1]) << 16).to_le_bytes());
        }
        if m.tex_translate == [0., 0.] {
            flags |= 8
        } else {
            d.extend(fx(m.tex_translate[0]).to_le_bytes());
            d.extend(fx(m.tex_translate[1]).to_le_bytes());
        }
        put16(&mut d, 30, flags);
        let size = d.len() as u16;
        put16(&mut d, 2, size);
        material_data.push(d);
    }
    b.extend(offset_dictionary(
        &model
            .materials
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>(),
        &material_data,
        4,
    )?);
    for (field, is_texture) in [(0, true), (2, false)] {
        let off = b.len() - mats;
        if off > 65535 {
            return Err(error(mats, "material section exceeds 64 KiB"));
        }
        put16(&mut b, mats + field, off as u16);
        let mut groups = std::collections::BTreeMap::<String, Vec<u8>>::new();
        for (i, m) in model.materials.iter().enumerate() {
            if let Some(n) = if is_texture { &m.texture } else { &m.palette } {
                groups.entry(n.clone()).or_default().push(i as u8);
            }
        }
        let mut entries = groups
            .keys()
            .map(|n| (n.clone(), vec![0; 4]))
            .collect::<Vec<_>>();
        let len = dictionary(&entries, 4)?.len();
        let mut ids = Vec::<u8>::new();
        for ((_, d), group) in entries.iter_mut().zip(groups.values()) {
            let p = off + len + ids.len();
            if p > 65535 {
                return Err(error(p, "material binding too large"));
            }
            put16(d, 0, p as u16);
            d[2] = group.len() as u8;
            ids.extend(group);
        }
        b.extend(dictionary(&entries, 4)?);
        b.extend(ids);
        align(&mut b, 4);
    }
    let shape_at = b.len();
    b.extend(offset_dictionary(&shape_names, &shapes, 0)?);
    let binds = b.len();
    for inv in inverses {
        for c in 0..4 {
            for r in 0..3 {
                b.extend(fixed(inv[c * 4 + r])?.to_le_bytes());
            }
        }
        for c in 0..3 {
            for r in 0..3 {
                b.extend(fixed(inv[c * 4 + r])?.to_le_bytes());
            }
        }
    }
    let len = b.len();
    put32(&mut b, 0, len as u32);
    put32(&mut b, 4, sbc as u32);
    put32(&mut b, 8, mats as u32);
    put32(&mut b, 12, shape_at as u32);
    put32(&mut b, 16, binds as u32);
    b[23] = model.bones.len() as u8;
    b[24] = model.materials.len() as u8;
    b[25] = model.meshes.len() as u8;
    b[26] = matrix_count as u8;
    put32(&mut b, 32, 4096);
    put32(&mut b, 36, 4096);
    put16(
        &mut b,
        40,
        model
            .meshes
            .iter()
            .map(|m| m.indices.len())
            .sum::<usize>()
            .min(65535) as u16,
    );
    put16(
        &mut b,
        42,
        (model
            .meshes
            .iter()
            .map(|m| m.indices.len() / 3)
            .sum::<usize>())
        .min(65535) as u16,
    );
    Ok(b)
}

fn stripify(indices: &[u32]) -> Vec<Vec<u32>> {
    let triangles = indices.chunks_exact(3).collect::<Vec<_>>();
    let mut strips = Vec::<Vec<u32>>::new();
    let mut i = 0;
    while i < triangles.len() {
        let t = triangles[i];
        let rotation = triangles
            .get(i + 1)
            .and_then(|next| {
                (0..3).find(|&r| {
                    (0..3).any(|k| next[k] == t[(r + 2) % 3] && next[(k + 1) % 3] == t[(r + 1) % 3])
                })
            })
            .unwrap_or(0);
        let mut strip = vec![t[rotation], t[(rotation + 1) % 3], t[(rotation + 2) % 3]];
        i += 1;
        while i < triangles.len() {
            let n = strip.len();
            let (a, b) = if n % 2 == 1 {
                (strip[n - 1], strip[n - 2])
            } else {
                (strip[n - 2], strip[n - 1])
            };
            let t = triangles[i];
            let Some(k) = (0..3).find(|&k| t[k] == a && t[(k + 1) % 3] == b) else {
                break;
            };
            strip.push(t[(k + 2) % 3]);
            i += 1;
        }
        strips.push(strip);
    }
    strips
}
