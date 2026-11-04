//! Replace an individual OAM cell using its existing palette, preserving adjacent art.
use crate::{project::Project, session::RomSession, Error, Result};
use nitro_formats::{
    cell::Ncer,
    graphics::{Image, Ncgr, Nclr},
};
use std::collections::BTreeMap;
fn shape(a: u16, b: u16) -> Result<(usize, usize)> {
    let sizes = [
        [(8, 8), (16, 16), (32, 32), (64, 64)],
        [(16, 8), (32, 8), (32, 16), (64, 32)],
        [(8, 16), (8, 32), (16, 32), (32, 64)],
    ];
    sizes
        .get((a >> 14) as usize)
        .map(|s| s[(b >> 14) as usize])
        .ok_or_else(|| Error::Invalid("invalid OAM shape".into()))
}
pub fn replace(
    tiles: &mut Ncgr,
    palette: &Nclr,
    bank: &Ncer,
    cell: usize,
    image: &Image,
) -> Result<()> {
    let cell = bank
        .cells
        .get(cell)
        .ok_or_else(|| Error::Invalid("choose a cell from the sprite bank".into()))?;
    let mut lo = [i32::MAX; 2];
    let mut hi = [i32::MIN; 2];
    for &[a, b, _] in &cell.objects {
        if a & 0x300 == 0x200 {
            continue;
        }
        if a & 0x100 != 0 {
            return Err(Error::Invalid(
                "affine sprite import requires baking its transform first".into(),
            ));
        }
        let (w, h) = shape(a, b)?;
        let x = (((b & 511) as i16) << 7 >> 7) as i32;
        let y = a as u8 as i8 as i32;
        lo[0] = lo[0].min(x);
        lo[1] = lo[1].min(y);
        hi[0] = hi[0].max(x + w as i32);
        hi[1] = hi[1].max(y + h as i32);
    }
    if lo[0] >= hi[0]
        || lo[1] >= hi[1]
        || image.width == 0
        || image.height == 0
        || image.rgba.len() != image.width as usize * image.height as usize * 4
    {
        return Err(Error::Invalid("empty sprite or invalid portrait".into()));
    }
    for &[a, b, c] in &cell.objects {
        if a & 0x300 == 0x200 {
            continue;
        }
        let (w, h) = shape(a, b)?;
        let x = (((b & 511) as i16) << 7 >> 7) as i32;
        let y = a as u8 as i8 as i32;
        let eight = a & 0x2000 != 0;
        let start = if eight { 0 } else { (c >> 12) as usize * 16 };
        let count = if eight { 256 } else { 16 };
        let colors = palette
            .colors
            .get(start..start + count)
            .ok_or_else(|| Error::Invalid("sprite palette bank outside allocation".into()))?;
        let base =
            (c & 1023) as usize * (1usize << if bank.mapping == 4 { 0 } else { bank.mapping }) * 64
                / if eight { 2 } else { 1 };
        for py in 0..h {
            for px in 0..w {
                let sx = ((x + px as i32 - lo[0]) as u64 * image.width as u64
                    / (hi[0] - lo[0]) as u64) as usize;
                let sy = ((y + py as i32 - lo[1]) as u64 * image.height as u64
                    / (hi[1] - lo[1]) as u64) as usize;
                let rgba = &image.rgba[(sy * image.width as usize + sx) * 4..][..4];
                let index = if rgba[3] < 128 {
                    0
                } else {
                    let color = nitro_formats::graphics::rgba_to_bgr555(rgba.try_into().unwrap());
                    (1..count)
                        .min_by_key(|&i| {
                            let a = nitro_formats::graphics::bgr555_to_rgba(color);
                            let b = nitro_formats::graphics::bgr555_to_rgba(colors[i]);
                            (0..3)
                                .map(|k| (a[k] as i32 - b[k] as i32).pow(2))
                                .sum::<i32>()
                        })
                        .unwrap_or(1) as u8
                };
                let sx = if b & 0x1000 != 0 { w - 1 - px } else { px };
                let sy = if b & 0x2000 != 0 { h - 1 - py } else { py };
                let tile = (sy / 8)
                    * if bank.mapping == 4 {
                        if eight {
                            16
                        } else {
                            32
                        }
                    } else {
                        w / 8
                    }
                    + sx / 8;
                let dest = tiles
                    .pixels
                    .get_mut(base + tile * 64 + (sy % 8) * 8 + sx % 8)
                    .ok_or_else(|| Error::Invalid("sprite tiles outside atlas".into()))?;
                *dest = index;
            }
        }
    }
    Ok(())
}
pub fn edit(
    p: &mut Project,
    s: &RomSession,
    tiles: &str,
    palette: &str,
    cells: &str,
    index: u32,
    image: &Image,
) -> Result<()> {
    let mut tile = Ncgr::parse(&crate::preview::plain(p.bytes(s, tiles)?)?)?;
    let pal = Nclr::parse(&crate::preview::plain(p.bytes(s, palette)?)?)?;
    let bank = Ncer::parse(&crate::preview::plain(p.bytes(s, cells)?)?)?;
    replace(&mut tile, &pal, &bank, index as usize, image)?;
    p.edit_many(
        BTreeMap::from([(tiles.into(), tile.write())]),
        "Replace sprite icon",
        s,
    )
}
