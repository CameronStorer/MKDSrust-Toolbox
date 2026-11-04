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
    pub cells: Vec<u32>,
    pub sequences: Vec<Vec<CellFrame>>,
    pub colors: Vec<u16>,
    pub palette_stride: u32,
    pub messages: Vec<TextMessage>,
    pub textures: Vec<String>,
    pub palettes: Vec<String>,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct CellFrame {
    pub cell: u32,
    pub duration: u32,
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
    if b.starts_with(b"BTX0") || (b.starts_with(b"BMD0") && Nsbtx::parse(&b).is_ok()) {
        let t = Nsbtx::parse(&b)?;
        p.textures = t.textures.iter().map(|t| t.name.clone()).collect();
        p.colors = t
            .palettes
            .first()
            .map(|p| p.colors.clone())
            .unwrap_or_default();
        p.palette_stride = p.colors.len() as u32;
        p.palettes = t.palettes.iter().map(|p| p.name.clone()).collect();
        if let Some(t) = t.textures.first() {
            p.width = t.width;
            p.height = t.height;
        }
    }
    if b.starts_with(b"RECN") {
        p.cells = nitro_formats::cell::Ncer::parse(&b)?
            .cells
            .iter()
            .map(|c| c.objects.len() as u32)
            .collect();
    }
    if b.starts_with(b"RNAN") {
        p.sequences = nitro_formats::cell::Nanr::parse(&b)?
            .sequences
            .iter()
            .map(|s| {
                s.frames
                    .iter()
                    .map(|f| CellFrame {
                        cell: f.cell as u32,
                        duration: f.duration as u32,
                    })
                    .collect()
            })
            .collect();
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
    if b.starts_with(b"RECN") || b.starts_with(b"RNAN") {
        return cell_image(
            project,
            session,
            path,
            palette_path,
            screen_path,
            texture,
            0,
        );
    }
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
    if b.starts_with(b"BTX0") || b.starts_with(b"BMD0") {
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
pub fn cell_image(
    project: Option<&Project>,
    session: &RomSession,
    path: &str,
    palette: Option<&str>,
    cell_path: Option<&str>,
    sequence: usize,
    frame: usize,
) -> Result<Image> {
    let bytes = |p: &str| -> Result<Vec<u8>> {
        plain(match project {
            Some(pj) => pj.bytes(session, p)?,
            None => session.file(p)?,
        })
    };
    let b = bytes(path)?;
    let ncer_path = if b.starts_with(b"RECN") {
        path.to_string()
    } else {
        cell_path
            .map(String::from)
            .or_else(|| find_companion(session, path, "ncer"))
            .ok_or_else(|| Error::Invalid("choose a companion NCER".into()))?
    };
    let cells = nitro_formats::cell::Ncer::parse(&bytes(&ncer_path)?)?;
    let tile_path = find_companion(session, &ncer_path, "ncgr")
        .ok_or_else(|| Error::Invalid("choose matching NCGR tiles beside the NCER".into()))?;
    let pal_path = palette
        .map(String::from)
        .or_else(|| find_companion(session, &ncer_path, "nclr"))
        .ok_or_else(|| Error::Invalid("choose a companion NCLR".into()))?;
    let tiles = Ncgr::parse(&bytes(&tile_path)?)?;
    let pal = Nclr::parse(&bytes(&pal_path)?)?;
    if b.starts_with(b"RNAN") {
        let anim = nitro_formats::cell::Nanr::parse(&b)?;
        let f = anim
            .sequences
            .get(sequence)
            .and_then(|s| s.frames.get(frame))
            .ok_or_else(|| Error::Invalid("animation frame outside sequence".into()))?;
        Ok(nitro_formats::cell::transform(
            &cells.render(f.cell as usize, &tiles, &pal)?,
            f,
        ))
    } else {
        Ok(cells.render(sequence, &tiles, &pal)?)
    }
}
pub fn find_companion(session: &RomSession, path: &str, ext: &str) -> Option<String> {
    crate::game::companion(&session.nodes, path, ext)
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
