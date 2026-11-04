use crate::{session::RomSession, Error, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_platforms")]
    pub platforms: Vec<String>,
}
fn default_platforms() -> Vec<String> {
    vec!["pc".into()]
}
impl Default for Manifest {
    fn default() -> Self {
        Self {
            id: "author.my-mod".into(),
            name: "Untitled mod".into(),
            version: "0.1.0".into(),
            author: "Author".into(),
            kind: "texture".into(),
            description: String::new(),
            platforms: default_platforms(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ProjectConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<crate::base::BaseConfig>,
    pub manifest: Manifest,
    pub rom_sha256: String,
    pub game: String,
    pub rom_path: String,
}
pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub edits: BTreeMap<String, Vec<u8>>,
    pub history: History,
    baselines: BTreeMap<String, Vec<u8>>,
}
#[derive(Clone, Debug)]
pub enum Command {
    Batch {
        title: String,
        commands: Vec<Command>,
    },
    File {
        title: String,
        path: String,
        before: Option<Vec<u8>>,
        after: Option<Vec<u8>>,
    },
    Manifest {
        before: Box<Manifest>,
        after: Box<Manifest>,
    },
}
impl Command {
    fn apply(&self, project: &mut Project, forward: bool) -> Result<()> {
        match self {
            Self::Batch { commands, .. } => {
                let ordered: Vec<_> = if forward {
                    commands.iter().collect()
                } else {
                    commands.iter().rev().collect()
                };
                for (i, c) in ordered.iter().enumerate() {
                    if let Err(e) = c.apply(project, forward) {
                        for previous in ordered[..i].iter().rev() {
                            let _ = previous.apply(project, !forward);
                        }
                        return Err(e);
                    }
                }
                Ok(())
            }
            Self::File {
                path,
                before,
                after,
                ..
            } => project.store(path, if forward { after } else { before }),
            Self::Manifest { before, after } => {
                let previous = project.config.manifest.clone();
                project.config.manifest = if forward { after } else { before }.as_ref().clone();
                if let Err(e) = project.save() {
                    project.config.manifest = previous;
                    return Err(e);
                }
                Ok(())
            }
        }
    }
    fn title(&self) -> String {
        match self {
            Self::Batch { title, .. } => title.clone(),
            Self::File { title, .. } => title.clone(),
            Self::Manifest { .. } => "Edit project information".into(),
        }
    }
}
#[derive(Default)]
pub struct History {
    commands: Vec<Command>,
    cursor: usize,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct HistoryItem {
    pub title: String,
    pub applied: bool,
}
impl Project {
    /// An ephemeral project for headless checks; this constructor never touches the filesystem.
    pub fn in_memory(
        session: &RomSession,
        manifest: Manifest,
        edits: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self> {
        validate_manifest(&manifest)?;
        let mut baselines = BTreeMap::new();
        for path in edits.keys() {
            check_path(path)?;
            if let Ok(b) = session.file(path) {
                baselines.insert(path.clone(), b.to_vec());
            }
        }
        Ok(Self {
            root: PathBuf::new(),
            config: ProjectConfig {
                base: None,
                manifest,
                rom_sha256: session.sha256.clone(),
                game: session.rom.game_code.clone(),
                rom_path: session.path.to_string_lossy().into(),
            },
            edits,
            history: History::default(),
            baselines,
        })
    }
    pub fn create(root: &Path, session: &RomSession, manifest: Manifest) -> Result<Self> {
        validate_manifest(&manifest)?;
        if root.join("project.toml").exists() {
            return Err(Error::Invalid(
                "project already exists; open it instead".into(),
            ));
        }
        let root = if root.exists() {
            root.canonicalize()?
        } else {
            fs::create_dir_all(root)?;
            root.canonicalize()?
        };
        if session
            .path
            .canonicalize()
            .unwrap_or_else(|_| session.path.clone())
            .starts_with(&root)
        {
            return Err(Error::Invalid(
                "keep the ROM outside the project folder".into(),
            ));
        }
        fs::create_dir_all(root.join("edits"))?;
        let ignore = root.join(".gitignore");
        let existing = if ignore.exists() {
            fs::read_to_string(&ignore)?
        } else {
            String::new()
        };
        if !existing.lines().any(|l| l == "build/") {
            atomic_write(
                &ignore,
                format!(
                    "{existing}{}build/\n",
                    if existing.is_empty() || existing.ends_with('\n') {
                        ""
                    } else {
                        "\n"
                    }
                )
                .as_bytes(),
            )?;
        }
        let config = ProjectConfig {
            base: None,
            manifest,
            rom_sha256: session.sha256.clone(),
            game: session.rom.game_code.clone(),
            rom_path: session.path.to_string_lossy().into(),
        };
        let p = Self {
            root,
            config,
            edits: BTreeMap::new(),
            history: History::default(),
            baselines: BTreeMap::new(),
        };
        p.save()?;
        Ok(p)
    }
    pub fn open(root: &Path, session: &RomSession) -> Result<Self> {
        let root = root.canonicalize()?;
        ensure_no_symlinks(&root, &root.join("edits"))?;
        let config: ProjectConfig =
            toml::from_str(&fs::read_to_string(root.join("project.toml"))?)?;
        validate_manifest(&config.manifest)?;
        if config.rom_sha256 != session.sha256 || config.game != session.rom.game_code {
            return Err(Error::Invalid(
                "project targets a different ROM SHA-256 or game code".into(),
            ));
        }
        let mut edits = BTreeMap::new();
        fn read(base: &Path, dir: &Path, edits: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
            if !dir.exists() {
                return Ok(());
            }
            for e in fs::read_dir(dir)? {
                let e = e?;
                let ty = e.file_type()?;
                if ty.is_symlink() {
                    return Err(Error::Invalid("symlinks are not allowed in edits".into()));
                }
                if ty.is_dir() {
                    read(base, &e.path(), edits)?;
                } else if ty.is_file() {
                    let path = e
                        .path()
                        .strip_prefix(base)
                        .map_err(|e| Error::Invalid(e.to_string()))?
                        .to_string_lossy()
                        .replace('\\', "/");
                    check_path(&path)?;
                    edits.insert(path, fs::read(e.path())?);
                }
            }
            Ok(())
        }
        read(&root.join("edits"), &root.join("edits"), &mut edits)?;
        let mut baselines = BTreeMap::new();
        for (path, bytes) in &mut edits {
            if let Ok(original) = session.file(path) {
                // Existing ROM files use compact editable source JSON; new files remain author bytes.
                *bytes = crate::source::decode(original, bytes)?;
                baselines.insert(path.clone(), original.to_vec());
            }
        }
        Ok(Self {
            root,
            config,
            edits,
            history: History::default(),
            baselines,
        })
    }
    pub fn save(&self) -> Result<()> {
        if self.root.as_os_str().is_empty() {
            return Err(Error::Invalid(
                "in-memory projects cannot save to disk".into(),
            ));
        }
        atomic_write(
            &self.root.join("project.toml"),
            toml::to_string_pretty(&self.config)?.as_bytes(),
        )
    }
    fn store(&mut self, path: &str, bytes: &Option<Vec<u8>>) -> Result<()> {
        if self.root.as_os_str().is_empty() {
            return Err(Error::Invalid(
                "in-memory projects cannot save to disk".into(),
            ));
        }
        check_path(path)?;
        let dest = self.root.join("edits").join(path);
        ensure_no_symlinks(&self.root, &dest)?;
        match bytes {
            Some(b) => {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                let source = match self.baselines.get(path) {
                    Some(original) => crate::source::encode(original, b)?,
                    None => b.clone(),
                };
                atomic_write(&dest, &source)?;
                self.edits.insert(path.into(), b.clone());
            }
            None => {
                if dest.exists() {
                    fs::remove_file(dest)?;
                }
                self.edits.remove(path);
            }
        }
        Ok(())
    }
    pub fn execute(&mut self, cmd: Command) -> Result<()> {
        cmd.apply(self, true)?;
        self.history.commands.truncate(self.history.cursor);
        self.history.commands.push(cmd);
        self.history.cursor += 1;
        Ok(())
    }
    pub fn edit(
        &mut self,
        path: &str,
        bytes: Vec<u8>,
        title: &str,
        session: &RomSession,
    ) -> Result<()> {
        check_path(path)?;
        check_edit_conflicts(self.edits.keys().map(String::as_str), path)?;
        if let Ok(original) = session.file(path) {
            self.baselines
                .entry(path.into())
                .or_insert_with(|| original.to_vec());
        }
        let after = if session.file(path).is_ok_and(|b| b == bytes) {
            None
        } else {
            Some(bytes)
        };
        let before = self.edits.get(path).cloned();
        if before == after {
            return Ok(());
        }
        self.execute(Command::File {
            title: title.into(),
            path: path.into(),
            before,
            after,
        })
    }
    pub fn edit_many(
        &mut self,
        edits: BTreeMap<String, Vec<u8>>,
        title: &str,
        session: &RomSession,
    ) -> Result<()> {
        for path in edits.keys() {
            check_path(path)?;
            check_edit_conflicts(edits.keys().map(String::as_str), path)?;
        }
        let mut commands = vec![];
        for (path, bytes) in edits {
            check_path(&path)?;
            check_edit_conflicts(self.edits.keys().map(String::as_str), &path)?;
            if let Ok(b) = session.file(&path) {
                self.baselines
                    .entry(path.clone())
                    .or_insert_with(|| b.to_vec());
            }
            let after = if session.file(&path).is_ok_and(|b| b == bytes) {
                None
            } else {
                Some(bytes)
            };
            let before = self.edits.get(&path).cloned();
            if before != after {
                commands.push(Command::File {
                    title: title.into(),
                    path,
                    before,
                    after,
                });
            }
        }
        if commands.is_empty() {
            return Ok(());
        }
        self.execute(Command::Batch {
            title: title.into(),
            commands,
        })
    }
    pub fn revert(&mut self, path: &str) -> Result<()> {
        self.execute(Command::File {
            title: format!("Revert {path}"),
            path: path.into(),
            before: self.edits.get(path).cloned(),
            after: None,
        })
    }
    pub fn update_manifest(&mut self, after: Manifest) -> Result<()> {
        validate_manifest(&after)?;
        self.execute(Command::Manifest {
            before: Box::new(self.config.manifest.clone()),
            after: Box::new(after),
        })
    }
    pub fn undo(&mut self) -> Result<()> {
        if self.history.cursor > 0 {
            let c = self.history.commands[self.history.cursor - 1].clone();
            c.apply(self, false)?;
            self.history.cursor -= 1;
        }
        Ok(())
    }
    pub fn redo(&mut self) -> Result<()> {
        if self.history.cursor < self.history.commands.len() {
            let c = self.history.commands[self.history.cursor].clone();
            c.apply(self, true)?;
            self.history.cursor += 1;
        }
        Ok(())
    }
    pub fn history(&self) -> Vec<HistoryItem> {
        self.history
            .commands
            .iter()
            .enumerate()
            .map(|(i, c)| HistoryItem {
                title: c.title(),
                applied: i < self.history.cursor,
            })
            .collect()
    }
    pub fn bytes<'a>(&'a self, session: &'a RomSession, path: &str) -> Result<&'a [u8]> {
        match self.edits.get(path) {
            Some(b) => Ok(b),
            None => session.file(path),
        }
    }
}
/// A whole-container replacement and a member edit cannot both describe the same output.
pub(crate) fn check_edit_conflicts<'a>(
    paths: impl Iterator<Item = &'a str>,
    path: &str,
) -> Result<()> {
    for other in paths {
        if path.starts_with(&format!("{other}/")) || other.starts_with(&format!("{path}/")) {
            return Err(Error::Invalid(format!(
                "conflicting edits: {path} and {other}; revert the container or member edit first"
            )));
        }
    }
    Ok(())
}
pub fn check_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.len() > 200
        || path
            .chars()
            .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
        || path.split('/').any(|s| {
            s.is_empty() || s == "." || s == ".." || s.ends_with([' ', '.']) || {
                let stem = s.split('.').next().unwrap_or("").to_ascii_lowercase();
                [
                    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6",
                    "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7",
                    "lpt8", "lpt9",
                ]
                .contains(&stem.as_str())
            }
        })
    {
        return Err(Error::Invalid(format!("unsafe or overlong path: {path}")));
    }
    Ok(())
}
pub fn validate_manifest(m: &Manifest) -> Result<()> {
    check_path(&m.id)?;
    if !(3..=64).contains(&m.id.len())
        || !m
            .id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c))
        || [".cache", "enabled.txt"].contains(&m.id.as_str())
        || m.id.contains('/')
    {
        return Err(Error::Invalid(
            "id must be 3–64 lowercase letters, digits, dots, underscores or hyphens".into(),
        ));
    }
    if m.name.trim().is_empty()
        || m.author.trim().is_empty()
        || m.name.chars().count() > 200
        || m.author.chars().count() > 200
        || m.description.chars().count() > 2000
        || !matches!(
            m.kind.as_str(),
            "character" | "track" | "kart" | "battle" | "music" | "texture" | "other"
        )
    {
        return Err(Error::Invalid(
            "invalid manifest name, author, type or description".into(),
        ));
    }
    if semver::Version::parse(&m.version).is_err() {
        return Err(Error::Invalid("invalid version".into()));
    }
    if m.platforms != ["pc"] && m.platforms != ["pc", "ds"] {
        return Err(Error::Invalid("platforms must be [pc] or [pc, ds]".into()));
    }
    Ok(())
}
pub fn ensure_no_symlinks(root: &Path, dest: &Path) -> Result<()> {
    let relative = dest
        .strip_prefix(root)
        .map_err(|_| Error::Invalid("path outside project".into()))?;
    let mut p = root.to_path_buf();
    for c in relative.components() {
        p.push(c);
        if fs::symlink_metadata(&p).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid(
                "symlinks are not allowed in project paths".into(),
            ));
        }
    }
    Ok(())
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_file_name(format!(
        ".{}.writing",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    if temp.exists() {
        return Err(Error::Invalid(format!(
            "unfinished write exists: {}",
            temp.display()
        )));
    }
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    use std::io::Write;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    // rename replaces an existing regular file on supported desktop platforms.
    if let Err(e) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(e.into());
    }
    Ok(())
}
