//! Rust owns the ROM session, persistent project, edits and global command history.
pub mod audio;
pub mod base;
pub mod bench;
pub mod collision;
pub mod engine;
pub mod export;
pub mod graphic;
pub mod model;
pub mod model_import;
pub mod pack;
pub mod preview;
pub mod project;
pub mod scene;
pub mod service;
pub mod session;
pub mod settings;
mod source;
pub mod synthetic;
pub mod validation;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Format(#[from] nitro_formats::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    TomlRead(#[from] toml::de::Error),
    #[error(transparent)]
    TomlWrite(#[from] toml::ser::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}
pub type Result<T> = std::result::Result<T, Error>;
