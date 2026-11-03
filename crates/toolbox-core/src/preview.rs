use crate::{project::Project, session::RomSession, Error, Result};
use nitro_formats::{
    bmg::{Bmg, Token},
    graphics::{bgr555_to_rgba, Image, Ncgr, Nclr, Nscr},
    texture::Nsbtx,
};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "value")]
pub enum TextToken {
    Text(String),
    Code(Vec<u8>),
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct TextMessage {
    pub id: u32,
    pub tokens: Vec<TextToken>,
    pub preview: String,
}
#[derive(Clone, Debug, Serialize, Type, Default)]
pub struct Preview {
    pub colors: Vec<u16>,
    pub palette_stride: u32,
    pub messages: Vec<TextMessage>,
    pub textures: Vec<String>,
    pub palettes: Vec<String>,
    pub width: u32,
    pub height: u32,
}
pub fn plain(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(if bytes.first() == Some(&0x10) {
        nitro_formats::decompress(bytes)?
    } else {
        bytes.to_vec()
    })
}
pub fn preview(bytes: &[u8]) -> Result<Preview> {
    let b = plain(bytes)?;
    let mut p = Preview::default();
    if b.starts_with(b"RLCN") {
        let palette = Nclr::parse(&b)?;
        p.palette_stride = if palette.depth == 3 { 16 } else { 256 };
        p.colors = palette.colors;
    }
    if b.starts_with(b"MESGbmg1") {
        p.messages = Bmg::parse(&b)?
            .messages
            .into_iter()
            .map(|m| TextMessage {
                id: m.id,
                preview: nitro_formats::bmg::text(&m.tokens),
                tokens: m
                    .tokens
                    .into_iter()
                    .map(|t| match t {
                        Token::Text(s) => TextToken::Text(String::from_utf16_lossy(&s)),
                        Token::Code(c) => TextToken::Code(c),
                    })
                    .collect(),
            })
            .collect();
    }
    if b.starts_with(b"BTX0") {
        let t = Nsbtx::parse(&b)?;
        p.textures = t.textures.iter().map(|t| t.name.clone()).collect();
        p.palettes = t.palettes.iter().map(|p| p.name.clone()).collect();
        if let Some(t) = t.textures.first() {
            p.width = t.width;
            p.height = t.height;
        }
    }
    Ok(p)
}
pub fn image(
    project: Option<&Project>,
    session: &RomSession,
    path: &str,
    palette_path: Option<&str>,
    screen_path: Option<&str>,
    texture: usize,
    bank: usize,
) -> Result<Image> {
    let bytes = |p: &str| -> Result<Vec<u8>> {
        plain(match project {
            Some(project) => project.bytes(session, p)?,
            None => session.file(p)?,
        })
    };
    let b = bytes(path)?;
    if b.starts_with(b"RLCN") {
        let p = Nclr::parse(&b)?;
        let width = 16;
        let height = p.colors.len().div_ceil(16).max(1);
        let mut rgba = vec![0; width * height * 4];
        for (i, c) in p.colors.iter().enumerate() {
            rgba[i * 4..i * 4 + 4].copy_from_slice(&bgr555_to_rgba(*c));
        }
        return Ok(Image {
            width: width as u32,
            height: height as u32,
            rgba,
        });
    }
    if b.starts_with(b"BTX0") {
        return Ok(Nsbtx::parse(&b)?.decode(texture, bank)?);
    }
    let screen = if b.starts_with(b"RCSN") {
        Some(Nscr::parse(&b)?)
    } else {
        screen_path
            .map(|p| bytes(p).and_then(|b| Ok(Nscr::parse(&b)?)))
            .transpose()?
    };
    let tile_path = if b.starts_with(b"RCSN") {
        find_companion(session, path, "ncgr")
            .ok_or_else(|| Error::Invalid("select companion NCGR tiles".into()))?
    } else {
        path.to_string()
    };
    let tile = Ncgr::parse(&bytes(&tile_path)?)?;
    let palette_path = palette_path
        .map(String::from)
        .or_else(|| find_companion(session, path, "nclr"))
        .ok_or_else(|| Error::Invalid("select a companion NCLR palette".into()))?;
    let palette = Nclr::parse(&bytes(&palette_path)?)?;
    Ok(tile.render(&palette, screen.as_ref(), bank)?)
}
pub fn find_companion(session: &RomSession, path: &str, ext: &str) -> Option<String> {
    let stem = path.rsplit_once('.').map_or(path, |x| x.0);
    let exact = format!("{stem}.{ext}");
    if session.file(&exact).is_ok() {
        return Some(exact);
    }
    let parent = path.rsplit_once('/').map_or("", |x| x.0);
    session
        .nodes
        .iter()
        .find(|n| n.format == ext && n.path.rsplit_once('/').map_or("", |x| x.0) == parent)
        .map(|n| n.path.clone())
}
pub fn png(image: &Image) -> Result<Vec<u8>> {
    let mut out = vec![];
    {
        let mut e = png::Encoder::new(&mut out, image.width, image.height);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e
            .write_header()
            .map_err(|e| Error::Invalid(e.to_string()))?;
        w.write_image_data(&image.rgba)
            .map_err(|e| Error::Invalid(e.to_string()))?;
    }
    Ok(out)
}
pub fn convert_json(bytes: &[u8]) -> Result<Vec<u8>> {
    let b = plain(bytes)?;
    let p = preview(&b)?;
    if b.starts_with(b"RLCN") || b.starts_with(b"MESGbmg1") || b.starts_with(b"BTX0") {
        return Ok(serde_json::to_vec_pretty(&p)?);
    }
    if b.starts_with(b"RGCN") {
        let n = Ncgr::parse(&b)?;
        return Ok(serde_json::to_vec_pretty(
            &serde_json::json!({"width_tiles":n.width_tiles,"height_tiles":n.height_tiles,"depth":n.depth,"bitmap":n.bitmap,"pixels":n.pixels}),
        )?);
    }
    if b.starts_with(b"RCSN") {
        let n = Nscr::parse(&b)?;
        return Ok(serde_json::to_vec_pretty(
            &serde_json::json!({"width":n.width,"height":n.height,"entries":n.entries}),
        )?);
    }
    Err(Error::Invalid(
        "JSON conversion supports BMG, NCLR, NCGR, NSCR and NSBTX".into(),
    ))
}
