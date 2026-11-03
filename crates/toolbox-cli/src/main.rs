use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use toolbox_core::{preview, service::Service, session::RomSession, Error, Result};

#[derive(Parser)]
#[command(
    name = "mkdstb",
    version,
    about = "MKDSrust Toolbox — headless ROM exploration and mod packaging"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Ls {
        rom: PathBuf,
        path: Option<String>,
    },
    Extract {
        rom: PathBuf,
        path: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    Convert {
        file: PathBuf,
        #[arg(long, value_enum)]
        to: To,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        palette: Option<PathBuf>,
        #[arg(long)]
        screen: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        texture: usize,
        #[arg(long, default_value_t = 0)]
        bank: usize,
        #[arg(long)]
        textures: Option<PathBuf>,
        #[arg(long)]
        animation: Option<PathBuf>,
    },
    Pack {
        project: PathBuf,
    },
    Validate {
        project: PathBuf,
    },
}
#[derive(Clone, ValueEnum)]
enum To {
    Png,
    Json,
    Glb,
    Obj,
}
fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Ls { rom, path } => {
            let session = RomSession::open(&rom)?;
            for warning in &session.warnings {
                eprintln!("Warning: {warning}");
            }
            for f in &session.nodes {
                if path.as_ref().is_none_or(|p| {
                    f.path == *p || f.path.starts_with(&format!("{}/", p.trim_end_matches('/')))
                }) {
                    println!("{:>10}  {:<7}  {}", f.size, f.format, f.path);
                }
            }
        }
        Command::Extract { rom, path, output } => {
            let session = RomSession::open(&rom)?;
            let service = Service {
                session: Some(session),
                persist_settings: false,
                ..Service::default()
            };
            service.check_output(&output)?;
            toolbox_core::project::atomic_write(
                &output,
                service
                    .session
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("missing ROM".into()))?
                    .file(&path)?,
            )?;
        }
        Command::Convert {
            file,
            to,
            output,
            palette,
            screen,
            texture,
            bank,
            textures,
            animation,
        } => {
            let bytes = std::fs::read(&file)?;
            let ext = match to {
                To::Png => "png",
                To::Json => "json",
                To::Glb => "glb",
                To::Obj => "obj",
            };
            let output = output.unwrap_or_else(|| file.with_extension(ext));
            if output == file {
                return Err(Error::Invalid("output must differ from input".into()));
            }
            let data = match to {
                To::Glb => {
                    let model = nitro_formats::model::Nsbmd::parse(&preview::plain(&bytes)?)?;
                    let textures = textures
                        .map(|p| {
                            Ok::<_, Error>(nitro_formats::texture::Nsbtx::parse(&preview::plain(
                                &std::fs::read(p)?,
                            )?)?)
                        })
                        .transpose()?;
                    let animations = animation
                        .map(|p| {
                            Ok::<_, Error>(nitro_formats::animation::parse(&preview::plain(
                                &std::fs::read(p)?,
                            )?)?)
                        })
                        .transpose()?
                        .unwrap_or_default();
                    toolbox_core::model::glb(Some(&model), textures.as_ref(), &animations, None)?
                }
                To::Obj => toolbox_core::model::obj(&nitro_formats::kcl::Kcl::parse(
                    &preview::plain(&bytes)?,
                )?)?,
                To::Json => preview::convert_json(&bytes)?,
                To::Png => {
                    let mut files = std::collections::BTreeMap::from([("image".into(), bytes)]);
                    if let Some(p) = palette {
                        files.insert("palette".into(), std::fs::read(p)?);
                    }
                    if let Some(p) = screen {
                        files.insert("screen".into(), std::fs::read(p)?);
                    }
                    // Reuse exactly the app's renderer in an in-memory synthetic filesystem.
                    let session = RomSession::from_bytes(
                        toolbox_core::synthetic::rom_with(files),
                        PathBuf::new(),
                    )?;
                    preview::png(&preview::image(
                        None,
                        &session,
                        "image",
                        Some("palette"),
                        session.file("screen").ok().map(|_| "screen"),
                        texture,
                        bank,
                    )?)?
                }
            };
            std::fs::write(&output, data)?;
            println!("{}", output.display());
        }
        Command::Pack { project } => {
            let (p, s) = Service::project_session(&project)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&toolbox_core::pack::pack(&p, &s)?)?
            );
        }
        Command::Validate { project } => {
            let (p, s) = Service::project_session(&project)?;
            let issues = toolbox_core::validation::validate(&p, &s);
            println!("{}", serde_json::to_string_pretty(&issues)?);
            if issues
                .iter()
                .any(|i| matches!(i.severity, toolbox_core::validation::Severity::Error))
            {
                return Err(Error::Invalid("validation failed".into()));
            }
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
