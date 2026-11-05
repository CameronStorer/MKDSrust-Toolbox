//! Explicit runtime variants. Never edit the developer archive or stale menu archive.
use crate::{
    engine::{self, ModelAction},
    preview,
    project::Project,
    session::RomSession,
    Error, Result,
};
use nitro_formats::model::Nsbmd;
use serde::Serialize;
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Type)]
pub struct LinkedCopy {
    pub path: String,
    pub role: String,
    pub triangles: u32,
    pub budget: u32,
}

pub fn paths(s: &RomSession, path: &str) -> Vec<(String, String)> {
    let stem = path.rsplit('/').next().unwrap_or("");
    let character = stem.strip_prefix("P_").or_else(|| stem.strip_prefix("E_"));
    let kart = stem
        .strip_prefix("kart_")
        .or_else(|| stem.strip_prefix("sh_"));
    let Some(code) = character.or(kart) else {
        return vec![];
    };
    if !path.contains("KartModel") || (kart.is_some() && code.starts_with("tire_")) {
        return vec![];
    }
    let mut out = vec![];
    for n in &s.nodes {
        if n.format != "nsbmd"
            || ![
                "data/KartModelSub.carc/",
                "data/KartModelMainA.carc/",
                "data/KartModelMainB.carc/",
                "data/KartModelMenu/",
            ]
            .iter()
            .any(|prefix| n.path.starts_with(prefix))
        {
            continue;
        }
        let name = n.path.rsplit('/').next().unwrap_or("");
        let menu = n.path.contains("KartModelMenu");
        let role = if character.is_some() {
            if name == format!("P_{code}") {
                if menu {
                    "Menu driver"
                } else {
                    "Race driver"
                }
            } else if name == format!("E_{code}") && !menu {
                "Far-away driver"
            } else {
                continue;
            }
        } else if name == format!("kart_{code}") {
            if menu {
                "Menu body"
            } else {
                "Race body"
            }
        } else if name == format!("sh_{code}") {
            if menu {
                "Menu shadow"
            } else {
                "Race shadow"
            }
        } else {
            continue;
        };
        out.push((n.path.clone(), role.into()));
    }
    out
}

pub fn copies(p: Option<&Project>, s: &RomSession, path: &str) -> Result<Vec<LinkedCopy>> {
    paths(s, path)
        .into_iter()
        .map(|(path, role)| {
            let read = |project: Option<&Project>| -> Result<Nsbmd> {
                Ok(Nsbmd::parse(&preview::plain(
                    project.map_or_else(|| s.file(&path), |p| p.bytes(s, &path))?,
                )?)?)
            };
            let count = |m: &Nsbmd| {
                m.models
                    .iter()
                    .flat_map(|m| &m.meshes)
                    .map(|m| m.indices.len() as u32 / 3)
                    .sum::<u32>()
            };
            Ok(LinkedCopy {
                triangles: count(&read(p)?),
                budget: count(&read(None)?),
                path,
                role,
            })
        })
        .collect()
}

/// Stage all variants before recording a single project command. Failure leaves no edits/history.
pub fn edit(
    p: &mut Project,
    s: &RomSession,
    path: &str,
    actions: Vec<ModelAction>,
    linked: bool,
) -> Result<()> {
    if actions.is_empty() || actions.len() > 1024 {
        return Err(Error::Invalid("choose model parts to edit".into()));
    }
    let source = Nsbmd::parse(&preview::plain(p.bytes(s, path)?)?)?;
    let mut targets = if linked { paths(s, path) } else { vec![] };
    if targets.is_empty() {
        targets.push((path.into(), "Model".into()));
    }
    if linked && !targets.iter().any(|(p, _)| p == path) {
        return Err(Error::Invalid(
            "Open the runtime copy before editing linked variants".into(),
        ));
    }
    let mut draft = Project::in_memory(s, p.config.manifest.clone(), p.edits.clone())?;
    let mut changed = BTreeSet::new();
    for (dest, role) in &targets {
        let donor = Nsbmd::parse(&preview::plain(p.bytes(s, dest)?)?)?;
        let sm = source
            .models
            .first()
            .ok_or_else(|| Error::Invalid("empty source model".into()))?;
        let dm = donor
            .models
            .first()
            .ok_or_else(|| Error::Invalid("empty linked model".into()))?;
        let selected: BTreeSet<_> = actions.iter().filter_map(part).collect();
        let full_elements = actions.iter().all(|a| match a {
            ModelAction::Edit {
                part, selection, ..
            } => sm.meshes.get(*part as usize).is_some_and(|m| {
                crate::mesh_edit::vertices(m, selection).is_ok_and(|v| v.len() == m.positions.len())
            }),
            _ => true,
        });
        let all = selected.len() == sm.meshes.len() && full_elements;
        let mut done = BTreeSet::new();
        for action in &actions {
            if let ModelAction::Import { source: file } = action {
                import(&mut draft, s, dest, role, file)?;
                changed.insert(dest.clone());
                for (candidate, bytes) in &draft.edits {
                    if p.edits.get(candidate) != Some(bytes) {
                        changed.insert(candidate.clone());
                    }
                }
                continue;
            }
            let source_part = part(action).ok_or_else(|| Error::Invalid("choose a part".into()))?;
            let mesh = sm
                .meshes
                .get(source_part)
                .ok_or_else(|| Error::Invalid("part outside source model".into()))?;
            let parts: Vec<_> = if dest == path {
                vec![source_part]
            } else if all {
                (0..dm.meshes.len()).collect()
            } else {
                let found: Vec<_> = dm
                    .meshes
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.name == mesh.name)
                    .map(|(i, _)| i)
                    .collect();
                if found.len() != 1 {
                    return Err(Error::Invalid(format!("{role}: no unique part named {}. Turn off Apply to all copies to edit this variant alone.", mesh.name)));
                }
                found
            };
            for i in parts {
                if !done.insert(i) {
                    continue;
                }
                let mut mapped = action.clone();
                set_part(&mut mapped, i as u32);
                // Element IDs have meaning only in the source topology. Map by bind-space position.
                if dest != path {
                    if let ModelAction::Edit { selection, .. } = &mut mapped {
                        *selection =
                            crate::mesh_edit::map_selection(mesh, &dm.meshes[i], selection)?;
                    }
                    if let ModelAction::Delete { selection, .. } = &mapped {
                        let faces = match selection {
                            engine::MeshSelection::Triangles { triangles } => {
                                Some(triangles.clone())
                            }
                            engine::MeshSelection::Connected { triangle } => Some(
                                nitro_formats::model::connected_part(mesh, *triangle as usize)?
                                    .into_iter()
                                    .map(|i| i as u32)
                                    .collect(),
                            ),
                            _ => None,
                        };
                        if let Some(faces) = faces {
                            let source_selection = crate::mesh_edit::Selection {
                                mode: "face".into(),
                                faces,
                                vertices: vec![],
                                edges: vec![],
                            };
                            mapped = ModelAction::Edit {
                                part: i as u32,
                                selection: crate::mesh_edit::map_selection(
                                    mesh,
                                    &dm.meshes[i],
                                    &source_selection,
                                )?,
                                operation: crate::mesh_edit::Operation::Delete,
                            };
                        } else if let ModelAction::Delete {
                            selection: engine::MeshSelection::Material { material },
                            ..
                        } = &mut mapped
                        {
                            *material = dm.meshes[i].material as u32;
                        }
                    }
                }
                engine::model(&mut draft, s, dest, mapped)?;
                changed.insert(dest.clone());
            }
        }
    }
    let title = draft
        .history()
        .last()
        .map(|h| h.title.clone())
        .unwrap_or_else(|| "Edit model".into());
    let edits = changed
        .into_iter()
        .map(|path| Ok((path.clone(), draft.bytes(s, &path)?.to_vec())))
        .collect::<Result<BTreeMap<_, _>>>()?;
    p.edit_many(edits, &title, s)
}
fn part(a: &ModelAction) -> Option<usize> {
    match a {
        ModelAction::Import { .. } => None,
        ModelAction::Transform { part, .. }
        | ModelAction::Decimate { part, .. }
        | ModelAction::Cut { part, .. }
        | ModelAction::Delete { part, .. }
        | ModelAction::Merge { part, .. }
        | ModelAction::Normals { part }
        | ModelAction::Edit { part, .. } => Some(*part as usize),
    }
}
fn set_part(a: &mut ModelAction, i: u32) {
    match a {
        ModelAction::Import { .. } => (),
        ModelAction::Transform { part, .. }
        | ModelAction::Decimate { part, .. }
        | ModelAction::Cut { part, .. }
        | ModelAction::Delete { part, .. }
        | ModelAction::Merge { part, .. }
        | ModelAction::Normals { part }
        | ModelAction::Edit { part, .. } => *part = i,
    }
}
fn import(p: &mut Project, s: &RomSession, path: &str, role: &str, file: &str) -> Result<()> {
    let donor = crate::scene::CourseScene::load(Some(p), s, path)?;
    let base = crate::scene::CourseScene::load(None, s, path)?;
    let source = crate::model_import::import_static(std::path::Path::new(file))?;
    let mut m = source
        .models
        .first()
        .ok_or_else(|| Error::Invalid("empty import".into()))?
        .clone();
    let dm = &donor
        .model
        .as_ref()
        .ok_or_else(|| Error::Invalid("empty donor".into()))?
        .models[0];
    let limit = &base.info.budget;
    let unreduced = m.clone();
    if let Err(error) = crate::wizard::reduce(
        &mut m,
        limit.polygons.min(2048) as usize,
        limit.vertices.min(6144) as usize,
    ) {
        m = unreduced;
        let total = m
            .meshes
            .iter()
            .map(|m| m.indices.len() / 3)
            .sum::<usize>()
            .max(1);
        for mesh in &mut m.meshes {
            let share = (mesh.indices.len() / 3 * limit.polygons.min(2048) as usize / total).max(1);
            nitro_formats::model::decimate_lod(mesh, share)?;
        }
        if m.meshes.iter().map(|m| m.indices.len() / 3).sum::<usize>()
            > limit.polygons.min(2048) as usize
            || m.meshes.iter().map(|m| m.positions.len()).sum::<usize>()
                > limit.vertices.min(6144) as usize
        {
            return Err(error);
        }
    }
    m.name = dm.name.clone();
    crate::wizard::rigid_rig(&mut m, dm)?;
    if role == "Far-away driver" {
        m.bones = vec![nitro_formats::model::Bone {
            name: dm
                .bones
                .first()
                .map_or_else(|| "root".into(), |b| b.name.clone()),
            parent: None,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }];
        m.matrix_count = 1;
        for mesh in &mut m.meshes {
            mesh.joints = vec![[0; 4]; mesh.positions.len()];
            mesh.weights = vec![[1., 0., 0., 0.]; mesh.positions.len()];
        }
    }
    let mut edits = BTreeMap::new();
    let textures = if role.contains("shadow") {
        let y = dm
            .meshes
            .iter()
            .flat_map(|m| &m.positions)
            .map(|p| p[1])
            .fold(0., f32::min);
        m.materials = dm.materials.clone();
        for mesh in &mut m.meshes {
            mesh.material = 0;
            for p in &mut mesh.positions {
                p[1] = y;
            }
        }
        None
    } else if let Some(tx) = donor
        .model
        .as_ref()
        .and_then(|m| m.textures.as_ref())
        .or(donor.textures.as_ref())
    {
        let tx = crate::wizard::driver_materials(&mut m, dm, &source, tx)?;
        for mat in &mut m.materials {
            if mat.palette.is_none() {
                mat.palette = tx
                    .palettes
                    .iter()
                    .find(|p| Some(&p.name) == mat.texture.as_ref())
                    .or(tx.palettes.first())
                    .map(|p| p.name.clone());
            }
        }
        if donor.model.as_ref().is_some_and(|m| m.textures.is_some()) {
            Some(tx)
        } else {
            let dest = texture_path(s, path)
                .ok_or_else(|| Error::Invalid("missing linked texture resource".into()))?;
            edits.insert(dest.clone(), engine::wrap(p.bytes(s, &dest)?, tx.write()));
            None
        }
    } else {
        None
    };
    let out = Nsbmd {
        models: vec![m],
        textures,
    };
    let b = crate::model::budget(&out, None);
    if !b.reasons.is_empty() {
        return Err(Error::Invalid(b.reasons.join("; ")));
    }
    let bytes = out.write()?;
    Nsbmd::parse(&bytes)?;
    edits.insert(path.into(), engine::wrap(p.bytes(s, path)?, bytes));
    p.edit_many(edits, "Import model", s)
}
pub(crate) fn texture_path(s: &RomSession, path: &str) -> Option<String> {
    mkds_data::assets::texture_candidates(path)
        .iter()
        .find_map(|candidate| {
            s.nodes
                .iter()
                .find(|n| n.path.eq_ignore_ascii_case(candidate))
                .map(|n| n.path.clone())
        })
}
