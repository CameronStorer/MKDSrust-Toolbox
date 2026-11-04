//! Palette-aware raster edits and deterministic median-cut conversion; no SDK tools required.
use crate::{preview, project::Project, session::RomSession, Error, Result};
use nitro_formats::{
    graphics::{bgr555_to_rgba, rgba_to_bgr555, Image, Ncgr, Nclr},
    texture::{self, Nsbtx},
};
use serde::Serialize;
use specta::Type;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Serialize, Type)]
pub struct ImportInfo {
    pub width: u32,
    pub height: u32,
    pub colors: u32,
    pub texture_bytes: u32,
    pub palette_bytes: u32,
}
pub struct ImportDraft {
    pub edits: BTreeMap<String, Vec<u8>>,
    pub image: Image,
    pub info: ImportInfo,
}

fn wrap(original: &[u8], bytes: Vec<u8>) -> Vec<u8> {
    if original.first() == Some(&0x10) {
        nitro_formats::recompress(original, &bytes)
    } else {
        bytes
    }
}
pub fn read_png(path: &Path) -> Result<Image> {
    read_png_data(&std::fs::read(path)?)
}
/// glTF 2.0 permits PNG and JPEG images. Bound dimensions and the decoder's
/// allocation before expanding either into the shared RGBA representation.
pub fn read_image_data(data: &[u8]) -> Result<Image> {
    if !data.starts_with(&[0xff, 0xd8]) {
        return read_png_data(data);
    }
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(data));
    decoder.set_max_decoding_buffer_size(64 * 1024 * 1024);
    decoder
        .read_info()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Invalid("JPEG has no image header".into()))?;
    if info.width == 0 || info.height == 0 || info.width > 4096 || info.height > 4096 {
        return Err(Error::Invalid("JPEG exceeds 4096 pixels per side".into()));
    }
    let bytes = decoder
        .decode()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut rgba = vec![];
    match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            for &v in &bytes {
                rgba.extend([v, v, v, 255]);
            }
        }
        jpeg_decoder::PixelFormat::RGB24 => {
            for c in bytes.chunks_exact(3) {
                rgba.extend([c[0], c[1], c[2], 255]);
            }
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            for c in bytes.chunks_exact(4) {
                let k = c[3] as u16;
                rgba.extend([
                    (c[0] as u16 * k / 255) as u8,
                    (c[1] as u16 * k / 255) as u8,
                    (c[2] as u16 * k / 255) as u8,
                    255,
                ]);
            }
        }
        _ => {
            return Err(Error::Invalid(
                "convert 16-bit JPEG to 8-bit RGB before import".into(),
            ))
        }
    }
    Ok(Image {
        width: info.width as u32,
        height: info.height as u32,
        rgba,
    })
}
pub fn read_png_data(data: &[u8]) -> Result<Image> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    decoder.set_limits(png::Limits {
        bytes: 32 * 1024 * 1024,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut bytes = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| Error::Invalid("PNG too large".into()))?
    ];
    let info = reader
        .next_frame(&mut bytes)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    if info.width > 4096 || info.height > 4096 {
        return Err(Error::Invalid("PNG exceeds 4096 pixels per side".into()));
    }
    let mut rgba = vec![];
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(&bytes[..info.buffer_size()]),
        png::ColorType::Rgb => {
            for c in bytes[..info.buffer_size()].chunks_exact(3) {
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for c in &bytes[..info.buffer_size()] {
                rgba.extend_from_slice(&[*c, *c, *c, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for c in bytes[..info.buffer_size()].chunks_exact(2) {
                rgba.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        _ => return Err(Error::Invalid("unsupported PNG colour type".into())),
    }
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}
pub fn quantize(
    image: &Image,
    count: usize,
    dither: bool,
    transparent: bool,
) -> (Vec<u16>, Vec<u8>) {
    let count = count.clamp(2, 256);
    let reserve = usize::from(transparent);
    let mut histogram = BTreeMap::<u16, u32>::new();
    for c in image
        .rgba
        .chunks_exact(4)
        .filter(|c| if transparent { c[3] >= 128 } else { c[3] > 0 })
    {
        *histogram
            .entry(rgba_to_bgr555([c[0], c[1], c[2], c[3]]))
            .or_default() += 1;
    }
    let channel = |c: u16, k: usize| (c >> (k * 5)) & 31;
    let mut boxes = vec![histogram.into_iter().collect::<Vec<_>>()];
    while boxes.len() < count - reserve {
        let best = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .map(|(i, b)| {
                let (axis, range) = (0..3)
                    .map(|k| {
                        (
                            k,
                            b.iter().map(|(c, _)| channel(*c, k)).max().unwrap_or(0)
                                - b.iter().map(|(c, _)| channel(*c, k)).min().unwrap_or(0),
                        )
                    })
                    .max_by_key(|(_, r)| *r)
                    .unwrap();
                (
                    i,
                    axis,
                    range as u64 * b.iter().map(|(_, n)| *n as u64).sum::<u64>(),
                )
            })
            .max_by_key(|(_, _, s)| *s);
        let Some((i, axis, _)) = best else {
            break;
        };
        let mut b = boxes.remove(i);
        b.sort_by_key(|(c, _)| channel(*c, axis));
        let total = b.iter().map(|(_, n)| *n as u64).sum::<u64>();
        let mut sum = 0;
        let mut split = 1;
        for (j, (_, n)) in b.iter().enumerate().take(b.len() - 1) {
            sum += *n as u64;
            split = j + 1;
            if sum * 2 >= total {
                break;
            }
        }
        let tail = b.split_off(split);
        boxes.push(b);
        boxes.push(tail);
    }
    let mut palette = vec![0; reserve];
    for b in boxes {
        let total = b.iter().map(|(_, n)| *n as u64).sum::<u64>().max(1);
        let mut c = 0;
        for k in 0..3 {
            let avg = b
                .iter()
                .map(|(c, n)| channel(*c, k) as u64 * *n as u64)
                .sum::<u64>()
                / total;
            c |= (avg as u16) << (k * 5);
        }
        palette.push(c);
    }
    palette.resize(count, 0);
    (
        palette.clone(),
        map_indices(image, &palette, dither, transparent),
    )
}
pub fn map_indices(image: &Image, palette: &[u16], dither: bool, transparent: bool) -> Vec<u8> {
    let rgb: Vec<_> = palette.iter().map(|c| bgr555_to_rgba(*c)).collect();
    let mut work: Vec<[f32; 3]> = image
        .rgba
        .chunks_exact(4)
        .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32])
        .collect();
    let w = image.width as usize;
    let mut out = vec![0; work.len()];
    for i in 0..work.len() {
        if transparent && image.rgba[i * 4 + 3] < 128 {
            continue;
        }
        let start = usize::from(transparent && palette.len() > 1);
        let index = (start..rgb.len())
            .min_by(|a, b| {
                let dist = |n: usize| {
                    (0..3)
                        .map(|k| (work[i][k] - rgb[n][k] as f32).powi(2))
                        .sum::<f32>()
                };
                dist(*a).total_cmp(&dist(*b))
            })
            .unwrap_or(0);
        out[i] = index as u8;
        if dither {
            let error = std::array::from_fn::<_, 3, _>(|k| work[i][k] - rgb[index][k] as f32);
            for (dx, dy, weight) in [(1, 0, 7.), (-1, 1, 3.), (0, 1, 5.), (1, 1, 1.)] {
                let x = (i % w) as i32 + dx;
                let y = (i / w) as i32 + dy;
                if x >= 0 && x < w as i32 && y < image.height as i32 {
                    let j = y as usize * w + x as usize;
                    for (k, e) in error.iter().enumerate() {
                        work[j][k] += e * weight / 16.;
                    }
                }
            }
        }
    }
    out
}
fn tile_offset(tile: &Ncgr, x: usize, y: usize, width: usize) -> usize {
    if tile.bitmap {
        y * width + x
    } else {
        (y / 8 * (width / 8) + x / 8) * 64 + (y % 8) * 8 + x % 8
    }
}
pub fn paint(
    project: &mut Project,
    session: &RomSession,
    path: &str,
    palette: Option<&str>,
    texture: usize,
    bank: usize,
    points: &[(u32, u32, u16)],
) -> Result<()> {
    if points.len() > 1_048_576 {
        return Err(Error::Invalid("paint stroke too large".into()));
    }
    let original = project.bytes(session, path)?;
    let plain = preview::plain(original)?;
    let b = if plain.starts_with(b"RGCN") {
        let mut tile = Ncgr::parse(&plain)?;
        let width = tile.width_tiles as usize;
        let width = if width == 0 || width == 65535 {
            8
        } else {
            width
        } * 8;
        let height = (tile.pixels.len() / 64).div_ceil(width / 8) * 8;
        for &(x, y, c) in points {
            if x as usize >= width
                || y as usize >= height
                || c >= if tile.depth == 3 { 16 } else { 256 }
            {
                return Err(Error::Invalid(
                    "paint pixel or palette index out of bounds".into(),
                ));
            }
            let i = tile_offset(&tile, x as usize, y as usize, width);
            *tile
                .pixels
                .get_mut(i)
                .ok_or_else(|| Error::Invalid("paint outside tile data".into()))? = c as u8;
        }
        tile.write()
    } else {
        let tx = Nsbtx::parse(&plain)?;
        let t = tx
            .textures
            .get(texture)
            .ok_or_else(|| Error::Invalid("texture index out of bounds".into()))?;
        let mut value = t.clone();
        for &(x, y, c) in points {
            if x >= t.width || y >= t.height {
                return Err(Error::Invalid("paint outside texture".into()));
            }
            let i = (y * t.width + x) as usize;
            let limit = match t.format {
                1 => 32,
                2 => 4,
                3 => 16,
                4 => 256,
                6 => 8,
                7 => 32768,
                _ => {
                    return Err(Error::Invalid(
                        "convert 4x4 compressed textures before painting".into(),
                    ))
                }
            };
            if c as usize >= limit {
                return Err(Error::Invalid(
                    "palette index outside texture format".into(),
                ));
            }
            match t.format {
                1 => value.data[i] = c as u8 | 0xe0,
                2 => {
                    let shift = i % 4 * 2;
                    value.data[i / 4] = (value.data[i / 4] & !(3 << shift)) | ((c as u8) << shift);
                }
                3 => {
                    let shift = i % 2 * 4;
                    value.data[i / 2] = (value.data[i / 2] & !(15 << shift)) | ((c as u8) << shift);
                }
                4 => value.data[i] = c as u8,
                6 => value.data[i] = c as u8 | 0xf8,
                7 => value.data[i * 2..i * 2 + 2].copy_from_slice(&(c | 0x8000).to_le_bytes()),
                _ => unreachable!(),
            }
        }
        tx.replace(texture, bank, &value, None)?
    };
    let b = wrap(original, b);
    let _ = palette;
    project.edit(path, b, "Paint pixels", session)
}
pub fn paint_data(bytes: &[u8], texture: usize, bank: usize) -> Result<Vec<u8>> {
    let b = preview::plain(bytes)?;
    let (width, height, format, pixels) = if b.starts_with(b"RGCN") {
        let tile = Ncgr::parse(&b)?;
        let w = tile.width_tiles as usize;
        let w = if w == 0 || w == 65535 { 8 } else { w } * 8;
        let h = (tile.pixels.len() / 64).div_ceil(w / 8) * 8;
        let mut pixels = vec![0; w * h];
        for y in 0..h {
            for x in 0..w {
                pixels[y * w + x] = tile
                    .pixels
                    .get(tile_offset(&tile, x, y, w))
                    .copied()
                    .unwrap_or(0) as u16;
            }
        }
        (w as u32, h as u32, 0u32, pixels)
    } else {
        let tx = Nsbtx::parse(&b)?;
        let t = tx
            .textures
            .get(texture)
            .ok_or_else(|| Error::Invalid("texture index outside dictionary".into()))?;
        let n = t.width as usize * t.height as usize;
        let mut pixels = vec![];
        for i in 0..n {
            pixels.push(match t.format {
                1 => t.data[i] as u16 & 31,
                2 => (t.data[i / 4] >> (i % 4 * 2)) as u16 & 3,
                3 => (t.data[i / 2] >> (i % 2 * 4)) as u16 & 15,
                4 => t.data[i] as u16,
                6 => t.data[i] as u16 & 7,
                7 => u16::from_le_bytes([t.data[i * 2], t.data[i * 2 + 1]]) & 0x7fff,
                _ => {
                    return Err(Error::Invalid(
                        "convert 4x4 compressed textures before painting".into(),
                    ))
                }
            });
        }
        let _ = bank;
        (t.width, t.height, t.format as u32, pixels)
    };
    let mut out = vec![];
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&format.to_le_bytes());
    for p in pixels {
        out.extend_from_slice(&p.to_le_bytes());
    }
    Ok(out)
}
#[allow(clippy::too_many_arguments)]
pub fn import(
    project: &Project,
    session: &RomSession,
    path: &str,
    source: &Path,
    palette: Option<&str>,
    texture: usize,
    bank: usize,
    format: u8,
    dither: bool,
) -> Result<ImportDraft> {
    let image = read_png(source)?;
    let original = project.bytes(session, path)?;
    let plain = preview::plain(original)?;
    let mut edits = BTreeMap::new();
    let (render, info) = if plain.starts_with(b"RGCN") {
        let mut tile = Ncgr::parse(&plain)?;
        let pal_path = palette
            .map(String::from)
            .or_else(|| preview::find_companion(session, path, "nclr"))
            .ok_or_else(|| Error::Invalid("choose a palette".into()))?;
        let pal_bytes = project.bytes(session, &pal_path)?;
        let mut pal = Nclr::parse(&preview::plain(pal_bytes)?)?;
        let current = tile.render(&pal, None, bank)?;
        if image.width != current.width || image.height != current.height {
            return Err(Error::Invalid(format!(
                "PNG must be {} × {} pixels",
                current.width, current.height
            )));
        }
        let count = if tile.depth == 3 { 16 } else { 256 };
        let (colors, indices) = quantize(&image, count, dither, true);
        let offset = bank * count;
        if offset + count > pal.colors.len() {
            return Err(Error::Invalid("palette bank outside palette".into()));
        }
        pal.colors[offset..offset + count].copy_from_slice(&colors);
        for y in 0..image.height as usize {
            for x in 0..image.width as usize {
                let dest = tile_offset(&tile, x, y, image.width as usize);
                if let Some(p) = tile.pixels.get_mut(dest) {
                    *p = indices[y * image.width as usize + x];
                }
            }
        }
        edits.insert(pal_path, wrap(pal_bytes, pal.write()));
        edits.insert(path.into(), wrap(original, tile.write()));
        (
            tile.render(&pal, None, bank)?,
            ImportInfo {
                width: image.width,
                height: image.height,
                colors: count as u32,
                texture_bytes: (tile.pixels.len() / if tile.depth == 3 { 2 } else { 1 }) as u32,
                palette_bytes: (count * 2) as u32,
            },
        )
    } else {
        let tx = Nsbtx::parse(&plain)?;
        let t = tx
            .textures
            .get(texture)
            .ok_or_else(|| Error::Invalid("texture index out of range".into()))?;
        if image.width != t.width || image.height != t.height {
            return Err(Error::Invalid(format!(
                "PNG must be {} × {} pixels",
                t.width, t.height
            )));
        }
        let count = match format {
            1 => 32,
            2 => 4,
            3 => 16,
            4 => 256,
            6 => 8,
            7 => 2,
            _ => return Err(Error::Invalid("choose a supported texture format".into())),
        };
        let (colors, indices) = quantize(&image, count, dither, [2, 3, 4].contains(&format));
        let value = texture::encode(
            &image,
            format,
            &colors,
            &indices,
            [2, 3, 4].contains(&format),
        )?;
        let render = texture::decode(&value, &colors)?;
        let b = tx.replace(
            texture,
            bank,
            &value,
            (format != 7).then_some(colors.as_slice()),
        )?;
        edits.insert(path.into(), wrap(original, b));
        (
            render,
            ImportInfo {
                width: image.width,
                height: image.height,
                colors: if format == 7 { 0 } else { count as u32 },
                texture_bytes: value.data.len() as u32,
                palette_bytes: if format == 7 { 0 } else { count as u32 * 2 },
            },
        )
    };
    Ok(ImportDraft {
        edits,
        image: render,
        info,
    })
}
