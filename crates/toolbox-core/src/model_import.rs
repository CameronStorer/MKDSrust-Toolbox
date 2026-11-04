//! Bounded glTF/GLB and OBJ import. No native converter or SDK is required.
use crate::{Error, Result};
use nitro_formats::{
    math::{self, IDENTITY},
    model::{Bone, Material, Mesh, Model, Nsbmd},
    texture::{Nsbtx, Palette, Texture},
};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
fn invalid(s: &str) -> Error {
    Error::Invalid(s.into())
}
fn number(v: &Value) -> Result<usize> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| invalid("missing or invalid glTF integer"))
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| invalid("missing glTF array"))
}
/// Splits an affine column-major matrix into translation, rotation and scale (a mirror goes into the X scale).
fn decompose(m: math::Mat4) -> Result<([f32; 3], [f32; 4], [f32; 3])> {
    let column = |k: usize| [m[k * 4], m[k * 4 + 1], m[k * 4 + 2]];
    let mut scale = [0, 1, 2].map(|k| math::dot(column(k), column(k)).sqrt());
    if scale.iter().any(|v| !v.is_finite() || *v < 1e-8) || m[3] != 0. || m[7] != 0. || m[11] != 0.
    {
        return Err(invalid(
            "a bone matrix is not an invertible affine transform",
        ));
    }
    if math::dot(math::cross(column(0), column(1)), column(2)) < 0. {
        scale[0] = -scale[0];
    }
    let r: [f32; 9] = std::array::from_fn(|i| m[i / 3 * 4 + i % 3] / scale[i / 3]);
    Ok(([m[12], m[13], m[14]], math::quaternion(r), scale))
}
fn floats<const N: usize>(v: &Value, default: [f32; N]) -> [f32; N] {
    std::array::from_fn(|i| v[i].as_f64().map_or(default[i], |n| n as f32))
}
pub fn default_material(name: String) -> Material {
    Material {
        name,
        texture: None,
        palette: None,
        diffuse: [1.; 4],
        width: 8,
        height: 8,
        repeat: [true; 2],
        mirror: [false; 2],
        double_sided: true,
    }
}
pub fn import(path: &Path) -> Result<Nsbmd> {
    import_with(path, true)
}
/// For the wizards, which re-rig onto the game's own skeleton: skins are dropped and skinned meshes keep their
/// bind pose, so source skeletons of any size or rest pose import.
pub fn import_static(path: &Path) -> Result<Nsbmd> {
    import_with(path, false)
}
fn import_with(path: &Path, rigged: bool) -> Result<Nsbmd> {
    let bytes = std::fs::read(path)?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err(invalid("model exceeds 128 MiB"));
    }
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "obj" => obj(std::str::from_utf8(&bytes).map_err(|_| invalid("OBJ must be UTF-8"))?),
        "gltf" | "glb" => gltf_with(&bytes, path.parent().unwrap_or(Path::new(".")), rigged),
        _ => Err(invalid(
            "Import glTF, GLB or OBJ; convert FBX/DAE in Blender first",
        )),
    }
}
pub fn obj(text: &str) -> Result<Nsbmd> {
    let mut positions = vec![];
    let mut normals = vec![];
    let mut uvs = vec![];
    let mut colors = vec![];
    let mut meshes = vec![];
    let mut materials = vec![default_material("default".into())];
    let mut material_ids = BTreeMap::<String, usize>::new();
    let mut mesh = Mesh {
        name: "part0".into(),
        ..Mesh::default()
    };
    let mut map = BTreeMap::<(usize, Option<usize>, Option<usize>), u32>::new();
    let parse = |s: &str| s.parse::<f32>().map_err(|_| invalid("invalid OBJ number"));
    let index = |s: &str, n: usize| -> Result<usize> {
        let i = s.parse::<i64>().map_err(|_| invalid("invalid OBJ index"))?;
        let i = if i < 0 { n as i64 + i } else { i - 1 };
        if i < 0 || i >= n as i64 {
            return Err(invalid("OBJ index out of range"));
        }
        Ok(i as usize)
    };
    for line in text.lines() {
        let words = line
            .split('#')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>();
        if words.is_empty() {
            continue;
        }
        match words[0] {
            "v" => {
                if words.len() < 4 {
                    return Err(invalid("OBJ vertex needs xyz"));
                }
                positions.push([parse(words[1])?, parse(words[2])?, parse(words[3])?]);
                colors.push(if words.len() >= 7 {
                    [parse(words[4])?, parse(words[5])?, parse(words[6])?, 1.]
                } else {
                    [1.; 4]
                });
            }
            "vn" => {
                if words.len() < 4 {
                    return Err(invalid("OBJ normal needs xyz"));
                }
                normals.push([parse(words[1])?, parse(words[2])?, parse(words[3])?]);
            }
            "vt" => {
                if words.len() < 3 {
                    return Err(invalid("OBJ UV needs uv"));
                }
                uvs.push([parse(words[1])?, 1. - parse(words[2])?]);
            }
            "usemtl" | "o" | "g" => {
                if !mesh.indices.is_empty() {
                    meshes.push(mesh.clone());
                    mesh = Mesh {
                        name: format!("part{}", meshes.len()),
                        material: mesh.material,
                        ..Mesh::default()
                    };
                    map.clear();
                }
                if let Some(name) = words.get(1) {
                    if words[0] == "usemtl" {
                        mesh.material = *material_ids.entry((*name).into()).or_insert_with(|| {
                            let i = materials.len();
                            let prefix = name
                                .chars()
                                .filter(char::is_ascii)
                                .take(11)
                                .collect::<String>();
                            materials.push(default_material(format!("{prefix}_{i}")));
                            i
                        });
                    } else {
                        mesh.name = format!("part{}", meshes.len());
                    }
                }
            }
            "f" => {
                if words.len() < 4 {
                    return Err(invalid("OBJ face needs at least 3 vertices"));
                }
                let mut face = vec![];
                for word in &words[1..] {
                    let v = word.split('/').collect::<Vec<_>>();
                    let p = index(v[0], positions.len())?;
                    let uv = v
                        .get(1)
                        .filter(|s| !s.is_empty())
                        .map(|s| index(s, uvs.len()))
                        .transpose()?;
                    let normal = v
                        .get(2)
                        .filter(|s| !s.is_empty())
                        .map(|s| index(s, normals.len()))
                        .transpose()?;
                    let key = (p, uv, normal);
                    let id = *map.entry(key).or_insert_with(|| {
                        let id = mesh.positions.len() as u32;
                        mesh.positions.push(positions[p]);
                        mesh.uvs.push(uv.map_or([0.; 2], |i| uvs[i]));
                        mesh.normals.push(normal.map_or([0.; 3], |i| normals[i]));
                        mesh.colors.push(colors[p]);
                        id
                    });
                    face.push(id);
                }
                for i in 1..face.len() - 1 {
                    mesh.indices.extend([face[0], face[i], face[i + 1]]);
                }
            }
            _ => {}
        }
    }
    if !mesh.indices.is_empty() {
        meshes.push(mesh);
    }
    if meshes.is_empty() {
        return Err(invalid("model has no faces"));
    }
    for m in &mut meshes {
        nitro_formats::model::validate_mesh(m)?;
        if m.normals.iter().all(|n| *n == [0.; 3]) {
            nitro_formats::model::recompute_normals(m)?;
        }
    }
    Ok(Nsbmd {
        models: vec![Model {
            name: "imported".into(),
            bones: vec![Bone {
                name: "root".into(),
                parent: None,
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
            }],
            materials,
            meshes,
            matrix_count: 1,
            warnings: vec![],
        }],
        textures: None,
    })
}
fn uri(uri: &str, base: &Path) -> Result<Vec<u8>> {
    if let Some((prefix, data)) = uri
        .split_once(',')
        .filter(|(p, _)| p.starts_with("data:") && p.ends_with(";base64"))
    {
        let _ = prefix;
        let mut out = vec![];
        let mut acc = 0u32;
        let mut bits = 0;
        for c in data
            .bytes()
            .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        {
            let v = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                _ => return Err(invalid("invalid base64 glTF buffer")),
            };
            acc = (acc << 6) | v as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        return Ok(out);
    }
    if uri.contains("://")
        || uri.contains('%')
        || Path::new(uri).is_absolute()
        || Path::new(uri)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(invalid(
            "glTF resources must be local relative paths without parent traversal",
        ));
    }
    let bytes = std::fs::read(base.join(uri))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err(invalid("glTF resource exceeds 128 MiB"));
    }
    Ok(bytes)
}
struct Gltf {
    json: Value,
    buffers: Vec<Vec<u8>>,
}
impl Gltf {
    fn view(&self, id: usize) -> Result<&[u8]> {
        let v = self.json["bufferViews"]
            .get(id)
            .ok_or_else(|| invalid("buffer view out of range"))?;
        let b = self
            .buffers
            .get(number(&v["buffer"])?)
            .ok_or_else(|| invalid("buffer out of range"))?;
        let at = v["byteOffset"].as_u64().unwrap_or(0) as usize;
        let len = number(&v["byteLength"])?;
        b.get(
            at..at
                .checked_add(len)
                .ok_or_else(|| invalid("buffer overflow"))?,
        )
        .ok_or_else(|| invalid("truncated buffer view"))
    }
    fn accessor(&self, id: usize, width: usize) -> Result<Vec<Vec<f32>>> {
        let a = self.json["accessors"]
            .get(id)
            .ok_or_else(|| invalid("accessor out of range"))?;
        if !a["sparse"].is_null() {
            return Err(invalid("expand sparse glTF accessors before import"));
        }
        let components = match a["type"].as_str() {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            Some("MAT4") => 16,
            _ => return Err(invalid("unsupported accessor type")),
        };
        if width != components {
            return Err(invalid("accessor width mismatch"));
        }
        let kind = number(&a["componentType"])?;
        let size = match kind {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return Err(invalid("unsupported accessor component")),
        };
        let view = number(&a["bufferView"])?;
        let bytes = self.view(view)?;
        let stride = self.json["bufferViews"][view]["byteStride"]
            .as_u64()
            .map_or(width * size, |n| n as usize);
        if stride < width * size {
            return Err(invalid("invalid accessor stride"));
        }
        let at = a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let count = number(&a["count"])?;
        if count > 1_000_000 {
            return Err(invalid("accessor exceeds one million elements"));
        }
        let normalized = a["normalized"].as_bool().unwrap_or(false);
        (0..count)
            .map(|i| {
                (0..width)
                    .map(|k| {
                        let p = i
                            .checked_mul(stride)
                            .and_then(|v| v.checked_add(k * size))
                            .and_then(|v| at.checked_add(v))
                            .ok_or_else(|| invalid("accessor overflow"))?;
                        let end = p
                            .checked_add(size)
                            .ok_or_else(|| invalid("accessor overflow"))?;
                        let s = bytes
                            .get(p..end)
                            .ok_or_else(|| invalid("truncated accessor"))?;
                        let v = match kind {
                            5120 => s[0] as i8 as f32,
                            5121 => s[0] as f32,
                            5122 => i16::from_le_bytes(s.try_into().unwrap()) as f32,
                            5123 => u16::from_le_bytes(s.try_into().unwrap()) as f32,
                            5125 => u32::from_le_bytes(s.try_into().unwrap()) as f32,
                            _ => f32::from_le_bytes(s.try_into().unwrap()),
                        };
                        let v = if normalized {
                            match kind {
                                5120 => (v / 127.).max(-1.),
                                5121 => v / 255.,
                                5122 => (v / 32767.).max(-1.),
                                5123 => v / 65535.,
                                _ => v,
                            }
                        } else {
                            v
                        };
                        if !v.is_finite() {
                            return Err(invalid("non-finite glTF attribute"));
                        }
                        Ok(v)
                    })
                    .collect()
            })
            .collect()
    }
}
pub fn gltf(bytes: &[u8], base: &Path) -> Result<Nsbmd> {
    gltf_with(bytes, base, true)
}
fn gltf_with(bytes: &[u8], base: &Path, rigged: bool) -> Result<Nsbmd> {
    let mut bin = None;
    let json = if bytes.starts_with(b"glTF") {
        if bytes.len() < 20
            || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 2
            || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize != bytes.len()
        {
            return Err(invalid("invalid GLB header"));
        }
        let mut at = 12;
        let mut json = None;
        while at < bytes.len() {
            let h = bytes
                .get(at..at + 8)
                .ok_or_else(|| invalid("truncated GLB chunk"))?;
            let n = u32::from_le_bytes(h[..4].try_into().unwrap()) as usize;
            let data = bytes
                .get(at + 8..at + 8 + n)
                .ok_or_else(|| invalid("truncated GLB payload"))?;
            match &h[4..] {
                b"JSON" => json = Some(serde_json::from_slice(data)?),
                b"BIN\0" => bin = Some(data.to_vec()),
                _ => {}
            }
            at += 8 + n;
        }
        json.ok_or_else(|| invalid("GLB has no JSON"))?
    } else {
        serde_json::from_slice(bytes)?
    };
    let mut doc = Gltf {
        json,
        buffers: vec![],
    };
    for b in array(&doc.json, "buffers")? {
        doc.buffers.push(if let Some(u) = b["uri"].as_str() {
            uri(u, base)?
        } else {
            bin.clone().ok_or_else(|| invalid("missing GLB buffer"))?
        });
    }
    let nodes = doc.json["nodes"].as_array().cloned().unwrap_or_default();
    let mut parents = vec![None; nodes.len()];
    for (i, n) in nodes.iter().enumerate() {
        if let Some(children) = n["children"].as_array() {
            for child in children {
                let c = number(child)?;
                if c >= nodes.len() || parents[c].replace(i).is_some() {
                    return Err(invalid("invalid node hierarchy"));
                }
            }
        }
    }
    let mut world = vec![IDENTITY; nodes.len()];
    let mut visited = vec![false; nodes.len()];
    for _ in 0..nodes.len() {
        for (i, n) in nodes.iter().enumerate() {
            if visited[i] || parents[i].is_some_and(|p| !visited[p]) {
                continue;
            }
            let local = if n["matrix"].is_array() {
                floats(&n["matrix"], IDENTITY)
            } else {
                math::trs(
                    floats(&n["translation"], [0.; 3]),
                    floats(&n["rotation"], [0., 0., 0., 1.]),
                    floats(&n["scale"], [1.; 3]),
                )
            };
            world[i] = math::mul(parents[i].map_or(IDENTITY, |p| world[p]), local);
            visited[i] = true;
        }
    }
    if visited.iter().any(|v| !*v) {
        return Err(invalid("cyclic glTF nodes"));
    }
    let has_skin = rigged && nodes.iter().any(|n| n["skin"].is_number());
    let mut bone_nodes = vec![];
    if has_skin {
        for skin in doc.json["skins"]
            .as_array()
            .ok_or_else(|| invalid("missing skins"))?
        {
            for joint in array(skin, "joints")? {
                let mut id = Some(number(joint)?);
                for _ in 0..=nodes.len() {
                    let Some(i) = id else {
                        break;
                    };
                    if i >= nodes.len() {
                        return Err(invalid("joint outside node array"));
                    }
                    if !bone_nodes.contains(&i) {
                        bone_nodes.push(i);
                    }
                    id = parents[i];
                }
            }
        }
        bone_nodes.sort_by_key(|&i| {
            let mut count = 0;
            let mut p = parents[i];
            while let Some(j) = p {
                count += 1;
                p = parents[j];
            }
            count
        });
    }
    let bones = if has_skin {
        bone_nodes
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                // Blender and other exporters sometimes write bones as matrices: split them into TRS.
                let (translation, rotation, scale) = if nodes[n]["matrix"].is_array() {
                    decompose(floats(&nodes[n]["matrix"], IDENTITY))?
                } else {
                    (
                        floats(&nodes[n]["translation"], [0.; 3]),
                        floats(&nodes[n]["rotation"], [0., 0., 0., 1.]),
                        floats(&nodes[n]["scale"], [1.; 3]),
                    )
                };
                Ok(Bone {
                    name: format!("bone{i}"),
                    parent: parents[n].and_then(|p| bone_nodes.iter().position(|&n| n == p)),
                    translation,
                    rotation,
                    scale,
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![Bone {
            name: "root".into(),
            parent: None,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }]
    };
    let mut textures = Vec::<Texture>::new();
    let mut palettes = Vec::<Palette>::new();
    let mut materials = vec![];
    for (i, material) in doc.json["materials"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let prefix = material["name"]
            .as_str()
            .unwrap_or("mat")
            .chars()
            .filter(char::is_ascii)
            .take(11)
            .collect::<String>();
        let mut m = default_material(format!("{prefix}_{i}"));
        let pbr = &material["pbrMetallicRoughness"];
        m.diffuse = floats(&pbr["baseColorFactor"], [1.; 4]);
        m.double_sided = material["doubleSided"].as_bool().unwrap_or(false);
        if let Some(id) = pbr["baseColorTexture"]["index"].as_u64() {
            let image = number(&doc.json["textures"][id as usize]["source"])?;
            let image = &doc.json["images"][image];
            let data = if let Some(u) = image["uri"].as_str() {
                uri(u, base)?
            } else {
                doc.view(number(&image["bufferView"])?)?.to_vec()
            };
            let image = crate::graphic::read_image_data(&data)?;
            let (t, p) = crate::model::convert_texture(&image, &format!("tex{i}"), 3, 256)?;
            m.width = t.width as u16;
            m.height = t.height as u16;
            m.texture = Some(t.name.clone());
            m.palette = Some(p.name.clone());
            textures.push(t);
            palettes.push(p);
        }
        materials.push(m);
    }
    if materials.is_empty() {
        materials.push(default_material("default".into()));
    }
    let mut meshes = vec![];
    for (node_id, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n["mesh"].is_number())
    {
        let source = &doc.json["meshes"][number(&node["mesh"])?];
        for primitive in array(source, "primitives")? {
            if primitive["mode"].as_u64().unwrap_or(4) != 4 {
                return Err(invalid("triangulate glTF primitives before import"));
            }
            let attributes = &primitive["attributes"];
            let read = |key: &str, width| -> Result<Vec<Vec<f32>>> {
                attributes[key]
                    .as_u64()
                    .map(|i| doc.accessor(i as usize, width))
                    .transpose()
                    .map(|v| v.unwrap_or_default())
            };
            let positions = read("POSITION", 3)?
                .into_iter()
                .map(|v| v.try_into().unwrap())
                .collect();
            let normals = read("NORMAL", 3)?
                .into_iter()
                .map(|v| v.try_into().unwrap())
                .collect();
            let uvs = read("TEXCOORD_0", 2)?
                .into_iter()
                .map(|v| v.try_into().unwrap())
                .collect();
            let colors = if let Some(id) = attributes["COLOR_0"].as_u64() {
                let width = if doc.json["accessors"][id as usize]["type"] == "VEC3" {
                    3
                } else {
                    4
                };
                doc.accessor(id as usize, width)?
                    .into_iter()
                    .map(|v| [v[0], v[1], v[2], v.get(3).copied().unwrap_or(1.)])
                    .collect()
            } else {
                vec![]
            };
            let mut m = Mesh {
                name: format!("mesh{}", meshes.len()),
                positions,
                normals,
                uvs,
                colors,
                material: primitive["material"].as_u64().unwrap_or(0) as usize,
                ..Mesh::default()
            };
            m.indices = if let Some(id) = primitive["indices"].as_u64() {
                doc.accessor(id as usize, 1)?
                    .into_iter()
                    .map(|v| v[0] as u32)
                    .collect()
            } else {
                (0..m.positions.len() as u32).collect()
            };
            if let Some(skin) = node["skin"].as_u64().filter(|_| rigged) {
                let skin = &doc.json["skins"][skin as usize];
                let joints = array(skin, "joints")?;
                let js = read("JOINTS_0", 4)?;
                let ws = read("WEIGHTS_0", 4)?;
                if js.len() != m.positions.len() || ws.len() != js.len() {
                    return Err(invalid("skin needs joints and weights per vertex"));
                }
                m.joints = js
                    .into_iter()
                    .map(|v| {
                        let mut out = [0; 4];
                        for k in 0..4 {
                            let n = joints
                                .get(v[k] as usize)
                                .ok_or_else(|| invalid("skin joint index out of range"))?;
                            out[k] = bone_nodes
                                .iter()
                                .position(|&i| Some(i as u64) == n.as_u64())
                                .ok_or_else(|| invalid("missing skin bone"))?
                                as u16;
                        }
                        Ok(out)
                    })
                    .collect::<Result<Vec<_>>>()?;
                m.weights = ws.into_iter().map(|v| v.try_into().unwrap()).collect();
                if let Some(id) = skin["inverseBindMatrices"].as_u64() {
                    let inv = doc.accessor(id as usize, 16)?;
                    if inv.len() != joints.len() {
                        return Err(invalid("inverse bind count mismatch"));
                    }
                    for (i, joint) in joints.iter().enumerate() {
                        let expected = math::inverse(world[number(joint)?])
                            .ok_or_else(|| invalid("singular joint"))?;
                        if expected
                            .iter()
                            .zip(&inv[i])
                            .any(|(a, b)| (a - b).abs() > 0.005)
                        {
                            return Err(invalid("apply the skin bind pose before import (inverse bind differs from rest pose)"));
                        }
                    }
                }
            } else if !node["skin"].is_number() {
                // a skinned mesh's vertices are already in bind (model) space; its node transform is ignored
                nitro_formats::model::transform(&mut m, world[node_id])?;
            }
            nitro_formats::model::validate_mesh(&m)?;
            if m.material >= materials.len() {
                return Err(invalid("material index out of range"));
            }
            if m.normals.is_empty() {
                nitro_formats::model::recompute_normals(&mut m)?;
            }
            meshes.push(m);
        }
    }
    if meshes.is_empty() {
        return Err(invalid("glTF has no triangle meshes"));
    }
    Ok(Nsbmd {
        models: vec![Model {
            name: "imported".into(),
            matrix_count: bones.len(),
            bones,
            materials,
            meshes,
            warnings: vec![],
        }],
        textures: if textures.is_empty() {
            None
        } else {
            Some(Nsbtx::new(textures, palettes)?)
        },
    })
}

#[cfg(test)]
mod decompose_tests {
    use super::*;
    #[test]
    fn matrix_bones_split_into_the_same_transform() {
        let (t, r, s) = ([1., -2., 3.], [0.2, 0.3, -0.1, 0.927], [2., 0.5, 1.5]);
        let n = r.iter().map(|v: &f32| v * v).sum::<f32>().sqrt();
        let r = r.map(|v| v / n);
        let m = math::trs(t, r, s);
        let (t2, r2, s2) = decompose(m).unwrap();
        let back = math::trs(t2, r2, s2);
        for (a, b) in m.iter().zip(back) {
            assert!((a - b).abs() < 1e-4, "{m:?} vs {back:?}");
        }
        assert!(decompose([0.; 16]).is_err());
    }
}
