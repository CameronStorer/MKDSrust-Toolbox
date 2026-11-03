//! Shared mesh representation and standalone glTF 2.0 binary export.
use crate::{preview, Error, Result};
pub use nitro_formats::model::Mesh;
use nitro_formats::{
    animation::Animation,
    kcl::Kcl,
    math::{self, Mat4, IDENTITY},
    model::{Model, Nsbmd},
    texture::Nsbtx,
};
use serde_json::{json, Value};
#[derive(Default)]
struct Buffer {
    data: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}
impl Buffer {
    fn view(&mut self, b: &[u8]) -> usize {
        while self.data.len() % 4 != 0 {
            self.data.push(0)
        }
        let i = self.views.len();
        self.views
            .push(json!({"buffer":0,"byteOffset":self.data.len(),"byteLength":b.len()}));
        self.data.extend(b);
        i
    }
    fn floats<const N: usize>(&mut self, v: &[[f32; N]], kind: &str, bounds: bool) -> usize {
        let bytes = v
            .iter()
            .flatten()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>();
        let view = self.view(&bytes);
        let mut a = json!({"bufferView":view,"componentType":5126,"count":v.len(),"type":kind});
        if bounds {
            a["min"] = json!((0..N)
                .map(|i| v.iter().map(|p| p[i]).fold(f32::INFINITY, f32::min))
                .collect::<Vec<_>>());
            a["max"] = json!((0..N)
                .map(|i| v.iter().map(|p| p[i]).fold(f32::NEG_INFINITY, f32::max))
                .collect::<Vec<_>>())
        }
        let i = self.accessors.len();
        self.accessors.push(a);
        i
    }
    fn indices(&mut self, v: &[u32]) -> usize {
        let bytes = v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>();
        let view = self.view(&bytes);
        let i = self.accessors.len();
        self.accessors
            .push(json!({"bufferView":view,"componentType":5125,"count":v.len(),"type":"SCALAR"}));
        i
    }
    fn joints(&mut self, v: &[[u16; 4]]) -> usize {
        let bytes = v
            .iter()
            .flatten()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>();
        let view = self.view(&bytes);
        let i = self.accessors.len();
        self.accessors
            .push(json!({"bufferView":view,"componentType":5123,"count":v.len(),"type":"VEC4"}));
        i
    }
}
pub fn bone_world(model: &Model) -> Vec<Mat4> {
    fn world(i: usize, m: &Model) -> Mat4 {
        if let Some(p) = m.bones[i].parent {
            math::mul(world(p, m), m.bones[i].matrix())
        } else {
            m.bones[i].matrix()
        }
    }
    (0..model.bones.len()).map(|i| world(i, model)).collect()
}
pub fn glb(
    model: Option<&Nsbmd>,
    external: Option<&Nsbtx>,
    animations: &[Animation],
    collision: Option<&Kcl>,
) -> Result<Vec<u8>> {
    let mut buffer = Buffer::default();
    let mut nodes = vec![];
    let mut roots = vec![];
    let mut meshes = vec![];
    let mut materials = vec![];
    let mut images = vec![];
    let mut textures = vec![];
    let mut samplers = vec![];
    let mut skins = vec![];
    let mut anims = vec![];
    if let Some(container) = model {
        let tex = container.textures.as_ref().or(external);
        for model in &container.models {
            let mat_start = materials.len();
            for material in &model.materials {
                let mut pbr = json!({"baseColorFactor":material.diffuse,"metallicFactor":0,"roughnessFactor":1});
                if let (Some(bank), Some(name)) = (tex, &material.texture) {
                    if let Some(t) = bank.textures.iter().position(|t| &t.name == name) {
                        let p = material
                            .palette
                            .as_ref()
                            .and_then(|name| bank.palettes.iter().position(|p| &p.name == name))
                            .unwrap_or(0);
                        let image = bank.decode(t, p)?;
                        let bytes = preview::png(&image)?;
                        let view = buffer.view(&bytes);
                        let id = images.len();
                        images.push(json!({"bufferView":view,"mimeType":"image/png","name":name}));
                        let wrap = |i| {
                            if material.mirror[i] {
                                33648
                            } else if material.repeat[i] {
                                10497
                            } else {
                                33071
                            }
                        };
                        let sampler = samplers.len();
                        samplers.push(json!({"magFilter":9728,"minFilter":9728,"wrapS":wrap(0),"wrapT":wrap(1)}));
                        let texture = textures.len();
                        textures.push(json!({"source":id,"sampler":sampler}));
                        pbr["baseColorTexture"] = json!({"index":texture});
                    }
                }
                materials.push(json!({"name":material.name,"pbrMetallicRoughness":pbr,"doubleSided":material.double_sided,"alphaMode":"MASK","alphaCutoff":0.01}));
            }
            let bone_start = nodes.len();
            let world = bone_world(model);
            let skin_id = skins.len();
            for (i, bone) in model.bones.iter().enumerate() {
                let mut n = json!({"name":bone.name,"translation":bone.translation,"rotation":bone.rotation,"scale":bone.scale});
                let children = (0..model.bones.len())
                    .filter(|j| model.bones[*j].parent == Some(i))
                    .map(|j| bone_start + j)
                    .collect::<Vec<_>>();
                if !children.is_empty() {
                    n["children"] = json!(children)
                }
                if bone.parent.is_none() {
                    roots.push(nodes.len())
                }
                nodes.push(n);
            }
            if !world.is_empty() {
                let inverses = world
                    .iter()
                    .map(|m| math::inverse(*m).unwrap_or(IDENTITY))
                    .collect::<Vec<_>>();
                let accessor = buffer.floats(&inverses, "MAT4", false);
                skins.push(json!({"joints":(bone_start..bone_start+world.len()).collect::<Vec<_>>(),"inverseBindMatrices":accessor}));
            }
            for mesh in &model.meshes {
                if mesh.indices.is_empty() {
                    continue;
                }
                let mut attrs = json!({"POSITION":buffer.floats(&mesh.positions,"VEC3",true),"NORMAL":buffer.floats(&mesh.normals,"VEC3",false),"TEXCOORD_0":buffer.floats(&mesh.uvs,"VEC2",false),"COLOR_0":buffer.floats(&mesh.colors,"VEC4",false)});
                let skinned =
                    !world.is_empty() && mesh.weights.iter().all(|w| w.iter().sum::<f32>() > 0.99);
                if skinned {
                    attrs["JOINTS_0"] = json!(buffer.joints(&mesh.joints));
                    attrs["WEIGHTS_0"] = json!(buffer.floats(&mesh.weights, "VEC4", false))
                }
                let id = meshes.len();
                meshes.push(json!({"name":mesh.name,"primitives":[{"attributes":attrs,"indices":buffer.indices(&mesh.indices),"material":mat_start+mesh.material}]}));
                let mut node = json!({"name":format!("model:{}",mesh.name),"mesh":id});
                if skinned {
                    node["skin"] = json!(skin_id)
                }
                roots.push(nodes.len());
                nodes.push(node);
            }
            for animation in animations {
                let mut channels = vec![];
                let mut samples = vec![];
                let times = (0..animation.frames)
                    .map(|f| [f as f32 / 60.])
                    .collect::<Vec<_>>();
                let input = buffer.floats(&times, "SCALAR", true);
                for joint in &animation.joints {
                    if joint.bone >= world.len() {
                        continue;
                    }
                    let mut outputs = vec![];
                    if let Some(v) = &joint.translations {
                        outputs.push(("translation", buffer.floats(v, "VEC3", false)))
                    }
                    if let Some(v) = &joint.rotations {
                        outputs.push(("rotation", buffer.floats(v, "VEC4", false)))
                    }
                    if let Some(v) = &joint.scales {
                        outputs.push(("scale", buffer.floats(v, "VEC3", false)))
                    }
                    for (path, output) in outputs {
                        channels.push(json!({"sampler":samples.len(),"target":{"node":bone_start+joint.bone,"path":path}}));
                        samples
                            .push(json!({"input":input,"output":output,"interpolation":"LINEAR"}));
                    }
                }
                if !channels.is_empty() {
                    anims
                        .push(json!({"name":animation.name,"channels":channels,"samplers":samples}))
                }
            }
        }
    }
    if let Some(kcl) = collision {
        let triangles = kcl.triangles()?;
        let types = triangles
            .iter()
            .map(|t| t.attribute & 31)
            .collect::<std::collections::BTreeSet<_>>();
        for kind in types {
            let mat = materials.len();
            let color = collision_color(kind);
            materials.push(json!({"name":format!("collision_{kind:02}"),"pbrMetallicRoughness":{"baseColorFactor":[color[0],color[1],color[2],0.65],"metallicFactor":0,"roughnessFactor":1},"doubleSided":true,"alphaMode":"BLEND"}));
            let mut p = vec![];
            let mut n = vec![];
            for t in triangles.iter().filter(|t| t.attribute & 31 == kind) {
                p.extend(t.positions);
                n.extend([t.normal; 3])
            }
            let indices = (0..p.len() as u32).collect::<Vec<_>>();
            let id = meshes.len();
            meshes.push(json!({"primitives":[{"attributes":{"POSITION":buffer.floats(&p,"VEC3",true),"NORMAL":buffer.floats(&n,"VEC3",false)},"indices":buffer.indices(&indices),"material":mat}]}));
            roots.push(nodes.len());
            nodes.push(json!({"name":format!("collision:{kind}"),"mesh":id}));
        }
    }
    if meshes.is_empty() {
        return Err(Error::Invalid("no drawable triangles in this scene".into()));
    }
    let mut document = json!({"asset":{"version":"2.0","generator":"MKDSrust Toolbox"},"scene":0,"scenes":[{"nodes":roots}],"nodes":nodes,"meshes":meshes,"materials":materials,"buffers":[{"byteLength":buffer.data.len()}],"bufferViews":buffer.views,"accessors":buffer.accessors});
    for (key, value) in [
        ("images", images),
        ("textures", textures),
        ("samplers", samplers),
        ("skins", skins),
        ("animations", anims),
    ] {
        if !value.is_empty() {
            document[key] = json!(value)
        }
    }
    let mut json = serde_json::to_vec(&document)?;
    while json.len() % 4 != 0 {
        json.push(b' ')
    }
    while buffer.data.len() % 4 != 0 {
        buffer.data.push(0)
    }
    let len = 12 + 8 + json.len() + 8 + buffer.data.len();
    if len > u32::MAX as usize {
        return Err(Error::Invalid("GLB exceeds 4 GiB".into()));
    }
    let mut b = b"glTF".to_vec();
    b.extend(2u32.to_le_bytes());
    b.extend((len as u32).to_le_bytes());
    b.extend((json.len() as u32).to_le_bytes());
    b.extend(b"JSON");
    b.extend(json);
    b.extend((buffer.data.len() as u32).to_le_bytes());
    b.extend(b"BIN\0");
    b.extend(buffer.data);
    Ok(b)
}
pub fn collision_color(kind: u16) -> [f32; 3] {
    let h = kind as f32 * 2.3999631;
    [
        0.5 + 0.35 * h.sin(),
        0.5 + 0.35 * (h + 2.094).sin(),
        0.5 + 0.35 * (h + 4.189).sin(),
    ]
}
pub fn obj(kcl: &Kcl) -> Result<Vec<u8>> {
    use std::fmt::Write;
    let triangles = kcl.triangles()?;
    let mut s = String::from("# MKDSrust Toolbox collision; one material per collision type\n");
    for (i, t) in triangles.iter().enumerate() {
        writeln!(
            s,
            "g collision_{:02}\nusemtl collision_{:02}",
            t.attribute & 31,
            t.attribute & 31
        )
        .unwrap();
        writeln!(s, "# attribute 0x{:04x}", t.attribute).unwrap();
        for p in t.positions {
            writeln!(s, "v {} {} {}", p[0], p[1], p[2]).unwrap()
        }
        writeln!(s, "vn {} {} {}", t.normal[0], t.normal[1], t.normal[2]).unwrap();
        writeln!(
            s,
            "f {}//{} {}//{} {}//{}",
            i * 3 + 1,
            i + 1,
            i * 3 + 2,
            i + 1,
            i * 3 + 3,
            i + 1
        )
        .unwrap()
    }
    Ok(s.into_bytes())
}
