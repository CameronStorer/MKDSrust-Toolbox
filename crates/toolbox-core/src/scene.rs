//! Course association and read-only overlays, shared by desktop, CLI and fixture generator.
use crate::{model, preview, project::Project, session::RomSession, Error, Result};
use nitro_formats::{
    animation::{self, Animation},
    kcl::Kcl,
    model::Nsbmd,
    nkm::{fx, vec3, Nkm},
    texture::Nsbtx,
};
use serde::Serialize;
use specta::Type;
#[derive(Clone, Debug, Default, Serialize, Type)]
pub struct Budget {
    pub polygons: u32,
    pub vertices: u32,
    pub texture_bytes: u32,
    pub palette_bytes: u32,
    pub matrices: u32,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct SceneEntry {
    pub section: String,
    pub index: u32,
    pub position: Option<[f32; 3]>,
    pub end: Option<[f32; 3]>,
    pub fields: Vec<(String, String)>,
    pub raw: Vec<u8>,
}
#[derive(Clone, Debug, Default, Serialize, Type)]
pub struct SceneInfo {
    pub linked_copies: Vec<crate::linked::LinkedCopy>,
    pub bone_details: Vec<BoneDetail>,
    pub material_details: Vec<MaterialDetail>,
    pub parts: Vec<ModelPart>,
    pub path: String,
    pub model_path: Option<String>,
    pub kcl_path: Option<String>,
    pub nkm_path: Option<String>,
    pub materials: Vec<String>,
    pub bones: Vec<String>,
    pub animations: Vec<String>,
    pub entries: Vec<SceneEntry>,
    pub budget: Budget,
    pub issues: Vec<String>,
    pub collision_types: Vec<u16>,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct BoneDetail {
    pub name: String,
    pub parent: Option<u32>,
    pub position: [f32; 3],
    pub vertices: u32,
    pub clips: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct MaterialDetail {
    pub name: String,
    pub texture: Option<String>,
    pub palette: Option<String>,
    pub path: Option<String>,
    pub texture_index: u32,
    pub palette_index: u32,
    pub format: u8,
    pub colors: Vec<u16>,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct ModelPart {
    pub name: String,
    pub triangles: u32,
    pub material: u32,
}
pub struct CourseScene {
    pub model: Option<Nsbmd>,
    pub textures: Option<Nsbtx>,
    pub animations: Vec<Animation>,
    pub kcl: Option<Kcl>,
    pub nkm: Option<Nkm>,
    pub info: SceneInfo,
}
impl CourseScene {
    pub fn load(project: Option<&Project>, session: &RomSession, path: &str) -> Result<Self> {
        let parent = path.rsplit_once('/').map_or("", |p| p.0);
        let course = parent.starts_with("data/Course/")
            && (path.ends_with("/course_map.nkm")
                || path.ends_with("/course_collision.kcl")
                || path.ends_with("/course_model.nsbmd")
                || path.ends_with("/course_model_V.nsbmd"));
        let find = |ext: &str| -> Option<String> {
            let stem = mkds_data::assets::model_stem(path);
            let exact = format!("{stem}.{ext}");
            if let Some(n) = session
                .nodes
                .iter()
                .find(|n| n.path.eq_ignore_ascii_case(&exact))
            {
                return Some(n.path.clone());
            }
            if course && ["nsbmd", "kcl", "nkm"].contains(&ext) {
                let name = match ext {
                    "nsbmd" => "course_model.nsbmd",
                    "kcl" => "course_collision.kcl",
                    _ => "course_map.nkm",
                };
                return session
                    .nodes
                    .iter()
                    .find(|n| n.path.eq_ignore_ascii_case(&format!("{parent}/{name}")))
                    .map(|n| n.path.clone());
            }
            session
                .nodes
                .iter()
                .find(|n| {
                    n.format == ext
                        && mkds_data::assets::normalized(&n.path)
                            == mkds_data::assets::normalized(&exact)
                })
                .map(|n| n.path.clone())
        };
        let bytes = |p: &str| -> Result<Vec<u8>> {
            preview::plain(match project {
                Some(project) => project.bytes(session, p)?,
                None => session.file(p)?,
            })
        };
        let model_path = find("nsbmd");
        let kcl_path = find("kcl");
        let nkm_path = find("nkm");
        let mut info = SceneInfo {
            path: path.into(),
            model_path: model_path.clone(),
            kcl_path: kcl_path.clone(),
            nkm_path: nkm_path.clone(),
            ..SceneInfo::default()
        };
        let model = model_path
            .as_ref()
            .map(|p| Ok::<_, Error>(Nsbmd::parse(&bytes(p)?)?))
            .transpose()?;
        let texture_path = model_path.as_ref().and_then(|p| {
            mkds_data::assets::texture_candidates(p)
                .iter()
                .find_map(|candidate| {
                    session
                        .nodes
                        .iter()
                        .find(|n| n.path.eq_ignore_ascii_case(candidate))
                        .map(|n| n.path.clone())
                })
        });
        let textures = texture_path
            .as_ref()
            .map(|p| Ok::<_, Error>(Nsbtx::parse(&bytes(p)?)?))
            .transpose()?;
        let kcl = kcl_path
            .map(|p| Ok::<_, Error>(Kcl::parse(&bytes(&p)?)?))
            .transpose()?;
        let nkm = nkm_path
            .map(|p| Ok::<_, Error>(Nkm::parse(&bytes(&p)?)?))
            .transpose()?;
        let mut animations = vec![];
        for n in &session.nodes {
            if n.format == "nsbca"
                && model_path
                    .as_ref()
                    .is_some_and(|m| mkds_data::assets::model_animation(m, &n.path))
            {
                match bytes(&n.path).and_then(|b| Ok(animation::parse(&b)?)) {
                    Ok(mut a) => {
                        for clip in &mut a {
                            if !["_drive.", "_spin.", "_win.", "_lose."]
                                .iter()
                                .any(|suffix| n.name.contains(suffix))
                            {
                                continue;
                            }
                            clip.name =
                                format!("{} - {}", n.name.trim_end_matches(".nsbca"), clip.name);
                        }
                        animations.extend(a);
                    }
                    Err(e) => info.issues.push(format!("{}: {e}", n.name)),
                }
            }
        }
        if let Some(m) = &model {
            info.linked_copies =
                crate::linked::copies(project, session, model_path.as_deref().unwrap_or(path))?;
            let bank = m.textures.as_ref().or(textures.as_ref());
            for m in &m.models {
                let offset = info.bone_details.len() as u32;
                let world = crate::model::bone_world(m);
                for (i, b) in m.bones.iter().enumerate() {
                    let vertices = m
                        .meshes
                        .iter()
                        .map(|mesh| {
                            mesh.joints
                                .iter()
                                .zip(&mesh.weights)
                                .filter(|(js, ws)| {
                                    js.iter()
                                        .zip(ws.iter())
                                        .any(|(&j, &w)| j as usize == i && w > 0.)
                                })
                                .count() as u32
                        })
                        .sum();
                    let position = nitro_formats::math::point(world[i], [0.; 3]);
                    info.bone_details.push(BoneDetail {
                        name: b.name.clone(),
                        parent: b.parent.map(|p| p as u32 + offset),
                        position,
                        vertices,
                        clips: animations
                            .iter()
                            .filter(|a| {
                                a.joints.iter().any(|j| {
                                    j.bone == i
                                        && (j.translations.is_some()
                                            || j.rotations.is_some()
                                            || j.scales.is_some())
                                })
                            })
                            .map(|a| a.name.clone())
                            .collect(),
                    });
                }
                for material in &m.materials {
                    let t = bank.and_then(|b| {
                        material
                            .texture
                            .as_ref()
                            .and_then(|n| b.textures.iter().position(|t| &t.name == n))
                    });
                    let p = bank.and_then(|b| {
                        material
                            .palette
                            .as_ref()
                            .and_then(|n| b.palettes.iter().position(|p| &p.name == n))
                    });
                    info.material_details.push(MaterialDetail {
                        name: material.name.clone(),
                        texture: material.texture.clone(),
                        palette: material.palette.clone(),
                        path: t.and_then(|_| {
                            if model.as_ref().is_some_and(|m| m.textures.is_some()) {
                                model_path.clone()
                            } else {
                                texture_path.clone()
                            }
                        }),
                        texture_index: t.unwrap_or(0) as u32,
                        palette_index: p.unwrap_or(0) as u32,
                        format: bank
                            .and_then(|b| t.map(|t| b.textures[t].format))
                            .unwrap_or(0),
                        colors: bank
                            .and_then(|b| p.map(|p| b.palettes[p].colors.clone()))
                            .unwrap_or_default(),
                    });
                }
                info.materials.extend(m.materials.iter().map(|m| {
                    format!(
                        "{} · {}",
                        m.name,
                        m.texture.as_deref().unwrap_or("untextured")
                    )
                }));
                info.bones.extend(m.bones.iter().map(|b| b.name.clone()));
                info.issues.extend(m.warnings.clone());
                info.parts.extend(m.meshes.iter().map(|mesh| ModelPart {
                    name: mesh.name.clone(),
                    triangles: (mesh.indices.len() / 3) as u32,
                    material: mesh.material as u32,
                }));
            }
            info.budget = crate::model::budget(m, textures.as_ref());
        }
        info.animations = animations
            .iter()
            .map(|a| format!("{} · {} frames", a.name, a.frames))
            .collect();
        if let Some(k) = &kcl {
            info.collision_types = k
                .prisms
                .iter()
                .map(|p| mkds_data::collision_type(p.attribute))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
        }
        if let Some(n) = &nkm {
            info.issues.extend(crate::track_wizard::validate(n));
        }
        if let Some(nkm) = &nkm {
            for s in &nkm.sections {
                for (i, b) in s.entries.iter().enumerate() {
                    info.entries.push(entry(&s.kind, i, b)?);
                }
            }
            validate_nkm(nkm, &mut info.issues)?;
        }
        Ok(Self {
            model,
            textures,
            animations,
            kcl,
            nkm,
            info,
        })
    }
    pub fn glb(&self, collision: bool) -> Result<Vec<u8>> {
        // Course models are drawn 16x by the game; scale them to collision/route units so overlays line up.
        let scaled = self
            .model
            .as_ref()
            .filter(|_| {
                self.info
                    .model_path
                    .as_deref()
                    .is_some_and(|p| p.contains("/course_model"))
            })
            .map(|m| {
                let mut m = m.clone();
                for model in &mut m.models {
                    for mesh in &mut model.meshes {
                        for p in &mut mesh.positions {
                            *p = p.map(|v| v * mkds_data::COURSE_MODEL_SCALE);
                        }
                    }
                }
                m
            });
        model::glb(
            scaled.as_ref().or(self.model.as_ref()),
            self.textures.as_ref(),
            &self.animations,
            if collision { self.kcl.as_ref() } else { None },
        )
    }
}
fn int(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([b[at], b[at + 1]])
}
fn entry(kind: &str, i: usize, b: &[u8]) -> Result<SceneEntry> {
    let mut fields = vec![];
    let mut position = None;
    let mut end = None;
    if [
        "OBJI", "POIT", "KTPS", "KTPJ", "KTP2", "KTPC", "KTPM", "IPOI", "EPOI", "MEPO", "AREA",
        "CAME",
    ]
    .contains(&kind)
    {
        position = Some(vec3(b, 0)?);
        fields.push(("Position".into(), format!("{:?}", position.unwrap())));
    }
    if kind == "CPOI" {
        position = Some([fx(b, 0)?, 0., fx(b, 4)?]);
        end = Some([fx(b, 8)?, 0., fx(b, 12)?]);
        fields.push(("Respawn".into(), b[34].to_string()));
    }
    if kind == "OBJI" {
        fields.push(("Object id".into(), (int(b, 36) as u16).to_string()));
        fields.push(("Path".into(), int(b, 38).to_string()));
        fields.push(("Rotation".into(), format!("{:?}", vec3(b, 12)?)));
        fields.push(("Scale".into(), format!("{:?}", vec3(b, 24)?)));
        for i in 0..7 {
            fields.push((format!("Setting {}", i + 1), int(b, 40 + i * 2).to_string()))
        }
        fields.push(("Visibility".into(), (int(b, 54) as u16).to_string()));
    }
    if ["CPAT", "EPAT", "IPAT", "MEPA"].contains(&kind) {
        fields.push(("Start index".into(), int(b, 0).to_string()));
        fields.push(("Length".into(), int(b, 2).to_string()));
        fields.push(("Next / previous links".into(), format!("{:?}", &b[4..])));
    }
    if kind == "PATH" {
        fields.push(("Point count".into(), int(b, 2).to_string()));
        fields.push(("Loop".into(), (b[1] != 0).to_string()));
    }
    if kind == "CAME" {
        end = Some(vec3(b, 24)?);
        fields.push(("Target 1".into(), format!("{:?}", vec3(b, 24)?)));
        fields.push(("Target 2".into(), format!("{:?}", vec3(b, 36)?)));
        fields.push(("Route".into(), int(b, 64).to_string()));
        fields.push(("Duration".into(), int(b, 70).to_string()));
    }
    if kind == "AREA" {
        fields.push(("Length vector".into(), format!("{:?}", vec3(b, 12)?)));
        for (j, n) in ["X vector", "Y vector", "Z vector"].iter().enumerate() {
            fields.push(((*n).into(), format!("{:?}", vec3(b, 24 + j * 12)?)))
        }
        fields.push((
            "Parameters".into(),
            format!("{}, {}", int(b, 60), int(b, 62)),
        ));
    }
    if kind == "STAG" {
        fields.push(("Course id".into(), (int(b, 0) as u16).to_string()));
        fields.push(("Laps".into(), int(b, 2).to_string()));
    }
    Ok(SceneEntry {
        section: kind.into(),
        index: i as u32,
        position,
        end,
        fields,
        raw: b.to_vec(),
    })
}
fn validate_nkm(nkm: &Nkm, issues: &mut Vec<String>) -> Result<()> {
    let count = |k| nkm.section(k).map_or(0, |s| s.entries.len());
    if count("KTPJ") == 0 {
        issues.push("Missing respawns (KTPJ)".into())
    }
    if let Some(s) = nkm.section("CPOI") {
        for (i, b) in s.entries.iter().enumerate() {
            if b[34] as usize >= count("KTPJ") {
                issues.push(format!("CPOI {i}: respawn {} is missing", b[34]))
            }
        }
    }
    for (paths, points) in [
        ("EPAT", "EPOI"),
        ("IPAT", "IPOI"),
        ("MEPA", "MEPO"),
        ("CPAT", "CPOI"),
    ] {
        if let Some(s) = nkm.section(paths) {
            for (i, b) in s.entries.iter().enumerate() {
                let start = int(b, 0);
                let len = int(b, 2);
                if start < 0 || len < 1 || start as usize + len as usize > count(points) {
                    issues.push(format!("{paths} {i}: invalid {points} range"))
                }
                let links = if paths == "MEPA" { 16 } else { 6 };
                if b[4..4 + links / 2].iter().all(|v| *v == 255) {
                    issues.push(format!("{paths} {i}: unlinked path"))
                }
                for &link in &b[4..4 + links] {
                    if link != 255 && link as usize >= s.entries.len() {
                        issues.push(format!("{paths} {i}: link {link} is missing"))
                    }
                }
            }
        }
    }
    Ok(())
}
