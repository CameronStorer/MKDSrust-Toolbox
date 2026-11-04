use crate::{
    pack::{self, PackResult},
    preview::{self, Preview, TextToken},
    project::{HistoryItem, Manifest, Project},
    session::{FileNode, RomSession},
    settings::Settings,
    validation::{self, Issue},
    Error, Result,
};
use nitro_formats::{
    bmg::{Bmg, Token},
    graphics::Nclr,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum Request {
    Snapshot,
    OpenRom {
        path: String,
    },
    CreateProject {
        path: String,
        manifest: Manifest,
    },
    CreateFromBase {
        path: String,
        manifest: Manifest,
        source: String,
    },
    ExportDs {
        rom: Option<String>,
        bps: Option<String>,
    },
    ShowInFolder {
        path: String,
    },
    Paint {
        path: String,
        palette: Option<String>,
        texture: u32,
        bank: u32,
        points: Vec<(u32, u32, u16)>,
    },
    ImportGraphic {
        path: String,
        source: String,
        palette: Option<String>,
        texture: u32,
        bank: u32,
        format: u8,
        dither: bool,
    },
    ApplyGraphic,
    CancelGraphic,
    OpenProject {
        path: String,
    },
    Select {
        path: String,
    },
    Palette {
        path: String,
        indices: Vec<u32>,
        colors: Vec<u16>,
    },
    Text {
        path: String,
        index: u32,
        tokens: Vec<TextToken>,
    },
    Revert {
        path: String,
    },
    UpdateManifest {
        manifest: Manifest,
    },
    Undo,
    Redo,
    Pack,
    Play {
        game: String,
    },
    Stop,
    Reload {
        game: String,
    },
    Courses {
        game: String,
    },
    Validate,
    Settings {
        settings: Settings,
    },
    Extract {
        path: String,
        output: String,
    },
    Export {
        path: String,
        output: String,
        to: String,
        palette: Option<String>,
        screen: Option<String>,
        texture: u32,
        bank: u32,
    },
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct Snapshot {
    pub graphic_import: Option<crate::graphic::ImportInfo>,
    pub base: Option<crate::base::BaseConfig>,
    pub base_changes: Vec<crate::base::BaseChange>,
    pub ds_export: Option<crate::export::ExportResult>,
    pub compatibility: String,
    pub bench: crate::bench::BenchSnapshot,
    pub rom_path: String,
    pub game: String,
    pub rom_hash: String,
    pub project_path: String,
    pub manifest: Option<Manifest>,
    pub files: Vec<FileNode>,
    pub edited: Vec<String>,
    pub history: Vec<HistoryItem>,
    pub selected: String,
    pub preview: Preview,
    pub settings: Settings,
    pub warnings: Vec<String>,
    pub issues: Vec<Issue>,
    pub pack: Option<PackResult>,
    pub can_undo: bool,
    pub can_redo: bool,
}
#[derive(Clone, Debug, Deserialize, Type)]
pub struct BinaryRequest {
    pub path: String,
    pub mode: String,
    pub start: u32,
    pub length: u32,
    pub palette: Option<String>,
    pub screen: Option<String>,
    pub texture: u32,
    pub bank: u32,
}
pub struct Service {
    pub graphic_import: Option<crate::graphic::ImportDraft>,
    pub ds_export: Option<crate::export::ExportResult>,
    pub bench: crate::bench::Bench,
    pub session: Option<RomSession>,
    pub project: Option<Project>,
    pub settings: Settings,
    pub selected: String,
    pub issues: Vec<Issue>,
    pub packed: Option<PackResult>,
    pub persist_settings: bool,
}
impl Default for Service {
    fn default() -> Self {
        Self {
            graphic_import: None,
            ds_export: None,
            bench: crate::bench::Bench::default(),
            session: None,
            project: None,
            settings: crate::settings::load(),
            selected: String::new(),
            issues: vec![],
            packed: None,
            persist_settings: true,
        }
    }
}
impl Service {
    fn session(&self) -> Result<&RomSession> {
        self.session
            .as_ref()
            .ok_or_else(|| Error::Invalid("open your ROM first".into()))
    }
    fn editing(&mut self) -> Result<(&mut Project, &RomSession)> {
        Ok((
            self.project
                .as_mut()
                .ok_or_else(|| Error::Invalid("create or open a project first".into()))?,
            self.session
                .as_ref()
                .ok_or_else(|| Error::Invalid("open a ROM first".into()))?,
        ))
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let session = self.session.as_ref();
        let project = self.project.as_ref();
        let history = project.map(Project::history).unwrap_or_default();
        let bytes = if self.selected.is_empty() {
            None
        } else {
            session.and_then(|s| match project {
                Some(p) => p.bytes(s, &self.selected).ok(),
                None => s.file(&self.selected).ok(),
            })
        };
        let mut warnings = session.map(|s| s.warnings.clone()).unwrap_or_default();
        let preview = match bytes.map(preview::preview).transpose() {
            Ok(p) => p.unwrap_or_default(),
            Err(e) => {
                warnings.push(e.to_string());
                Preview::default()
            }
        };
        let mut files = session.map(|s| s.nodes.clone()).unwrap_or_default();
        if let Some(p) = project {
            for (path, b) in &p.edits {
                if let Some(file) = files.iter_mut().find(|f| &f.path == path) {
                    file.size = b.len() as u32;
                } else {
                    let f = nitro_formats::detect::detect(b, path);
                    files.push(FileNode {
                        path: path.clone(),
                        name: path.rsplit('/').next().unwrap_or(path).into(),
                        size: b.len() as u32,
                        format: f.as_ref().map_or("bin", |f| f.extension).into(),
                        category: f.as_ref().map_or("Binary", |f| f.category).into(),
                        offset: 0,
                        compressed: false,
                        archive: false,
                        parent: None,
                    });
                }
            }
        }
        Ok(Snapshot {
            graphic_import: self.graphic_import.as_ref().map(|d| d.info.clone()),
            base: project.and_then(|p| p.config.base.clone()),
            base_changes: session
                .map(crate::base::changes)
                .transpose()?
                .unwrap_or_default(),
            ds_export: self.ds_export.clone(),
            compatibility: if project.is_none() {
                "Unknown"
            } else if self.issues.is_empty() {
                "DS OK"
            } else {
                "PC only"
            }
            .into(),
            bench: self.bench.snapshot(Path::new(&self.settings.install_path)),
            rom_path: session
                .map(|s| s.path.to_string_lossy().into())
                .unwrap_or_default(),
            game: session.map(|s| s.rom.game_code.clone()).unwrap_or_default(),
            rom_hash: session.map(|s| s.sha256.clone()).unwrap_or_default(),
            project_path: project
                .map(|p| p.root.to_string_lossy().into())
                .unwrap_or_default(),
            manifest: project.map(|p| p.config.manifest.clone()),
            files,
            edited: project
                .map(|p| p.edits.keys().cloned().collect())
                .unwrap_or_default(),
            can_undo: history.iter().any(|h| h.applied),
            can_redo: history.iter().any(|h| !h.applied),
            history,
            selected: self.selected.clone(),
            preview,
            settings: self.settings.clone(),
            warnings,
            issues: self.issues.clone(),
            pack: self.packed.clone(),
        })
    }
    pub fn dispatch(&mut self, request: Request) -> Result<Snapshot> {
        self.bench.poll()?;
        let changes_project = matches!(
            &request,
            Request::CreateProject { .. }
                | Request::CreateFromBase { .. }
                | Request::OpenProject { .. }
                | Request::Palette { .. }
                | Request::Text { .. }
                | Request::Revert { .. }
                | Request::UpdateManifest { .. }
                | Request::Undo
                | Request::Redo
                | Request::Paint { .. }
                | Request::ApplyGraphic
        );
        match request {
            Request::Paint {
                path,
                palette,
                texture,
                bank,
                points,
            } => {
                let (p, s) = self.editing()?;
                crate::graphic::paint(
                    p,
                    s,
                    &path,
                    palette.as_deref(),
                    texture as usize,
                    bank as usize,
                    &points,
                )?;
                self.packed = None;
            }
            Request::ImportGraphic {
                path,
                source,
                palette,
                texture,
                bank,
                format,
                dither,
            } => {
                let (p, s) = self.editing()?;
                self.graphic_import = Some(crate::graphic::import(
                    p,
                    s,
                    &path,
                    Path::new(&source),
                    palette.as_deref(),
                    texture as usize,
                    bank as usize,
                    format,
                    dither,
                )?);
            }
            Request::ApplyGraphic => {
                let draft = self
                    .graphic_import
                    .take()
                    .ok_or_else(|| Error::Invalid("preview a PNG import first".into()))?;
                let (p, s) = self.editing()?;
                p.edit_many(draft.edits, "Import PNG", s)?;
                self.packed = None;
            }
            Request::CancelGraphic => self.graphic_import = None,
            Request::ShowInFolder { path } => {
                crate::bench::show_in_folder(Path::new(&path))?;
            }
            Request::ExportDs { rom, bps } => {
                for path in [rom.as_ref(), bps.as_ref()].into_iter().flatten() {
                    self.check_output(Path::new(path))?;
                }
                let (p, s) = self.editing()?;
                self.ds_export = Some(crate::export::export_ds(
                    p,
                    s,
                    rom.as_deref().map(Path::new),
                    bps.as_deref().map(Path::new),
                )?);
            }
            Request::CreateFromBase {
                path,
                manifest,
                source,
            } => {
                let clean = RomSession::from_bytes(
                    self.session()?.clean_bytes().to_vec(),
                    self.session()?.path.clone(),
                )?;
                let (p, s) =
                    crate::base::create(Path::new(&path), &clean, manifest, Path::new(&source))?;
                self.project = Some(p);
                self.session = Some(s);
                self.selected.clear();
                self.packed = None;
                self.ds_export = None;
            }
            Request::Snapshot => {}
            Request::Stop => self.bench.stop()?,
            Request::Courses { game } => {
                self.bench.list_courses(
                    Path::new(&self.settings.install_path),
                    &game,
                    self.session
                        .as_ref()
                        .ok_or_else(|| Error::Invalid("open a ROM first".into()))?,
                )?;
            }
            Request::Play { game } | Request::Reload { game } => {
                self.bench.stop()?;
                let project = self
                    .project
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("create or open a project first".into()))?;
                let session = self
                    .session
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("open your ROM first".into()))?;
                if self.settings.play_target == "emulator" {
                    self.bench.play_emulator(
                        Path::new(&self.settings.emulator_path),
                        project,
                        session,
                    )?;
                } else if self.settings.play_target == "flashcart" {
                    if self.settings.flashcart_path.trim().is_empty()
                        || !Path::new(&self.settings.flashcart_path).is_dir()
                    {
                        return Err(Error::Invalid(
                            "choose a flashcart export folder in Settings".into(),
                        ));
                    }
                    let out = Path::new(&self.settings.flashcart_path)
                        .join(format!("{}.nds", project.config.manifest.id));
                    self.check_output(&out)?;
                    self.ds_export = Some(crate::export::export_ds(
                        project,
                        session,
                        Some(&out),
                        None,
                    )?);
                } else {
                    self.bench.play_race(
                        Path::new(&self.settings.install_path),
                        &game,
                        project,
                        session,
                        &self.settings.race,
                    )?;
                }
            }
            Request::OpenRom { path } => {
                let session = RomSession::open(Path::new(&path))?;
                self.session = Some(session);
                self.project = None;
                self.selected.clear();
                self.issues.clear();
                self.packed = None;
                self.settings.rom_path = path;
                self.ds_export = None;
                if self.persist_settings {
                    crate::settings::save(&self.settings)?;
                }
            }
            Request::CreateProject { path, manifest } => {
                self.session = Some(RomSession::from_bytes(
                    self.session()?.clean_bytes().to_vec(),
                    self.session()?.path.clone(),
                )?);
                self.project = Some(Project::create(
                    Path::new(&path),
                    self.session()?,
                    manifest,
                )?);
                self.packed = None;
                self.selected.clear();
            }
            Request::OpenProject { path } => {
                // The persisted ROM path is a hint; the hash is always checked by Project::open.
                if self.session.is_none() {
                    let config: crate::project::ProjectConfig = toml::from_str(
                        &std::fs::read_to_string(Path::new(&path).join("project.toml"))?,
                    )?;
                    self.session = Some(RomSession::open(Path::new(&config.rom_path))?);
                }
                self.session = Some(crate::base::working(Path::new(&path), self.session()?)?);
                self.project = Some(Project::open(Path::new(&path), self.session()?)?);
                self.packed = None;
                self.selected.clear();
            }
            Request::Select { path } => {
                if let Some(p) = &self.project {
                    p.bytes(self.session()?, &path)?;
                } else {
                    self.session()?.file(&path)?;
                }
                self.selected = path;
            }
            Request::Palette {
                path,
                indices,
                colors,
            } => {
                if indices.len() != colors.len() || indices.is_empty() {
                    return Err(Error::Invalid(
                        "palette edit requires matching nonempty indices and colours".into(),
                    ));
                }
                let (p, s) = self.editing()?;
                let original = p.bytes(s, &path)?;
                let plain = preview::plain(original)?;
                let mut palette = Nclr::parse(&plain)?;
                for (&i, &c) in indices.iter().zip(&colors) {
                    let color = palette
                        .colors
                        .get_mut(i as usize)
                        .ok_or_else(|| Error::Invalid("palette index out of range".into()))?;
                    *color = c & 0x7fff;
                }
                let b = palette.write();
                let b = if original.first() == Some(&0x10) {
                    nitro_formats::recompress(original, &b)
                } else {
                    b
                };
                p.edit(&path, b, "Edit palette colours", s)?;
                self.packed = None;
            }
            Request::Text {
                path,
                index,
                tokens,
            } => {
                let (p, s) = self.editing()?;
                let original = p.bytes(s, &path)?;
                let plain = preview::plain(original)?;
                let mut bmg = Bmg::parse(&plain)?;
                let message = bmg
                    .messages
                    .get_mut(index as usize)
                    .ok_or_else(|| Error::Invalid("message index out of range".into()))?;
                let old_codes = message
                    .tokens
                    .iter()
                    .filter_map(|t| {
                        if let Token::Code(c) = t {
                            Some(c.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let new_codes = tokens
                    .iter()
                    .filter_map(|t| {
                        if let TextToken::Code(c) = t {
                            Some(c.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                for c in &new_codes {
                    if c.len() < 4
                        || c.len() > 254
                        || c.len() % 2 != 0
                        || c[..2] != [0x1a, 0]
                        || c[2] as usize != c.len()
                    {
                        return Err(Error::Invalid("invalid BMG code".into()));
                    }
                }
                // Existing escapes are immutable; additional valid codes may be inserted.
                let mut pos = 0;
                for c in &new_codes {
                    if old_codes.get(pos) == Some(c) {
                        pos += 1;
                    }
                }
                if pos < old_codes.len() {
                    return Err(Error::Invalid(
                        "existing escape codes must be preserved in order".into(),
                    ));
                }
                let mut new_tokens = vec![];
                for t in tokens {
                    match t {
                        TextToken::Text(text) => {
                            if text.contains(['\0', '\u{1a}']) {
                                return Err(Error::Invalid(
                                    "use the insert-code menu for escapes".into(),
                                ));
                            }
                            new_tokens.push(Token::Text(text.encode_utf16().collect()));
                        }
                        TextToken::Code(c) => new_tokens.push(Token::Code(c)),
                    }
                }
                message.tokens = new_tokens;
                let b = bmg.write();
                let b = if original.first() == Some(&0x10) {
                    nitro_formats::recompress(original, &b)
                } else {
                    b
                };
                p.edit(&path, b, "Edit text message", s)?;
                self.packed = None;
            }
            Request::Revert { path } => {
                self.editing()?.0.revert(&path)?;
                self.packed = None;
            }
            Request::UpdateManifest { manifest } => {
                self.editing()?.0.update_manifest(manifest)?;
                self.packed = None;
            }
            Request::Undo => {
                self.editing()?.0.undo()?;
                self.packed = None;
            }
            Request::Redo => {
                self.editing()?.0.redo()?;
                self.packed = None;
            }
            Request::Pack => {
                let (p, s) = self.editing()?;
                self.packed = Some(pack::pack(p, s)?);
            }
            Request::Validate => {
                let (p, s) = self.editing()?;
                self.issues = validation::validate(p, s);
            }
            Request::Settings { settings } => {
                settings.race.argument()?;
                if !["emulator", "mkdsrust", "flashcart"].contains(&settings.play_target.as_str()) {
                    return Err(Error::Invalid("invalid play target".into()));
                }
                if !["dark", "light"].contains(&settings.theme.as_str())
                    || !["Blender", "Maya", "MKTB"].contains(&settings.navigation.as_str())
                    || !settings.camera_speed.is_finite()
                    || !(0.1..=10.).contains(&settings.camera_speed)
                {
                    return Err(Error::Invalid(
                        "invalid theme or navigation preference".into(),
                    ));
                }
                if self.persist_settings {
                    crate::settings::save(&settings)?;
                }
                self.settings = settings;
            }
            Request::Extract { path, output } => {
                let s = self.session()?;
                let b = match &self.project {
                    Some(p) => p.bytes(s, &path)?,
                    None => s.file(&path)?,
                };
                self.check_output(Path::new(&output))?;
                crate::project::atomic_write(Path::new(&output), b)?;
            }
            Request::Export {
                path,
                output,
                to,
                palette,
                screen,
                texture,
                bank,
            } => {
                let s = self.session()?;
                let bytes = if to == "png" {
                    preview::png(&preview::image(
                        self.project.as_ref(),
                        s,
                        &path,
                        palette.as_deref(),
                        screen.as_deref(),
                        texture as usize,
                        bank as usize,
                    )?)?
                } else if to == "json" {
                    preview::convert_json(match &self.project {
                        Some(p) => p.bytes(s, &path)?,
                        None => s.file(&path)?,
                    })?
                } else if to == "glb" {
                    crate::scene::CourseScene::load(self.project.as_ref(), s, &path)?.glb(false)?
                } else if to == "obj" {
                    let bytes = match &self.project {
                        Some(p) => p.bytes(s, &path)?,
                        None => s.file(&path)?,
                    };
                    crate::model::obj(&nitro_formats::kcl::Kcl::parse(&preview::plain(bytes)?)?)?
                } else {
                    return Err(Error::Invalid(
                        "export supports png, json, glb or obj".into(),
                    ));
                };
                self.check_output(Path::new(&output))?;
                crate::project::atomic_write(Path::new(&output), &bytes)?;
            }
        }
        if changes_project {
            self.graphic_import = None;
            self.issues = if let (Some(p), Some(s)) = (&self.project, &self.session) {
                validation::validate(p, s)
            } else {
                vec![]
            };
            self.ds_export = None;
        }
        self.snapshot()
    }
    pub fn check_output(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let canonical = if path.exists() {
            path.canonicalize()?
        } else {
            parent.canonicalize()?.join(
                path.file_name()
                    .ok_or_else(|| Error::Invalid("output needs a filename".into()))?,
            )
        };
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.path.canonicalize().is_ok_and(|p| p == canonical))
        {
            return Err(Error::Invalid("cannot overwrite the ROM".into()));
        }
        if self
            .project
            .as_ref()
            .is_some_and(|p| canonical.starts_with(&p.root))
        {
            return Err(Error::Invalid(
                "extract and export outside the project; edits are managed by Toolbox".into(),
            ));
        }
        Ok(())
    }
    pub fn binary(&self, r: BinaryRequest) -> Result<Vec<u8>> {
        if r.mode == "cell-png" {
            return preview::png(&preview::cell_image(
                self.project.as_ref(),
                self.session()?,
                &r.path,
                r.palette.as_deref(),
                r.screen.as_deref(),
                r.texture as usize,
                r.start as usize,
            )?);
        }
        if r.mode == "import-preview" {
            return preview::png(
                &self
                    .graphic_import
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("no import preview".into()))?
                    .image,
            );
        }
        if r.mode == "paint-data" || r.mode == "palette-colors" {
            let s = self.session()?;
            let b = match &self.project {
                Some(p) => p.bytes(s, &r.path)?,
                None => s.file(&r.path)?,
            };
            if r.mode == "paint-data" {
                return crate::graphic::paint_data(b, r.texture as usize, r.bank as usize);
            }
            let b = preview::plain(b)?;
            let colors = if b.starts_with(b"BTX0") {
                nitro_formats::texture::Nsbtx::parse(&b)?
                    .palettes
                    .get(r.bank as usize)
                    .map(|p| p.colors.clone())
                    .unwrap_or_default()
            } else {
                let p = Nclr::parse(&b)?;
                let stride = if p.depth == 3 { 16 } else { 256 };
                p.colors
                    .chunks(stride)
                    .nth(r.bank as usize)
                    .unwrap_or(&[])
                    .to_vec()
            };
            return Ok(colors.iter().flat_map(|c| c.to_le_bytes()).collect());
        }
        let s = self.session()?;
        if r.mode == "glb" || r.mode == "course-glb" {
            return crate::scene::CourseScene::load(self.project.as_ref(), s, &r.path)?
                .glb(r.mode == "course-glb");
        }
        if r.mode == "png" {
            return preview::png(&preview::image(
                self.project.as_ref(),
                s,
                &r.path,
                r.palette.as_deref(),
                r.screen.as_deref(),
                r.texture as usize,
                r.bank as usize,
            )?);
        }
        if r.mode != "hex" {
            return Err(Error::Invalid("unknown binary request mode".into()));
        }
        let b = match &self.project {
            Some(p) => p.bytes(s, &r.path)?,
            None => s.file(&r.path)?,
        };
        let start = (r.start as usize).min(b.len());
        let len = (r.length as usize).min(65536).min(b.len() - start);
        Ok(b[start..start + len].to_vec())
    }
    pub fn project_session(project: &Path) -> Result<(Project, RomSession)> {
        let config: crate::project::ProjectConfig =
            toml::from_str(&std::fs::read_to_string(project.join("project.toml"))?)?;
        let rom = std::env::var_os("MKDS_ROM")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(config.rom_path));
        let session = crate::base::working(project, &RomSession::open(&rom)?)?;
        let project = Project::open(project, &session)?;
        Ok((project, session))
    }
}
