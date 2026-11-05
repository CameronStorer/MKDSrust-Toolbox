//! Portable install handoff and owned game process. Packages contain deltas only.
use crate::{
    pack,
    project::{atomic_write, check_path, ensure_no_symlinks, Project},
    session::RomSession,
    settings, Error, Result,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::VecDeque,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};
#[derive(Clone, Debug, Serialize, Type)]
pub struct Course {
    pub id: u32,
    pub name: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, Type)]
#[serde(default)]
pub struct RaceSetup {
    pub course: Option<u32>,
    pub character: Option<u8>,
    pub kart: Option<u8>,
    pub racers: Option<u8>,
}
impl RaceSetup {
    pub fn argument(&self) -> Result<Option<String>> {
        if self.racers.is_some_and(|n| !(1..=8).contains(&n)) {
            return Err(Error::Invalid("racers must be 1–8".into()));
        }
        let mut fields = vec![];
        if let Some(n) = self.course {
            fields.push(format!("course={n}"));
        }
        if let Some(n) = self.character {
            fields.push(format!("character={n}"));
        }
        if let Some(n) = self.kart {
            fields.push(format!("kart={n}"));
        }
        if let Some(n) = self.racers {
            fields.push(format!("racers={n}"));
        }
        Ok((!fields.is_empty()).then(|| fields.join(",")))
    }
}
pub fn parse_courses(output: &str) -> Vec<Course> {
    output
        .lines()
        .filter_map(|s| {
            let (id, name) = s.trim().split_once('\t')?;
            Some(Course {
                id: id.parse().ok()?,
                name: name.into(),
            })
        })
        .collect()
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct GameInstall {
    pub id: String,
    pub name: String,
    pub path: String,
}
#[derive(Clone, Debug, Default, Serialize, Type)]
pub struct RunSummary {
    pub package_size: u32,
    pub install_path: String,
    pub executable: String,
    pub exit_code: Option<i32>,
    pub stopped: bool,
}
#[derive(Clone, Debug, Default, Serialize, Type)]
pub struct BenchSnapshot {
    pub courses: Vec<Course>,
    pub games: Vec<GameInstall>,
    pub running: bool,
    pub logs: Vec<String>,
    pub last_run: Option<RunSummary>,
}
#[derive(Default)]
pub struct Bench {
    courses: Vec<Course>,
    course_key: String,
    child: Option<Child>,
    logs: Arc<Mutex<VecDeque<String>>>,
    last: Option<RunSummary>,
    workers: Vec<std::thread::JoinHandle<()>>,
}
pub fn discover(root: &Path) -> Vec<GameInstall> {
    if !settings::is_install(root) {
        return vec![];
    }
    let Ok(read) = std::fs::read_dir(root.join("games")) else {
        return vec![];
    };
    let mut games = vec![];
    for entry in read.flatten() {
        let path = entry.path();
        let id = entry.file_name().to_string_lossy().into_owned();
        if check_path(&id).is_err() || ensure_no_symlinks(root, &path).is_err() {
            continue;
        }
        let Ok(bytes) = std::fs::read(path.join("manifest.json")) else {
            continue;
        };
        let Ok(m) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        if m["executable"].as_str().is_some() {
            games.push(GameInstall {
                id,
                name: m["rom"]["title"].as_str().unwrap_or("Native game").into(),
                path: path.to_string_lossy().into(),
            });
        }
    }
    games.sort_by(|a, b| a.id.cmp(&b.id));
    games
}
fn install(root: &Path, game: &Path, id: &str, bytes: &[u8]) -> Result<PathBuf> {
    let mods = game.join("mods");
    ensure_no_symlinks(root, &mods)?;
    std::fs::create_dir_all(&mods)?;
    let dest = mods.join(id);
    let enabled = mods.join("enabled.txt");
    ensure_no_symlinks(root, &dest)?;
    ensure_no_symlinks(root, &enabled)?;
    if dest.exists() {
        let m: pack::PackageManifest =
            serde_json::from_slice(&std::fs::read(dest.join("manifest.json"))?)?;
        if m.info.id != id {
            return Err(Error::Invalid(
                "existing mod folder belongs to another package".into(),
            ));
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Invalid(e.to_string()))?
        .as_nanos();
    let stage = mods.join(format!(".toolbox-{id}-{stamp}"));
    let backup = mods.join(format!(".toolbox-{id}-{stamp}-old"));
    std::fs::create_dir(&stage)?;
    let result = (|| -> Result<()> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
        if zip.len() > 5000 {
            return Err(Error::Invalid("package file count exceeds contract".into()));
        }
        let mut total = 0u64;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i)?;
            let name = f.name()?.into_owned();
            check_path(&name)?;
            total = total.saturating_add(f.size());
            if total > 400 * 1024 * 1024
                || f.is_dir()
                || f.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err(Error::Invalid("invalid package member".into()));
            }
            let out = stage.join(&name);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?
            }
            let mut dest = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(out)?;
            std::io::copy(&mut f, &mut dest)?;
        }
        let mut order = match std::fs::read_to_string(&enabled) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e.into()),
        };
        let lines = order
            .lines()
            .filter(|s| s.trim() != id)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        order = lines.join("\n");
        if !order.is_empty() {
            order.push('\n')
        }
        order.push_str(id);
        order.push('\n');
        let existed = dest.exists();
        if existed {
            std::fs::rename(&dest, &backup)?
        }
        if let Err(e) = std::fs::rename(&stage, &dest) {
            if existed {
                let _ = std::fs::rename(&backup, &dest);
            }
            return Err(e.into());
        }
        if let Err(e) = atomic_write(&enabled, order.as_bytes()) {
            let _ = std::fs::rename(&dest, &stage);
            if existed {
                let _ = std::fs::rename(&backup, &dest);
            }
            return Err(e);
        }
        if existed {
            std::fs::remove_dir_all(&backup)?
        }
        Ok(())
    })();
    if stage.exists() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result?;
    Ok(dest)
}
impl Bench {
    pub fn play_emulator(
        &mut self,
        executable: &Path,
        project: &Project,
        session: &RomSession,
    ) -> Result<()> {
        self.stop()?;
        if !executable.is_file() {
            return Err(Error::Invalid(
                "choose an emulator executable in Settings".into(),
            ));
        }
        let dir = directories::ProjectDirs::from("com", "mkdsrust", "Toolbox")
            .ok_or_else(|| Error::Invalid("user cache unavailable".into()))?
            .cache_dir()
            .join("play");
        std::fs::create_dir_all(&dir)?;
        let out = dir.join(format!("{}.nds", project.config.manifest.id));
        crate::export::export_ds(project, session, Some(&out), None)?;
        let executable = executable.canonicalize()?;
        let mut child = Command::new(&executable)
            .arg(&out)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        self.logs = Arc::default();
        if let Some(out) = child.stdout.take() {
            self.drain(out, "stdout");
        }
        if let Some(err) = child.stderr.take() {
            self.drain(err, "stderr");
        }
        self.last = Some(RunSummary {
            executable: executable.display().to_string(),
            install_path: out.display().to_string(),
            ..Default::default()
        });
        self.child = Some(child);
        Ok(())
    }
    pub fn snapshot(&self, root: &Path) -> BenchSnapshot {
        BenchSnapshot {
            courses: self.courses.clone(),
            games: discover(root),
            running: self.child.is_some(),
            logs: self
                .logs
                .lock()
                .map(|l| l.iter().cloned().collect())
                .unwrap_or_default(),
            last_run: self.last.clone(),
        }
    }
    pub fn poll(&mut self) -> Result<()> {
        if let Some(child) = &mut self.child {
            if let Some(status) = child.try_wait()? {
                if let Some(last) = &mut self.last {
                    last.exit_code = Some(status.code().unwrap_or(-1))
                }
                self.child = None;
            }
        }
        self.workers.retain(|w| !w.is_finished());
        Ok(())
    }
    pub fn play(
        &mut self,
        root: &Path,
        game_id: &str,
        project: &Project,
        session: &RomSession,
    ) -> Result<()> {
        self.play_race(root, game_id, project, session, &RaceSetup::default())
    }
    pub fn play_race(
        &mut self,
        root: &Path,
        game_id: &str,
        project: &Project,
        session: &RomSession,
        race: &RaceSetup,
    ) -> Result<()> {
        let race_arg = race.argument()?;
        self.poll()?;
        if self.child.is_some() {
            return Err(Error::Invalid("stop the running game first".into()));
        }
        if !settings::is_install(root) {
            return Err(Error::Invalid(
                "choose a portable install containing data/ and games/ in Settings".into(),
            ));
        }
        let root = root.canonicalize()?;
        let games = discover(&root);
        let game = if game_id.is_empty() && games.len() == 1 {
            &games[0]
        } else {
            games
                .iter()
                .find(|g| g.id == game_id)
                .ok_or_else(|| Error::Invalid("select an installed game in Test Bench".into()))?
        };
        let folder = PathBuf::from(&game.path);
        let (executable, mut command) = launch_command(&root, &folder, session)?;
        if let Some(arg) = race_arg {
            command.arg("--race").arg(arg);
        }
        let (bytes, dest) = if project.edits.is_empty() && crate::base::changes(session)?.is_empty()
        {
            let enabled = folder.join("mods/enabled.txt");
            ensure_no_symlinks(&root, &enabled)?;
            if enabled.exists() {
                let order = std::fs::read_to_string(&enabled)?
                    .lines()
                    .filter(|s| s.trim() != project.config.manifest.id)
                    .map(|s| format!("{s}\n"))
                    .collect::<String>();
                atomic_write(&enabled, order.as_bytes())?;
            }
            (vec![], folder.clone())
        } else {
            let (bytes, _) = pack::build_legacy(project, session)?;
            let dest = install(&root, &folder, &project.config.manifest.id, &bytes)?;
            (bytes, dest)
        };
        self.logs = Arc::default();
        self.last = Some(RunSummary {
            package_size: bytes.len() as u32,
            install_path: dest.to_string_lossy().into(),
            executable: executable.to_string_lossy().into(),
            ..RunSummary::default()
        });
        let mut child = command.spawn()?;
        if let Some(out) = child.stdout.take() {
            self.drain(out, "stdout")
        }
        if let Some(err) = child.stderr.take() {
            self.drain(err, "stderr")
        }
        self.child = Some(child);
        Ok(())
    }
    pub fn list_courses(&mut self, root: &Path, game_id: &str, session: &RomSession) -> Result<()> {
        let game = discover(root)
            .into_iter()
            .find(|g| g.id == game_id)
            .ok_or_else(|| Error::Invalid("select an installed game first".into()))?;
        let folder = PathBuf::from(game.path);
        let (executable, mut command) = launch_command(root, &folder, session)?;
        let cache_key = format!(
            "{}:{:?}:{}",
            executable.display(),
            std::fs::metadata(&executable)?.modified()?,
            session.sha256
        );
        if self.course_key == cache_key {
            return Ok(());
        }
        command.args([
            "--list-courses",
            "--headless",
            "--instructions",
            "20000000",
            "--mute",
        ]);
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid("missing course output stream".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Invalid("missing course error stream".into()))?;
        let read = |stream: Box<dyn Read + Send>| {
            std::thread::spawn(move || {
                let mut bytes = vec![];
                let result = stream.take(1024 * 1024).read_to_end(&mut bytes);
                (result, bytes)
            })
        };
        let out = read(Box::new(stdout));
        let err = read(Box::new(stderr));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Invalid(
                    "course listing timed out after 20 seconds".into(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let (result, stdout) = out
            .join()
            .map_err(|_| Error::Invalid("course reader stopped".into()))?;
        result?;
        let (result, stderr) = err
            .join()
            .map_err(|_| Error::Invalid("course reader stopped".into()))?;
        result?;
        if !status.success() {
            return Err(Error::Invalid(format!(
                "course listing failed: {}",
                String::from_utf8_lossy(&stderr)
            )));
        }
        self.courses = parse_courses(&String::from_utf8_lossy(&stdout));
        self.course_key = cache_key;
        Ok(())
    }
    fn drain(&mut self, mut reader: impl Read + Send + 'static, stream: &'static str) {
        let logs = self.logs.clone();
        self.workers.push(std::thread::spawn(move || {
            let mut b = [0; 2048];
            loop {
                match reader.read(&mut b) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut logs) = logs.lock() {
                            if logs.len() >= 512 {
                                logs.pop_front();
                            }
                            logs.push_back(format!(
                                "[{stream}] {}",
                                String::from_utf8_lossy(&b[..n])
                            ));
                        }
                    }
                }
            }
        }));
    }
    pub fn stop(&mut self) -> Result<()> {
        self.poll()?;
        if let Some(mut child) = self.child.take() {
            child.kill()?;
            let status = child.wait()?;
            if let Some(last) = &mut self.last {
                last.exit_code = Some(status.code().unwrap_or(-1));
                last.stopped = true;
            }
        }
        Ok(())
    }
}
fn launch_command(root: &Path, folder: &Path, session: &RomSession) -> Result<(PathBuf, Command)> {
    ensure_no_symlinks(root, &folder.join("manifest.json"))?;
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json"))?)?;
    if m["rom"]["sha256"]
        .as_str()
        .is_some_and(|h| !h.eq_ignore_ascii_case(&session.sha256))
    {
        return Err(Error::Invalid(
            "installed game was built from a different ROM".into(),
        ));
    }
    let exe = m["executable"]
        .as_str()
        .ok_or_else(|| Error::Invalid("game manifest lacks executable".into()))?;
    check_path(exe)?;
    if exe.contains('/') {
        return Err(Error::Invalid("executable must be a plain filename".into()));
    }
    let executable = folder.join(exe);
    ensure_no_symlinks(root, &executable)?;
    if !executable.is_file() {
        return Err(Error::Invalid("game executable is missing".into()));
    }
    let mut command = Command::new(&executable);
    command
        .current_dir(folder)
        .env("MKDS_DATA_DIR", root.join("data"))
        .env_remove("MKDS_NO_MODS")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(args) = m["launch_args"].as_array() {
        for arg in args {
            let arg = arg
                .as_str()
                .ok_or_else(|| Error::Invalid("non-string launch argument".into()))?;
            match arg {
                "{rom}" => {
                    command.arg(&session.path);
                }
                "{assets}" => {
                    let assets = m["assets"]["path"].as_str().unwrap_or("assets");
                    check_path(assets)?;
                    let assets = folder.join(assets);
                    ensure_no_symlinks(root, &assets)?;
                    command.arg(assets);
                }
                _ => {
                    command.arg(arg);
                }
            }
        }
    } else {
        command.arg("--rom").arg(&session.path);
    }
    for key in ["MKDS_HW_FS", "MKDS_HW_ARM7", "MKDS_HW_TIME", "MKDS_HW_SND"] {
        if std::env::var_os(key).is_none() {
            command.env(key, "native");
        }
    }
    Ok((executable, command))
}
pub fn show_in_folder(path: &Path) -> Result<()> {
    let path = path.canonicalize()?;
    let folder = if path.is_dir() {
        path.as_path()
    } else {
        path.parent()
            .ok_or_else(|| Error::Invalid("missing parent folder".into()))?
    };
    let opener = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(opener).arg(folder).spawn()?;
    Ok(())
}
impl Drop for Bench {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
