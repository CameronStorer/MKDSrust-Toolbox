//! Beginner recipes produce complete related file sets, committed as one history command.
use crate::{model, preview, project::Project, session::RomSession, Error, Result};
use nitro_formats::{
    math,
    model::{Model, Nsbmd},
    texture::Nsbtx,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Type)]
pub struct CharacterOptions {
    pub slot: u32,
    pub base: u32,
    pub source: String,
    /// Fraction of source height to remove before fitting (zero skips cutting).
    pub hips: f32,
    pub rotation: [f32; 4],
}
#[derive(Clone, Debug, Deserialize, Type)]
pub struct KartOptions {
    pub slot: u32,
    pub base: u32,
    pub source: String,
    pub tire: String,
    /// Kart-object units as displayed in the offset table, FL/FR/RL/RR.
    pub wheels: Option<[[f32; 3]; 4]>,
    pub seat: [f32; 3],
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: f32,
}
#[derive(Clone, Debug, Serialize, Type, Default)]
pub struct Report {
    pub files: Vec<String>,
    pub warnings: Vec<String>,
    pub polygons: u32,
}
#[derive(Clone, Debug, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum Action {
    Character {
        options: CharacterOptions,
    },
    Kart {
        options: KartOptions,
    },
    Track {
        options: crate::track_wizard::TrackOptions,
    },
    Cup {
        slot: u32,
        courses: [u32; 4],
    },
    Music {
        course: u32,
        source: String,
        options: crate::engine::SongOptions,
    },
    Name {
        path: String,
        index: u32,
        text: String,
    },
    Icon {
        tiles: String,
        palette: String,
        cells: String,
        index: u32,
        source: String,
    },
    Rig {
        path: String,
        part: u32,
        vertices: Vec<u32>,
        bone: u32,
    },
    RigPaint {
        path: String,
        part: u32,
        triangles: Vec<u32>,
        bone: u32,
    },
    Checkpoint {
        path: String,
        index: u32,
        left: [f32; 2],
        right: [f32; 2],
    },
    Table {
        overlay: Option<u32>,
        address: u32,
        values: Vec<crate::tables::Value>,
        uncompressed: bool,
    },
    NkmPoint {
        path: String,
        section: String,
        index: u32,
        position: [f32; 3],
    },
}
pub fn apply(p: &mut Project, s: &RomSession, action: Action) -> Result<Report> {
    // Preparation is memory-only. Failure never leaves half a recipe applied.
    let mut draft = Project::in_memory(s, p.config.manifest.clone(), p.edits.clone())?;
    let before = draft.edits.clone();
    let mut report = match action {
        Action::Character { options } => character(&mut draft, s, &options)?,
        Action::Kart { options } => kart(&mut draft, s, &options)?,
        Action::Track { options } => crate::track_wizard::track(&mut draft, s, &options)?,
        Action::Cup { slot, courses } => {
            crate::tables::set_cup(&mut draft, s, slot, courses)?;
            Report::default()
        }
        Action::Music {
            course,
            source,
            options,
        } => {
            let song = crate::engine::import_song(
                &mut draft,
                s,
                "data/Sound/sound_data.sdat",
                std::path::Path::new(&source),
                &options,
            )?;
            let sequence = u8::try_from(options.sequence)
                .map_err(|_| invalid("music sequence must fit one byte"))?;
            crate::tables::assign_music(&mut draft, s, course, sequence)?;
            Report {
                warnings: vec![format!(
                    "Song uses {} KiB / 700 KiB sound heap; aim below 250 KiB of samples",
                    song.ram_bytes.div_ceil(1024)
                )],
                ..Report::default()
            }
        }
        Action::Name { path, index, text } => {
            crate::names::rename(&mut draft, s, &path, index as usize, &text)?;
            Report::default()
        }
        Action::Icon {
            tiles,
            palette,
            cells,
            index,
            source,
        } => {
            let image = crate::graphic::read_png(std::path::Path::new(&source))?;
            crate::icon::edit(&mut draft, s, &tiles, &palette, &cells, index, &image)?;
            Report::default()
        }
        Action::Rig {
            path,
            part,
            vertices,
            bone,
        } => {
            let mut m = loaded(&draft, s, &path)?;
            reassign(
                m.models
                    .first_mut()
                    .ok_or_else(|| invalid("model is empty"))?,
                part as usize,
                &vertices,
                bone as usize,
            )?;
            draft.edit(&path, m.write()?, "Paint bone assignment", s)?;
            Report::default()
        }
        Action::RigPaint {
            path,
            part,
            triangles,
            bone,
        } => {
            let mut model = loaded(&draft, s, &path)?;
            let m = model
                .models
                .first_mut()
                .ok_or_else(|| invalid("empty model"))?;
            let mesh = m
                .meshes
                .get(part as usize)
                .ok_or_else(|| invalid("choose a model part"))?;
            let mut vertices = vec![];
            for triangle in triangles {
                let at = triangle as usize * 3;
                let ids = mesh
                    .indices
                    .get(at..at + 3)
                    .ok_or_else(|| invalid("triangle outside model"))?;
                vertices.extend_from_slice(ids);
            }
            reassign(m, part as usize, &vertices, bone as usize)?;
            draft.edit(&path, model.write()?, "Paint bone assignments", s)?;
            Report::default()
        }
        Action::Checkpoint {
            path,
            index,
            left,
            right,
        } => {
            let mut n = nitro_formats::nkm::Nkm::parse(draft.bytes(s, &path)?)?;
            crate::track_wizard::move_checkpoint(&mut n, index as usize, left, right)?;
            draft.edit(&path, n.write()?, "Move checkpoint gate", s)?;
            Report::default()
        }
        Action::Table {
            overlay,
            address,
            values,
            uncompressed,
        } => {
            crate::tables::edit(&mut draft, s, overlay, address, &values)?;
            if overlay.is_none() && uncompressed {
                let path = nitro_formats::executable::ARM9;
                let mut bytes = draft.bytes(s, path)?.to_vec();
                let at = nitro_formats::executable::module_params(&bytes)
                    .ok_or_else(|| invalid("module parameters not found"))?;
                bytes[at + 20..at + 24].fill(0);
                draft.edit(path, bytes, "Store ARM9 uncompressed", s)?;
            }
            Report::default()
        }
        Action::NkmPoint {
            path,
            section,
            index,
            position,
        } => {
            if ![
                "KTPS", "KTP2", "KTPJ", "IPOI", "EPOI", "CAME", "OBJI", "POIT",
            ]
            .contains(&section.as_str())
            {
                return Err(invalid("select an editable position section"));
            }
            let mut n = nitro_formats::nkm::Nkm::parse(draft.bytes(s, &path)?)?;
            let record = n
                .sections
                .iter_mut()
                .find(|s| s.kind == section)
                .and_then(|s| s.entries.get_mut(index as usize))
                .ok_or_else(|| invalid("point index outside NKM"))?;
            crate::tables::write(record, 0, 0, &position.map(crate::tables::Value::Fx32))?;
            draft.edit(&path, n.write()?, "Move track point", s)?;
            Report::default()
        }
    };
    let mut edits = draft
        .edits
        .iter()
        .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect::<BTreeMap<_, _>>();
    for path in before
        .keys()
        .filter(|path| !draft.edits.contains_key(*path))
    {
        edits.insert(path.clone(), s.file(path)?.to_vec());
    }
    report.files = edits.keys().cloned().collect();
    p.edit_many(edits, "Beginner recipe", s)?;
    Ok(report)
}
fn invalid(s: &str) -> Error {
    Error::Invalid(s.into())
}
pub fn character_id(id: u32) -> Result<(&'static str, &'static str)> {
    mkds_data::CHARACTERS
        .get(id as usize)
        .copied()
        .ok_or_else(|| invalid("choose a character slot 0..12"))
}
pub fn kart_id(id: u32) -> Result<(String, &'static str)> {
    if id > 36 {
        return Err(invalid("choose a kart id 0..36"));
    }
    if id.is_multiple_of(3) {
        return Ok(("MR_a".into(), "mario"));
    }
    let (code, dir) = character_id(id / 3)?;
    Ok((
        format!("{code}_{}", if id % 3 == 1 { 'b' } else { 'c' }),
        dir,
    ))
}
fn loaded(p: &Project, s: &RomSession, path: &str) -> Result<Nsbmd> {
    Ok(Nsbmd::parse(&preview::plain(p.bytes(s, path)?)?)?)
}
pub fn bounds(m: &Model) -> Result<([f32; 3], [f32; 3])> {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for v in m.meshes.iter().flat_map(|m| &m.positions) {
        for k in 0..3 {
            if !v[k].is_finite() {
                return Err(invalid("model has non-finite positions"));
            }
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    if !lo[0].is_finite() {
        return Err(invalid("model has no vertices"));
    }
    Ok((lo, hi))
}
fn fit(m: &mut Model, donor: &Model) -> Result<()> {
    let (lo, hi) = bounds(m)?;
    let (dl, dh) = bounds(donor)?;
    let height = hi[1] - lo[1];
    if height < 1e-6 {
        return Err(invalid("model has no height; rotate it upright"));
    }
    let scale = (dh[1] - dl[1]) / height;
    let t = [
        -(lo[0] + hi[0]) * 0.5 * scale,
        dl[1] - lo[1] * scale,
        -(lo[2] + hi[2]) * 0.5 * scale + (dl[2] + dh[2]) * 0.5,
    ];
    for mesh in &mut m.meshes {
        nitro_formats::model::transform(mesh, math::trs(t, [0., 0., 0., 1.], [scale; 3]))?;
    }
    Ok(())
}
fn reduce(m: &mut Model, target: usize, vertices: usize) -> Result<()> {
    let total: usize = m.meshes.iter().map(|m| m.indices.len() / 3).sum();
    for mesh in &mut m.meshes {
        let share = ((mesh.indices.len() / 3) * target / total.max(1)).max(1);
        if mesh.indices.len() / 3 > share {
            nitro_formats::model::decimate(mesh, share)?;
        }
    }
    let tris: usize = m.meshes.iter().map(|m| m.indices.len() / 3).sum();
    let verts: usize = m.meshes.iter().map(|m| m.positions.len()).sum();
    if tris > target || verts > vertices {
        return Err(invalid(
            "safe reduction could not reach the slot budget; simplify the mesh in Model and retry",
        ));
    }
    Ok(())
}
pub fn rigid_rig(m: &mut Model, donor: &Model) -> Result<()> {
    m.bones = donor.bones.clone();
    m.matrix_count = m.bones.len();
    let worlds = model::bone_world(m);
    let origins: Vec<_> = worlds.iter().map(|w| math::point(*w, [0.; 3])).collect();
    for mesh in &mut m.meshes {
        mesh.joints = mesh
            .positions
            .iter()
            .map(|p| {
                let mut best = (f32::INFINITY, 0);
                // Use bone segments, rather than points, to avoid assigning torsos to elbows.
                for (i, b) in m.bones.iter().enumerate().skip(1) {
                    let end = origins[i];
                    let start = b.parent.map_or(end, |p| origins[p]);
                    let d = std::array::from_fn(|k| end[k] - start[k]);
                    let v = std::array::from_fn(|k| p[k] - start[k]);
                    let t = (math::dot(v, d) / math::dot(d, d).max(1e-10)).clamp(0., 1.);
                    let distance = (0..3)
                        .map(|k| (p[k] - start[k] - d[k] * t).powi(2))
                        .sum::<f32>();
                    if distance < best.0 {
                        best = (distance, i);
                    }
                }
                [best.1 as u16, 0, 0, 0]
            })
            .collect();
        mesh.weights = vec![[1., 0., 0., 0.]; mesh.positions.len()];
    }
    Ok(())
}
pub fn reassign(m: &mut Model, part: usize, vertices: &[u32], bone: usize) -> Result<()> {
    if bone >= m.bones.len() {
        return Err(invalid("choose a bone from the base skeleton"));
    }
    let mesh = m
        .meshes
        .get_mut(part)
        .ok_or_else(|| invalid("choose a mesh part"))?;
    if vertices.iter().any(|&i| i as usize >= mesh.positions.len()) {
        return Err(invalid("vertex index outside mesh"));
    }
    if mesh.joints.len() != mesh.positions.len() {
        return Err(invalid("auto-rig the model first"));
    }
    for &i in vertices {
        mesh.joints[i as usize] = [bone as u16, 0, 0, 0];
        mesh.weights[i as usize] = [1., 0., 0., 0.];
    }
    Ok(())
}
fn texture_image(source: &Nsbmd, index: usize) -> Result<nitro_formats::graphics::Image> {
    if let Some(tx) = &source.textures {
        if !tx.textures.is_empty() && !tx.palettes.is_empty() {
            return Ok(tx.decode(
                index.min(tx.textures.len() - 1),
                index.min(tx.palettes.len() - 1),
            )?);
        }
    }
    let color = source.models[0]
        .materials
        .get(index)
        .map_or([0.5, 0.7, 0.9, 1.], |m| m.diffuse);
    let rgba: Vec<_> = (0..64)
        .flat_map(|_| color.map(|c| (c.clamp(0., 1.) * 255.) as u8))
        .collect();
    Ok(nitro_formats::graphics::Image {
        width: 8,
        height: 8,
        rgba,
    })
}
fn driver_materials(
    m: &mut Model,
    donor: &Model,
    source: &Nsbmd,
    donor_tx: &Nsbtx,
) -> Result<Nsbtx> {
    let mut textures = vec![];
    let mut palettes = vec![];
    for (i, t) in donor_tx.textures.iter().enumerate() {
        let image = texture_image(
            source,
            usize::from(t.name.to_ascii_lowercase().contains("face")),
        )?;
        let (mut texture, mut palette) = model::convert_texture(&image, &t.name, 3, 32)?;
        texture.name = t.name.clone();
        palette.name = donor_tx
            .palettes
            .get(i)
            .or(donor_tx.palettes.first())
            .map_or_else(|| t.name.clone(), |p| p.name.clone());
        textures.push(texture);
        if !palettes
            .iter()
            .any(|p: &nitro_formats::texture::Palette| p.name == palette.name)
        {
            palettes.push(palette);
        }
    }
    m.materials = donor.materials.clone();
    for mesh in &mut m.meshes {
        let face = mesh.name.to_ascii_lowercase().contains("face");
        mesh.material = if face {
            m.materials
                .iter()
                .position(|m| m.name.to_ascii_lowercase().contains("face"))
                .unwrap_or(0)
        } else {
            0
        };
    }
    Ok(Nsbtx::new(textures, palettes)?)
}
fn existing_model_path(s: &RomSession, suffix: &str, menu: bool) -> Result<String> {
    s.nodes
        .iter()
        .find(|n| {
            n.path.ends_with(suffix)
                && n.path.contains("KartModelMenu") == menu
                && !n.path.contains("KartModelMain.carc")
        })
        .map(|n| n.path.clone())
        .ok_or_else(|| invalid("base/slot model is absent from the ROM"))
}
pub fn character(p: &mut Project, s: &RomSession, o: &CharacterOptions) -> Result<Report> {
    let source = crate::model_import::import(std::path::Path::new(&o.source))?;
    character_from(p, s, o, source)
}
pub fn character_from(
    p: &mut Project,
    s: &RomSession,
    o: &CharacterOptions,
    source: Nsbmd,
) -> Result<Report> {
    let (slot, dir) = character_id(o.slot)?;
    let (base, base_dir) = character_id(o.base)?;
    if !o.hips.is_finite() || !(0. ..0.95).contains(&o.hips) {
        return Err(invalid("hip cut must be a fraction from 0 to 0.95"));
    }
    let base_path = existing_model_path(s, &format!("character/{base_dir}/P_{base}.nsbmd"), false)?;
    let donor = loaded(p, s, &base_path)?;
    let donor = &donor.models[0];
    let tx = Nsbtx::parse(p.bytes(s, &base_path.replace(".nsbmd", ".nsbtx"))?)?;
    let mut m = source
        .models
        .first()
        .ok_or_else(|| invalid("source has no model"))?
        .clone();
    for mesh in &mut m.meshes {
        nitro_formats::model::transform(mesh, math::trs([0.; 3], o.rotation, [1.; 3]))?;
    }
    if o.hips > 0. {
        let (lo, hi) = bounds(&m)?;
        for mesh in &mut m.meshes {
            nitro_formats::model::cut_plane(
                mesh,
                [0., 1., 0.],
                lo[1] + (hi[1] - lo[1]) * o.hips,
                true,
            )?;
        }
    }
    fit(&mut m, donor)?;
    reduce(&mut m, 237, 536)?;
    rigid_rig(&mut m, donor)?;
    let tx = driver_materials(&mut m, donor, &source, &tx)?;
    m.name = format!("P_{slot}");
    let b = Nsbmd {
        models: vec![m.clone()],
        textures: None,
    }
    .write()?;
    let slot_path = existing_model_path(s, &format!("character/{dir}/P_{slot}.nsbmd"), false)?;
    let menu_path = existing_model_path(s, &format!("character/{dir}/P_{slot}.nsbmd"), true).ok();
    let mut edits = BTreeMap::new();
    for path in std::iter::once(&slot_path).chain(menu_path.iter()) {
        edits.insert(path.clone(), b.clone());
        edits.insert(path.replace(".nsbmd", ".nsbtx"), tx.write());
        for clip in ["drive", "spin", "win", "lose"] {
            let dest = path.replace(".nsbmd", &format!("_{clip}.nsbca"));
            if s.file(&dest).is_ok() {
                let from = base_path.replace(".nsbmd", &format!("_{clip}.nsbca"));
                edits.insert(dest, p.bytes(s, &from)?.to_vec());
            }
        }
    }
    let epath = existing_model_path(s, &format!("character/{dir}/E_{slot}.nsbmd"), false)?;
    let edonor = loaded(p, s, &epath)?;
    let mut e = m.clone();
    e.name = format!("E_{slot}");
    e.bones = edonor.models[0].bones.clone();
    e.matrix_count = 1;
    reduce(&mut e, 171, 385)?;
    for mesh in &mut e.meshes {
        mesh.joints = vec![[0; 4]; mesh.positions.len()];
        mesh.weights = vec![[1., 0., 0., 0.]; mesh.positions.len()];
    }
    // Keep E texture names and the runtime face lookup contract.
    let etx = Nsbtx::parse(p.bytes(s, &epath.replace(".nsbmd", ".nsbtx"))?)?;
    let etx = driver_materials(&mut e, &edonor.models[0], &source, &etx)?;
    edits.insert(epath.replace(".nsbmd", ".nsbtx"), etx.write());
    edits.insert(
        epath,
        Nsbmd {
            models: vec![e],
            textures: None,
        }
        .write()?,
    );
    for archive in ["KartModelMainA", "KartModelMenu"] {
        let path = format!("data/{archive}.carc/kartoffsetdata.bin");
        let mut bytes = p.bytes(s, &path)?.to_vec();
        for kart in 0..37 {
            let start = kart * 224 + 16 + 13 * 4;
            let donor = start + o.base as usize * 12;
            let target = start + o.slot as usize * 12;
            let seat = bytes[donor..donor + 12].to_vec();
            bytes[target..target + 12].copy_from_slice(&seat);
        }
        edits.insert(path, bytes);
    }
    let report = Report {
        files: edits.keys().cloned().collect(),
        polygons: m.meshes.iter().map(|m| m.indices.len() as u32 / 3).sum(),
        warnings: if o.base != o.slot {
            vec!["Voice remains selected by the destination character id; donor voice routing is not yet verified".into()]
        } else {
            vec![]
        },
    };
    p.edit_many(edits, "New character", s)?;
    Ok(report)
}
pub fn kart(p: &mut Project, s: &RomSession, o: &KartOptions) -> Result<Report> {
    let source = crate::model_import::import(std::path::Path::new(&o.source))?;
    kart_from(p, s, o, source)
}
pub fn kart_from(
    p: &mut Project,
    s: &RomSession,
    o: &KartOptions,
    source: Nsbmd,
) -> Result<Report> {
    let (slot, dir) = kart_id(o.slot)?;
    let (base, base_dir) = kart_id(o.base)?;
    if !["S", "M", "L"].contains(&o.tire.as_str()) || !o.scale.is_finite() || o.scale <= 0. {
        return Err(invalid("choose S/M/L tires and a positive placement scale"));
    }
    let base_path = existing_model_path(s, &format!("kart/{base_dir}/kart_{base}.nsbmd"), false)?;
    let donor = loaded(p, s, &base_path)?;
    let donor = &donor.models[0];
    let mut m = source
        .models
        .first()
        .ok_or_else(|| invalid("source has no model"))?
        .clone();
    fit(&mut m, donor)?;
    for mesh in &mut m.meshes {
        nitro_formats::model::transform(mesh, math::trs(o.translation, o.rotation, [o.scale; 3]))?;
        mesh.material = 0;
        mesh.joints = vec![[0; 4]; mesh.positions.len()];
        mesh.weights = vec![[1., 0., 0., 0.]; mesh.positions.len()];
    }
    reduce(&mut m, 180, 330)?;
    m.name = format!("kart_{slot}");
    m.bones = donor.bones.clone();
    m.materials = donor.materials.clone();
    m.matrix_count = 1;
    // Retain the separate runtime emblem and baked low-detail tire shapes.
    m.meshes.extend(
        donor
            .meshes
            .iter()
            .filter(|mesh| mesh.material != 0)
            .cloned(),
    );
    let mut tx = Nsbtx::parse(p.bytes(s, &base_path.replace(".nsbmd", ".nsbtx"))?)?;
    if let Some(t) = tx.textures.first() {
        let (mut texture, mut palette) =
            model::convert_texture(&texture_image(&source, 0)?, &t.name, 3, 32)?;
        texture.name = t.name.clone();
        palette.name = tx.palettes[0].name.clone();
        tx.textures[0] = texture;
        tx.palettes[0] = palette;
    }
    let bytes = Nsbmd {
        models: vec![m.clone()],
        textures: None,
    }
    .write()?;
    let mut edits = BTreeMap::new();
    for menu in [false, true] {
        let path = existing_model_path(s, &format!("kart/{dir}/kart_{slot}.nsbmd"), menu)?;
        edits.insert(path.replace(".nsbmd", ".nsbtx"), tx.write());
        edits.insert(path, bytes.clone());
        let shadow_suffix = format!("kart/shadow/sh_{slot}.nsbmd");
        if let Ok(shadow_path) = existing_model_path(s, &shadow_suffix, menu) {
            let mut shadow = loaded(p, s, &shadow_path)?;
            let sm = shadow
                .models
                .first_mut()
                .ok_or_else(|| invalid("base shadow is empty"))?;
            let (lo, hi) = bounds(&m)?;
            let mesh = nitro_formats::model::Mesh {
                name: "footprint".into(),
                positions: vec![
                    [lo[0], 0., lo[2]],
                    [hi[0], 0., lo[2]],
                    [hi[0], 0., hi[2]],
                    [lo[0], 0., hi[2]],
                ],
                indices: vec![0, 2, 1, 0, 3, 2],
                material: 0,
                ..Default::default()
            };
            sm.meshes = vec![mesh];
            sm.materials.truncate(1);
            sm.bones.truncate(1);
            sm.matrix_count = 1;
            edits.insert(shadow_path, shadow.write()?);
        }
    }
    for archive in ["KartModelMainA", "KartModelMenu"] {
        for (file, stride) in [
            ("kartoffsetdata.bin", 224),
            (
                "kartphysicalparam.bin",
                if archive == "KartModelMainA" {
                    152
                } else {
                    148
                },
            ),
            ("kartmenuparam.bin", 12),
            ("kartAIparam.bin", 160),
        ] {
            let path = format!("data/{archive}.carc/{file}");
            if s.file(&path).is_err() {
                continue;
            }
            let mut b = p.bytes(s, &path)?.to_vec();
            let from = o.base as usize * stride;
            let to = o.slot as usize * stride;
            if from + stride > b.len() || to + stride > b.len() {
                return Err(invalid("kart table row outside verified table"));
            }
            let row = b[from..from + stride].to_vec();
            b[to..to + stride].copy_from_slice(&row);
            if file == "kartoffsetdata.bin" {
                b[to..to + 16].fill(0);
                let name = format!("kart_tire_{}", o.tire);
                b[to..to + name.len()].copy_from_slice(name.as_bytes());
                let scale = match o.tire.as_str() {
                    "S" => 1.,
                    "L" => 0.8,
                    _ => 0.6,
                };
                crate::tables::write(
                    &mut b,
                    0,
                    (to + 16) as u32,
                    &[crate::tables::Value::Fx32(scale)],
                )?;
                if let Some(wheels) = o.wheels {
                    for (i, wheel) in wheels.iter().enumerate() {
                        crate::tables::write(
                            &mut b,
                            0,
                            (to + 20 + i * 12) as u32,
                            &wheel.map(crate::tables::Value::Fx32),
                        )?;
                    }
                }
                for char in 0..13 {
                    for k in 0..3 {
                        let at = to + 16 + 13 * 4 + char * 12 + k * 4;
                        let v = i32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as f32
                            / 4096.
                            + o.seat[k];
                        crate::tables::write(
                            &mut b,
                            0,
                            at as u32,
                            &[crate::tables::Value::Fx32(v)],
                        )?;
                    }
                }
            }
            edits.insert(path, b);
        }
    }
    let polygons = m.meshes.iter().map(|m| m.indices.len() as u32 / 3).sum();
    let report = Report {
        files: edits.keys().cloned().collect(),
        polygons,
        warnings: if o.slot.is_multiple_of(3) {
            vec!["Standard body geometry is shared by all standard kart ids".into()]
        } else {
            vec![]
        },
    };
    p.edit_many(edits, "New kart", s)?;
    Ok(report)
}
