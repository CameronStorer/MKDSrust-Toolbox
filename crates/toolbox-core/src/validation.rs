use crate::{project::Project, session::RomSession};
use nitro_formats::{
    bmg::{Bmg, Token},
    graphics::Nclr,
    texture::Nsbtx,
};
use serde::Serialize;
use specta::Type;
#[derive(Clone, Debug, Serialize, Type)]
pub enum Severity {
    Warning,
    Error,
}
pub type FixId = String;
#[derive(Clone, Debug, Serialize, Type)]
pub struct Issue {
    pub severity: Severity,
    pub location: String,
    pub message: String,
    pub why: String,
    pub fix: Option<FixId>,
}
pub trait Check {
    fn check(&self, path: &str, bytes: &[u8]) -> Vec<Issue>;
}
pub struct Stage1Check;
impl Check for Stage1Check {
    fn check(&self, path: &str, bytes: &[u8]) -> Vec<Issue> {
        check_file(path, bytes, 0, &mut (64 * 1024 * 1024))
    }
}
fn invalid(path: &str, message: String) -> Vec<Issue> {
    vec![Issue {
        severity: Severity::Error,
        location: path.into(),
        message,
        why: "Malformed data cannot be loaded safely".into(),
        fix: None,
    }]
}
fn check_file(path: &str, bytes: &[u8], depth: usize, budget: &mut usize) -> Vec<Issue> {
    if depth > 8 || bytes.len() > *budget {
        return invalid(path, "validation expansion limit exceeded".into());
    }
    *budget -= bytes.len();
    if bytes.first() == Some(&0x10) {
        return match nitro_formats::decompress(bytes) {
            Ok(b) => check_file(path, &b, depth + 1, budget),
            Err(e) => invalid(path, e.to_string()),
        };
    }
    if bytes.starts_with(b"NARC") {
        return match nitro_formats::archive::Narc::parse(bytes) {
            Ok(a) => a
                .entries
                .into_iter()
                .flat_map(|e| check_file(&format!("{path}/{}", e.name), &e.data, depth + 1, budget))
                .collect(),
            Err(e) => invalid(path, e.to_string()),
        };
    }
    let mut issues = vec![];
    let parsed = if bytes.starts_with(b"RGCN") {
        nitro_formats::graphics::Ncgr::parse(bytes).map(|_| ())
    } else if bytes.starts_with(b"RCSN") {
        nitro_formats::graphics::Nscr::parse(bytes).map(|_| ())
    } else if bytes.starts_with(b"NKMD") {
        nitro_formats::nkm::Nkm::parse(bytes).map(|_| ())
    } else if path.to_ascii_lowercase().ends_with(".kcl") {
        nitro_formats::kcl::Kcl::parse(bytes).map(|_| ())
    } else {
        Ok(())
    };
    if let Err(e) = parsed {
        return invalid(path, e.to_string());
    }
    if bytes.starts_with(b"NKMD") {
        let n = nitro_formats::nkm::Nkm::parse(bytes).expect("validated above");
        // Mission/battle maps have different requirements. Race wizard and
        // retail race maps use checkpoints and CPU routes.
        if n.section("CPOI").is_some_and(|s| !s.entries.is_empty())
            || n.section("EPOI").is_some_and(|s| !s.entries.is_empty())
        {
            for message in crate::track_wizard::validate(&n) {
                issues.push(Issue {
                    severity: Severity::Error,
                    location: path.into(),
                    message,
                    why: "A race needs closed routes, checkpoints, starts and respawns".into(),
                    fix: Some("track-routes".into()),
                });
            }
        }
    }
    if path.to_ascii_lowercase().ends_with(".kcl") {
        if bytes.len() > nitro_formats::kcl::MAX_BYTES {
            return vec![Issue {
                severity: Severity::Error,
                location: path.into(),
                message: format!(
                    "KCL uses {} bytes; maximum is 215 KB. Compact the tree or simplify collision",
                    bytes.len()
                ),
                why: "Course collision must fit the fixed DS course heap".into(),
                fix: Some("collision-budget".into()),
            }];
        }
        if bytes.len() > nitro_formats::kcl::WARN_BYTES {
            issues.push(Issue {
                severity: Severity::Warning,
                location: path.into(),
                message: format!(
                    "KCL uses {} bytes; above the 150 KB warning budget",
                    bytes.len()
                ),
                why: "Course collision must fit the fixed DS course heap".into(),
                fix: Some("collision-budget".into()),
            });
        }
    }
    let mut add = |message: String, why: &str| {
        issues.push(Issue {
            severity: Severity::Warning,
            location: path.into(),
            message,
            why: why.into(),
            fix: None,
        })
    };
    if nitro_formats::detect::detect(bytes, path).is_none() {
        add(
            "DS compatibility of this file is unknown".into(),
            "Use a supported DS data format to verify compatibility",
        );
    }
    if bytes.starts_with(b"RLCN") {
        match Nclr::parse(bytes) {
            Ok(p) => {
                let bank = if p.depth == 3 { 16 } else { 256 };
                let max = if p.extended { 4096 } else { 256 };
                if p.colors.is_empty() || p.colors.len() % bank != 0 || p.colors.len() > max {
                    add(format!("Palette has {} colours",p.colors.len()),"DS palettes require complete 16/256-colour banks within their palette allocation");
                }
            }
            Err(e) => return invalid(path, e.to_string()),
        }
    }
    if bytes.starts_with(b"MESGbmg1") {
        match Bmg::parse(bytes) {
            Ok(b) => {
                for m in b.messages {
                    let len: usize = m
                        .tokens
                        .iter()
                        .map(|t| match t {
                            Token::Text(s) => s.len(),
                            _ => 0,
                        })
                        .sum();
                    if len > mkds_data::MAX_MESSAGE_UNITS {
                        add(format!("Message {} has {len} UTF-16 units",m.id),"Long messages can overflow the game's text box; check wrapping in game");
                    }
                }
            }
            Err(e) => return invalid(path, e.to_string()),
        }
    }
    if bytes.starts_with(b"BTX0") {
        match Nsbtx::parse(bytes) {
            Ok(b) => {
                let texture_bytes: usize = b
                    .textures
                    .iter()
                    .map(|t| t.data.len() + t.indices.len())
                    .sum();
                let palette_bytes: usize = b.palettes.iter().map(|p| p.colors.len() * 2).sum();
                if texture_bytes > 512 * 1024 {
                    add(
                        format!("Texture VRAM: {texture_bytes} bytes / 512 KiB"),
                        "The DS texture memory allocation is limited",
                    );
                }
                if palette_bytes > 96 * 1024 {
                    add(
                        format!("Palette VRAM: {palette_bytes} bytes / 96 KiB"),
                        "The DS palette memory allocation is limited",
                    );
                }
                for t in b.textures {
                    if t.width > 1024
                        || t.height > 1024
                        || !t.width.is_power_of_two()
                        || !t.height.is_power_of_two()
                    {
                        add(
                            format!("{}: invalid {}×{} dimensions", t.name, t.width, t.height),
                            "DS texture dimensions must be powers of two, at most 1024 pixels",
                        );
                    }
                }
            }
            Err(e) => return invalid(path, e.to_string()),
        }
    }
    if bytes.starts_with(b"RECN") {
        if let Err(e) = nitro_formats::cell::Ncer::parse(bytes) {
            return invalid(path, e.to_string());
        }
    }
    if bytes.starts_with(b"RNAN") {
        if let Err(e) = nitro_formats::cell::Nanr::parse(bytes) {
            return invalid(path, e.to_string());
        }
    }
    if bytes.starts_with(b"BMD0") {
        match nitro_formats::model::Nsbmd::parse(bytes) {
            Err(e) => return invalid(path, e.to_string()),
            Ok(b) => {
                for reason in crate::model::budget(&b, None).reasons {
                    add(
                        reason,
                        "The DS geometry engine and texture memory are limited",
                    );
                }
                for m in b.models {
                    for w in m.warnings {
                        add(w, "This model uses features the reader cannot verify");
                    }
                }
            }
        }
    }
    if bytes.starts_with(b"SDAT") {
        match nitro_formats::audio::Sdat::parse(bytes).and_then(|s| s.validate_files()) {
            Ok(()) => {}
            Err(e) => return invalid(path, e.to_string()),
        }
    }
    issues
}
pub fn validate(project: &Project, session: &RomSession) -> Vec<Issue> {
    let mut out = vec![];
    if session.rom.bytes().len() > 256 * 1024 * 1024 {
        out.extend(invalid("ROM", "ROM exceeds the 256 MiB DS limit".into()));
    }
    if let Ok(changes) = crate::base::changes(session) {
        for c in changes {
            if let Ok(bytes) = session.file(&c.path) {
                out.extend(Stage1Check.check(&c.path, bytes));
            }
        }
    }
    let track_limits = project
        .edits
        .keys()
        .any(|p| crate::track_wizard::file_role(p).is_some())
        .then(|| crate::track_wizard::file_budgets(session));
    for path in project.edits.keys() {
        if path.starts_with("system/overlay9/") && path.ends_with(".uncompressed") {
            continue;
        }
        match project.bytes(session, path) {
            Ok(b) => {
                out.extend(Stage1Check.check(path, b));
                if crate::track_wizard::file_role(path).is_some() {
                    if let Some(limits) = &track_limits {
                        let check = match limits {
                            Ok(limits) => crate::track_wizard::check_file_size(path, b, limits),
                            Err(e) => Err(crate::Error::Invalid(e.to_string())),
                        };
                        if let Err(e) = check {
                            out.extend(invalid(path, e.to_string()));
                        }
                    }
                }
            }
            Err(e) => out.push(Issue {
                severity: Severity::Error,
                location: path.clone(),
                message: e.to_string(),
                why: "Cannot read edit".into(),
                fix: None,
            }),
        }
    }
    out
}
