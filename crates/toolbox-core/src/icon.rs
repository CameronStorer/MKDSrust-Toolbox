//! Replace an individual OAM cell using its existing palette, preserving adjacent art.
use crate::{project::Project, session::RomSession, Error, Result};
use nitro_formats::{
    cell::Ncer,
    graphics::{Image, Ncgr, Nclr},
};
use std::collections::BTreeMap;
/// Deterministic orthographic portrait of the authored model. Transparent pixels
/// outside the silhouette allow import into the existing OAM cell palette.
pub fn portrait(model: &nitro_formats::model::Model) -> Result<Image> {
    let (lo, hi) = crate::wizard::bounds(model)?;
    let scale = 112. / (hi[0] - lo[0]).max(hi[1] - lo[1]).max(0.001);
    let mut image = Image {
        width: 128,
        height: 128,
        rgba: vec![0; 128 * 128 * 4],
    };
    let mut depth = vec![f32::NEG_INFINITY; 128 * 128];
    let project = |p: [f32; 3]| {
        [
            64. + (p[0] - (lo[0] + hi[0]) / 2.) * scale,
            64. - (p[1] - (lo[1] + hi[1]) / 2.) * scale,
        ]
    };
    for mesh in &model.meshes {
        nitro_formats::model::validate_mesh(mesh)?;
        let color = model
            .materials
            .get(mesh.material)
            .map_or([0.6, 0.7, 0.9, 1.], |m| m.diffuse);
        for ids in mesh.indices.chunks_exact(3) {
            let p = ids
                .iter()
                .map(|&i| mesh.positions[i as usize])
                .collect::<Vec<_>>();
            let q = p.iter().copied().map(project).collect::<Vec<_>>();
            let edge = |a: [f32; 2], b: [f32; 2], p: [f32; 2]| {
                (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
            };
            let area = edge(q[0], q[1], q[2]);
            if area.abs() < 1e-6 {
                continue;
            }
            let normal = nitro_formats::math::normalize(nitro_formats::math::cross(
                std::array::from_fn(|k| p[1][k] - p[0][k]),
                std::array::from_fn(|k| p[2][k] - p[0][k]),
            ));
            let light =
                (0.45 + 0.55 * nitro_formats::math::dot(normal, [0.3, 0.7, 0.6]).abs()).min(1.);
            let min: [usize; 2] = std::array::from_fn(|k| {
                q.iter()
                    .map(|v| v[k])
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .clamp(0., 127.) as usize
            });
            let max: [usize; 2] = std::array::from_fn(|k| {
                q.iter()
                    .map(|v| v[k])
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .clamp(0., 127.) as usize
            });
            for y in min[1]..=max[1] {
                for x in min[0]..=max[0] {
                    let pixel = [x as f32 + 0.5, y as f32 + 0.5];
                    let w = [
                        edge(q[1], q[2], pixel) / area,
                        edge(q[2], q[0], pixel) / area,
                        edge(q[0], q[1], pixel) / area,
                    ];
                    if w.iter().any(|&v| v < 0.) {
                        continue;
                    }
                    let z: f32 = (0..3).map(|i| p[i][2] * w[i]).sum();
                    let index = y * 128 + x;
                    if z >= depth[index] {
                        depth[index] = z;
                        for (k, c) in color.iter().take(3).enumerate() {
                            image.rgba[index * 4 + k] = (c * light * 255.).clamp(0., 255.) as u8;
                        }
                        image.rgba[index * 4 + 3] = 255;
                    }
                }
            }
        }
    }
    Ok(image)
}
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
