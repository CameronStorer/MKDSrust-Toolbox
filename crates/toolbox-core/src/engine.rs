//! Project-facing authoring commands. Every mutation uses the shared history
//! and archive rebuild pipeline, including compressed nested members.
use crate::{
    audio, collision, model_import, preview, project::Project, session::RomSession, Error, Result,
};
use nitro_formats::{
    audio::{Sbnk, Sdat, Sseq},
    model::{self, Nsbmd},
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
#[derive(Clone, Debug, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum MeshSelection {
    Part,
    Material {
        material: u32,
    },
    Connected {
        triangle: u32,
    },
    Box {
        minimum: [f32; 3],
        maximum: [f32; 3],
    },
    Triangles {
        triangles: Vec<u32>,
    },
}
#[derive(Clone, Debug, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum ModelAction {
    Edit {
        part: u32,
        selection: crate::mesh_edit::Selection,
        operation: crate::mesh_edit::Operation,
    },
    Import {
        source: String,
    },
    Transform {
        part: u32,
        translation: [f32; 3],
        rotation: [f32; 4],
        scale: [f32; 3],
    },
    Decimate {
        part: u32,
        target: u32,
    },
    Cut {
        part: u32,
        normal: [f32; 3],
        distance: f32,
        positive: bool,
    },
    Delete {
        part: u32,
        selection: MeshSelection,
    },
    Merge {
        part: u32,
        distance: f32,
    },
    Normals {
        part: u32,
    },
}
#[derive(Clone, Debug, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum MaterialAction {
    Replace { source: String },
    Recolour { index: u32, color: u16 },
    Tint { color: u16 },
}
pub fn material(
    p: &mut Project,
    s: &RomSession,
    path: &str,
    material: u32,
    action: MaterialAction,
) -> Result<()> {
    let scene = crate::scene::CourseScene::load(Some(p), s, path)?;
    let detail = scene
        .info
        .material_details
        .get(material as usize)
        .ok_or_else(|| Error::Invalid("choose a material".into()))?;
    let dest = detail
        .path
        .as_ref()
        .ok_or_else(|| Error::Invalid("material has no bound texture".into()))?;
    let mut container = scene
        .model
        .ok_or_else(|| Error::Invalid("no model".into()))?;
    let mut tx = container
        .textures
        .as_ref()
        .or(scene.textures.as_ref())
        .ok_or_else(|| Error::Invalid("missing texture bank".into()))?
        .clone();
    match action {
        MaterialAction::Recolour { index, color } => {
            if detail.format == 7 || detail.palette.is_none() {
                return Err(Error::Invalid(
                    "This texture has no palette; use Tint instead".into(),
                ));
            }
            let c = tx
                .palettes
                .get_mut(detail.palette_index as usize)
                .and_then(|p| p.colors.get_mut(index as usize))
                .ok_or_else(|| Error::Invalid("choose a palette colour".into()))?;
            *c = color & 0x7fff;
        }
        MaterialAction::Tint { color } => {
            let mut image =
                tx.decode(detail.texture_index as usize, detail.palette_index as usize)?;
            let c = nitro_formats::graphics::bgr555_to_rgba(color);
            for p in image.rgba.chunks_exact_mut(4) {
                p[..3].copy_from_slice(&c[..3]);
            }
            let old = &tx.textures[detail.texture_index as usize];
            let (t, _) = crate::model::convert_texture(&image, &old.name, 7, 1024)?;
            tx.textures[detail.texture_index as usize] = t;
        }
        MaterialAction::Replace { source } => {
            let image = crate::graphic::read_png(Path::new(&source))?;
            let old = &tx.textures[detail.texture_index as usize];
            let (t, mut pal) = crate::model::convert_texture(
                &image,
                &old.name,
                if old.format == 5 { 3 } else { old.format },
                1024,
            )?;
            for model in &mut container.models {
                for mat in &mut model.materials {
                    if mat.texture.as_ref() == Some(&t.name) {
                        mat.width = t.width as u16;
                        mat.height = t.height as u16;
                    }
                }
            }
            tx.textures[detail.texture_index as usize] = t;
            if detail.format != 7 {
                let old = tx
                    .palettes
                    .get_mut(detail.palette_index as usize)
                    .ok_or_else(|| Error::Invalid("missing palette binding".into()))?;
                pal.name = old.name.clone();
                *old = pal;
            }
        }
    }
    // Rebuild through the common writer, including for embedded TEX0 resources.
    let tx = nitro_formats::texture::Nsbtx::new(tx.textures, tx.palettes)?;
    let mut edits = std::collections::BTreeMap::new();
    if dest == path {
        container.textures = Some(tx);
    } else {
        edits.insert(dest.clone(), wrap(p.bytes(s, dest)?, tx.write()));
    }
    let external = if dest == path {
        None
    } else {
        Some(nitro_formats::texture::Nsbtx::parse(&preview::plain(
            &edits[dest],
        )?)?)
    };
    let budget = crate::model::budget(&container, external.as_ref());
    if !budget.reasons.is_empty() {
        return Err(Error::Invalid(budget.reasons.join("; ")));
    }
    let bytes = container.write()?;
    Nsbmd::parse(&bytes)?;
    edits.insert(path.into(), wrap(p.bytes(s, path)?, bytes));
    p.edit_many(edits, "Edit model texture", s)
}
pub(crate) fn wrap(original: &[u8], bytes: Vec<u8>) -> Vec<u8> {
    if original.first() == Some(&0x10) {
        nitro_formats::recompress(original, &bytes)
    } else {
        bytes
    }
}
pub fn model(p: &mut Project, s: &RomSession, path: &str, action: ModelAction) -> Result<()> {
    let original = p.bytes(s, path)?;
    let mut container = Nsbmd::parse(&preview::plain(original)?)?;
    if container.models.len() != 1 {
        return Err(Error::Invalid("This file contains multiple models. Export a model to GLB and edit it separately before replacing this set.".into()));
    }
    let title;
    match action {
        ModelAction::Import { source } => {
            container = model_import::import(Path::new(&source))?;
            title = "Import model";
        }
        action => {
            let m = container
                .models
                .first_mut()
                .ok_or_else(|| Error::Invalid("model has no geometry".into()))?;
            let part = match &action {
                ModelAction::Transform { part, .. }
                | ModelAction::Decimate { part, .. }
                | ModelAction::Cut { part, .. }
                | ModelAction::Delete { part, .. }
                | ModelAction::Merge { part, .. }
                | ModelAction::Normals { part } => *part as usize,
                ModelAction::Edit { part, .. } => *part as usize,
                _ => unreachable!(),
            };
            let mesh = m
                .meshes
                .get_mut(part)
                .ok_or_else(|| Error::Invalid("choose a model part".into()))?;
            title = match action {
                ModelAction::Edit {
                    selection,
                    operation,
                    ..
                } => {
                    crate::mesh_edit::apply(mesh, &selection, &operation)?;
                    operation.title()
                }
                ModelAction::Transform {
                    translation,
                    rotation,
                    scale,
                    ..
                } => {
                    model::transform(mesh, nitro_formats::math::trs(translation, rotation, scale))?;
                    "Transform model"
                }
                ModelAction::Decimate { target, .. } => {
                    model::decimate(mesh, target as usize)?;
                    "Decimate model"
                }
                ModelAction::Cut {
                    normal,
                    distance,
                    positive,
                    ..
                } => {
                    model::cut_plane(mesh, normal, distance, positive)?;
                    "Cut model"
                }
                ModelAction::Delete { selection, .. } => {
                    let triangles = match selection {
                        MeshSelection::Part => (0..mesh.indices.len() / 3).collect(),
                        MeshSelection::Connected { triangle } => {
                            model::connected_part(mesh, triangle as usize)?
                        }
                        MeshSelection::Box { minimum, maximum } => {
                            model::select_box(mesh, minimum, maximum)
                        }
                        MeshSelection::Triangles { triangles } => {
                            triangles.into_iter().map(|i| i as usize).collect()
                        }
                        MeshSelection::Material { material } => {
                            for m in m
                                .meshes
                                .iter_mut()
                                .filter(|m| m.material == material as usize)
                            {
                                m.indices.clear();
                            }
                            vec![]
                        }
                    };
                    if !triangles.is_empty() {
                        model::delete_triangles(&mut m.meshes[part], &triangles)?;
                    }
                    "Delete model selection"
                }
                ModelAction::Merge { distance, .. } => {
                    model::merge_vertices(mesh, distance)?;
                    "Merge vertices"
                }
                ModelAction::Normals { .. } => {
                    model::recompute_normals(mesh)?;
                    "Recompute normals"
                }
                _ => unreachable!(),
            };
        }
    }
    let bytes = container.write()?;
    let external = if container.textures.is_none() {
        crate::linked::texture_path(s, path)
            .map(|candidate| {
                Ok::<_, Error>(nitro_formats::texture::Nsbtx::parse(&preview::plain(
                    p.bytes(s, &candidate)?,
                )?)?)
            })
            .transpose()?
    } else {
        None
    };
    let b = crate::model::budget(&container, external.as_ref());
    if !b.reasons.is_empty() {
        return Err(Error::Invalid(b.reasons.join("; ")));
    }
    Nsbmd::parse(&bytes)?;
    p.edit(path, wrap(original, bytes), title, s)
}
pub fn generate_collision(
    p: &mut Project,
    s: &RomSession,
    model_path: &str,
    path: &str,
) -> Result<()> {
    let original = p.bytes(s, path)?;
    let m = Nsbmd::parse(&preview::plain(p.bytes(s, model_path)?)?)?;
    let model = m
        .models
        .first()
        .ok_or_else(|| Error::Invalid("model contains no geometry".into()))?;
    let k = collision::generate(model, &collision::AttributeTable::default())?;
    p.edit(path, wrap(original, k.write()), "Generate collision", s)
}
pub fn paint_collision(
    p: &mut Project,
    s: &RomSession,
    path: &str,
    triangles: &[u32],
    attribute: u16,
) -> Result<()> {
    let original = p.bytes(s, path)?;
    let mut k = nitro_formats::kcl::Kcl::parse(&preview::plain(original)?)?;
    k.repaint(
        &triangles.iter().map(|i| *i as usize).collect::<Vec<_>>(),
        attribute,
    )?;
    p.edit(path, wrap(original, k.write()), "Paint collision", s)
}
#[derive(Clone, Debug, Deserialize, Type)]
pub struct SongOptions {
    pub sequence: u32,
    pub bank: u32,
    pub route: String,
    pub rate: u16,
    pub stereo: bool,
    pub encoding: u8,
    pub loop_start: Option<u32>,
    pub loop_end: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct SongReport {
    pub ram_bytes: u32,
    pub loop_sample: Option<u32>,
    pub route: String,
}
pub fn import_song(
    p: &mut Project,
    s: &RomSession,
    path: &str,
    source: &Path,
    options: &SongOptions,
) -> Result<SongReport> {
    let original = p.bytes(s, path)?;
    let mut sdat = Sdat::parse(&preview::plain(original)?)?;
    let bytes = std::fs::read(source)?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err(Error::Invalid("song exceeds 128 MiB".into()));
    }
    let report;
    if bytes.starts_with(b"MThd") {
        if !["auto", "midi"].contains(&options.route.as_str()) {
            return Err(Error::Invalid(
                "choose the MIDI route for a MIDI file".into(),
            ));
        }
        let bank = Sbnk::parse(&sdat.files[sdat.resource_file(2, options.bank as usize)?])?;
        let seq = audio::midi(&bytes, &audio::program_map(&bank)?)?;
        let id = sdat.resource_file(0, options.sequence as usize)?;
        sdat.files[id] = seq.write();
        let record = sdat.info[0][options.sequence as usize].as_mut().unwrap();
        record[4..6].copy_from_slice(&(options.bank as u16).to_le_bytes());
        report = SongReport {
            ram_bytes: (sdat.files[id].len() + bank.write()?.len()) as u32,
            loop_sample: None,
            route: "midi".into(),
        };
    } else {
        if !["auto", "sampled"].contains(&options.route.as_str()) {
            return Err(Error::Invalid(
                "choose the sampled route for a WAV file".into(),
            ));
        }
        let pcm = audio::read_wav(&bytes)?;
        let song = audio::sampled(
            &pcm,
            options.rate,
            options.stereo,
            options.encoding,
            options.loop_start,
            options.loop_end,
        )?;
        if song.ram_bytes > mkds_data::SOUND_HEAP as usize {
            return Err(Error::Invalid(format!("Sampled song needs {} bytes; reduce rate, use mono or trim below the 700 KiB sound heap (aim under 250 KiB)",song.ram_bytes)));
        }
        report = SongReport {
            ram_bytes: song.ram_bytes as u32,
            loop_sample: song.loop_sample.map(|v| v as u32),
            route: "sampled".into(),
        };
        sdat.replace_sampled(
            options.sequence as usize,
            song.sequence,
            song.bank,
            song.waves,
        )?;
    }
    let bytes = sdat.write()?;
    let _ = Sdat::parse(&bytes)?;
    p.edit(path, wrap(original, bytes), "Import song", s)?;
    Ok(report)
}
pub fn preview_audio(
    bytes: &[u8],
    kind: u32,
    index: u32,
    sample: u32,
    seconds: u32,
) -> Result<Vec<u8>> {
    let sdat = Sdat::parse(&preview::plain(bytes)?)?;
    let duration = if seconds == 0 {
        300
    } else {
        seconds.clamp(1, 300)
    };
    let pcm = match kind {
        0 => audio::preview_sequence(
            &sdat,
            index as usize,
            if seconds == 0 { 0. } else { duration as f32 },
        )?,
        3 => {
            let waves = nitro_formats::audio::Swar::parse(
                &sdat.files[sdat.resource_file(3, index as usize)?],
            )?;
            let wave = waves
                .waves
                .get(sample as usize)
                .ok_or_else(|| Error::Invalid("sample index outside wave archive".into()))?;
            let mut samples = wave.samples()?;
            samples.truncate(wave.sample_rate as usize * duration as usize);
            audio::Pcm {
                rate: wave.sample_rate as u32,
                channels: vec![samples],
            }
        }
        7 => {
            let stream = nitro_formats::audio::Strm::parse(
                &sdat.files[sdat.resource_file(7, index as usize)?],
            )?;
            let mut channels = stream.pcm()?;
            for c in &mut channels {
                c.truncate(stream.sample_rate as usize * duration as usize);
            }
            audio::Pcm {
                rate: stream.sample_rate as u32,
                channels,
            }
        }
        _ => {
            return Err(Error::Invalid(
                "choose a sequence, wave archive or stream".into(),
            ))
        }
    };
    audio::wav(&pcm)
}
pub struct EffectOptions {
    pub archive: u32,
    pub sample: u32,
    pub rate: u16,
    pub encoding: u8,
}
pub fn replace_effect(
    p: &mut Project,
    s: &RomSession,
    path: &str,
    source: &Path,
    options: EffectOptions,
) -> Result<()> {
    let EffectOptions {
        archive,
        sample,
        rate,
        encoding,
    } = options;
    let original = p.bytes(s, path)?;
    let mut sdat = Sdat::parse(&preview::plain(original)?)?;
    let id = sdat.resource_file(3, archive as usize)?;
    let mut swar = nitro_formats::audio::Swar::parse(&sdat.files[id])?;
    let pcm = audio::read_wav(&std::fs::read(source)?)?;
    let song = audio::sampled(&pcm, rate, false, encoding, None, None)?;
    let target = swar
        .waves
        .get_mut(sample as usize)
        .ok_or_else(|| Error::Invalid("sample index outside archive".into()))?;
    let capacity = match encoding {
        0 => target.data.len(),
        1 => target.data.len() / 2,
        2 => target.data.len().saturating_sub(4) * 2 + 1,
        _ => return Err(Error::Invalid("choose PCM8, PCM16 or ADPCM".into())),
    };
    let mut samples = song.waves.waves[0].samples()?;
    samples.truncate(capacity);
    if samples.is_empty() {
        return Err(Error::Invalid("effect allocation is empty".into()));
    }
    let loop_start = target
        .looped
        .then(|| target.loop_sample().min(samples.len() - 1));
    let replacement = nitro_formats::audio::Swav::from_pcm(&samples, rate, encoding, loop_start)?;
    if replacement.data.len() > target.data.len() {
        return Err(Error::Invalid(
            "effect exceeds its sample allocation; reduce encoding quality".into(),
        ));
    }
    *target = replacement;
    sdat.files[id] = swar.write()?;
    p.edit(
        path,
        wrap(original, sdat.write()?),
        "Replace sound effect",
        s,
    )
}
pub fn sequence_bytes(bytes: &[u8]) -> Result<Sseq> {
    Ok(Sseq::parse(bytes)?)
}
