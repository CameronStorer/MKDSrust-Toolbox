//! Touch-screen course map for generated tracks.
//!
//! The race's bottom-screen overview is `Map2D/global.NCGR` + `global.NCLR` + `global1.NSCR` (4bpp, a 256x192
//! view of a 256x256 screen): a grey road with a white outline, as in Mario Kart Toolbox's global map generator
//! (MIT, HaroohiePals), whose style and road-type list this follows. The game places kart icons with its own
//! per-course world-to-map transform, so we measure that transform from the slot's retail map and retail road
//! collision (read from the user's ROM in place) and draw the new road with it.

use crate::{session::RomSession, Error, Result};
use nitro_formats::{
    graphics::{Ncgr, Nscr},
    kcl::Kcl,
};

/// KCL types drawn as road (Mario Kart Toolbox's list): road, slippery road (1 and 2), boost pad, jump pad, road
/// without drivers, water fall, minimum speed boost, loop and special road.
const ROAD: [u16; 10] = [0, 1, 6, 7, 12, 13, 17, 18, 19, 20];
const W: usize = 256;
const VIEW_H: usize = 192;

/// World (x, z) to map pixel: optional axis swap and flips, then uniform scale and offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapTransform {
    pub swap: bool,
    pub flip: [bool; 2],
    pub scale: f32,
    pub offset: [f32; 2],
}
impl MapTransform {
    pub fn apply(&self, x: f32, z: f32) -> [f32; 2] {
        let (u, v) = if self.swap { (z, x) } else { (x, z) };
        let u = if self.flip[0] { -u } else { u };
        let v = if self.flip[1] { -v } else { v };
        [
            u * self.scale + self.offset[0],
            v * self.scale + self.offset[1],
        ]
    }
    /// Fits the world bounds `lo..hi` (x, z) into the pixel box `px0..px1`, keeping the aspect ratio.
    fn fit(
        swap: bool,
        flip: [bool; 2],
        lo: [f32; 2],
        hi: [f32; 2],
        px0: [f32; 2],
        px1: [f32; 2],
    ) -> Self {
        let mut t = Self {
            swap,
            flip,
            scale: 1.,
            offset: [0.; 2],
        };
        let corners = [t.apply(lo[0], lo[1]), t.apply(hi[0], hi[1])];
        let (u0, u1) = (
            corners[0][0].min(corners[1][0]),
            corners[0][0].max(corners[1][0]),
        );
        let (v0, v1) = (
            corners[0][1].min(corners[1][1]),
            corners[0][1].max(corners[1][1]),
        );
        t.scale =
            ((px1[0] - px0[0]) / (u1 - u0).max(1e-3)).min((px1[1] - px0[1]) / (v1 - v0).max(1e-3));
        t.offset = [
            (px0[0] + px1[0]) * 0.5 - (u0 + u1) * 0.5 * t.scale,
            (px0[1] + px1[1]) * 0.5 - (v0 + v1) * 0.5 * t.scale,
        ];
        t
    }
}

/// Indexed 256x256 image of a screen (tile ids, palette bank and flips honoured, 4bpp tiles).
fn decode(tiles: &Ncgr, screen: &Nscr) -> Vec<u8> {
    let mut out = vec![0u8; W * W];
    for (i, e) in screen.entries.iter().enumerate().take(1024) {
        let (tx, ty) = (i % 32, i / 32);
        let id = (*e & 0x3ff) as usize;
        let (hf, vf) = (e & 0x400 != 0, e & 0x800 != 0);
        for p in 0..64 {
            let (x, y) = (p % 8, p / 8);
            let (sx, sy) = (if hf { 7 - x } else { x }, if vf { 7 - y } else { y });
            out[(ty * 8 + y) * W + tx * 8 + x] = tiles
                .pixels
                .get(id * 64 + sy * 8 + sx)
                .copied()
                .unwrap_or(0);
        }
    }
    out
}

fn fill_triangle(img: &mut [u8], tri: [[f32; 2]; 3], value: u8) {
    let lo = |k: usize| {
        tri.iter()
            .map(|p| p[k])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as usize
    };
    let hi = |k: usize, max: usize| {
        (tri.iter()
            .map(|p| p[k])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.) as usize)
            .min(max - 1)
    };
    for y in lo(1)..=hi(1, W) {
        for x in lo(0)..=hi(0, W) {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let s: Vec<f32> = (0..3)
                .map(|k| {
                    let (a, b) = (tri[k], tri[(k + 1) % 3]);
                    (b[0] - a[0]) * (py - a[1]) - (b[1] - a[1]) * (px - a[0])
                })
                .collect();
            if s.iter().all(|v| *v >= 0.) || s.iter().all(|v| *v <= 0.) {
                img[y * W + x] = value;
            }
        }
    }
}

/// Measures the retail slot's map transform. Returns it with its overlap score (intersection over union of the
/// drawn retail road and the retail map's road pixels), or None when the retail map can't be read.
pub fn measure(kcl: &Kcl, tiles: &Ncgr, screen: &Nscr) -> Option<(MapTransform, f32)> {
    let img = decode(tiles, screen);
    let mut counts = [0usize; 256];
    for v in &img[..W * VIEW_H] {
        counts[*v as usize] += 1;
    }
    // Road fill: the most common non-backdrop colour.
    let fill = (1..256)
        .max_by_key(|i| counts[*i])
        .filter(|i| counts[*i] > 50)? as u8;
    let mask: Vec<bool> = img[..W * VIEW_H].iter().map(|v| *v == fill).collect();
    let (mut x0, mut x1, mut y0, mut y1) = (W, 0, VIEW_H, 0);
    for (i, m) in mask.iter().enumerate() {
        if *m {
            let (x, y) = (i % W, i / W);
            (x0, x1, y0, y1) = (x0.min(x), x1.max(x + 1), y0.min(y), y1.max(y + 1));
        }
    }
    let road: Vec<[[f32; 3]; 3]> = kcl
        .triangles()
        .ok()?
        .into_iter()
        .filter(|t| ROAD.contains(&mkds_data::collision_type(t.attribute)))
        .map(|t| t.positions)
        .collect();
    if road.is_empty() || x1 <= x0 || y1 <= y0 {
        return None;
    }
    let lo = [0, 2].map(|k| {
        road.iter()
            .flatten()
            .map(|p| p[k])
            .fold(f32::INFINITY, f32::min)
    });
    let hi = [0, 2].map(|k| {
        road.iter()
            .flatten()
            .map(|p| p[k])
            .fold(f32::NEG_INFINITY, f32::max)
    });
    let mut best: Option<(MapTransform, f32)> = None;
    for swap in [false, true] {
        for flip in [[false, false], [true, false], [false, true], [true, true]] {
            let t = MapTransform::fit(
                swap,
                flip,
                lo,
                hi,
                [x0 as f32, y0 as f32],
                [x1 as f32, y1 as f32],
            );
            let mut drawn = vec![0u8; W * W];
            for tri in &road {
                fill_triangle(&mut drawn, tri.map(|p| t.apply(p[0], p[2])), 1);
            }
            let (mut both, mut either) = (0usize, 0usize);
            for (i, m) in mask.iter().enumerate() {
                let d = drawn[i] != 0;
                both += usize::from(d && *m);
                either += usize::from(d || *m);
            }
            let score = both as f32 / either.max(1) as f32;
            if best.is_none_or(|b| score > b.1) {
                best = Some((t, score));
            }
        }
    }
    best
}

/// Draws a closed racing line of `width` world units as a grey road with a white outline.
pub fn draw(line: &[[f32; 3]], width: f32, t: &MapTransform, fill: u8, outline: u8) -> Vec<u8> {
    let mut img = vec![0u8; W * W];
    // Retail roads are about 8 px wide on this map; keep thin roads readable.
    let r = (width * 0.5 * t.scale).max(4.);
    let pts: Vec<[f32; 2]> = line.iter().map(|p| t.apply(p[0], p[2])).collect();
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (nx, ny) = (-dy / len * r, dx / len * r);
        let quad = [
            [a[0] + nx, a[1] + ny],
            [b[0] + nx, b[1] + ny],
            [b[0] - nx, b[1] - ny],
            [a[0] - nx, a[1] - ny],
        ];
        fill_triangle(&mut img, [quad[0], quad[1], quad[2]], fill);
        fill_triangle(&mut img, [quad[0], quad[2], quad[3]], fill);
        // Round joints.
        let steps = 12;
        for k in 0..steps {
            let ang = |j: usize| j as f32 / steps as f32 * std::f32::consts::TAU;
            let p0 = [a[0] + r * ang(k).cos(), a[1] + r * ang(k).sin()];
            let p1 = [a[0] + r * ang(k + 1).cos(), a[1] + r * ang(k + 1).sin()];
            fill_triangle(&mut img, [a, p0, p1], fill);
        }
    }
    // Outline: fill pixels touching the outside (Mario Kart Toolbox's ApplyOutline).
    let snapshot = img.clone();
    for y in 0..VIEW_H {
        for x in 0..W {
            if snapshot[y * W + x] != fill {
                continue;
            }
            let outside = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|(dx, dy)| {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    nx < 0
                        || ny < 0
                        || nx >= W as i32
                        || ny >= VIEW_H as i32
                        || snapshot[ny as usize * W + nx as usize] == 0
                });
            if outside {
                img[y * W + x] = outline;
            }
        }
    }
    for v in &mut img[W * VIEW_H..] {
        *v = 0;
    }
    img
}

/// Encodes an indexed 256x256 image as deduplicated 4bpp tiles (32 per row) and a 256x256 screen.
pub fn encode(img: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut tiles: Vec<Vec<u8>> = vec![vec![0; 64]];
    let mut entries = Vec::with_capacity(1024);
    for i in 0..1024 {
        let (tx, ty) = (i % 32, i / 32);
        let tile: Vec<u8> = (0..64)
            .map(|p| img[(ty * 8 + p / 8) * W + tx * 8 + p % 8] & 15)
            .collect();
        let id = match tiles.iter().position(|t| *t == tile) {
            Some(id) => id,
            None => {
                tiles.push(tile);
                tiles.len() - 1
            }
        };
        if id > 0x3ff {
            return Err(Error::Invalid(
                "course map has too many distinct tiles".into(),
            ));
        }
        entries.push(id as u16);
    }
    while !tiles.len().is_multiple_of(32) {
        tiles.push(vec![0; 64]);
    }
    let rows = (tiles.len() / 32) as u16;
    Ok((
        Ncgr::new(32, rows, 3, false, tiles.concat())?.write(),
        Nscr::new(256, 256, &entries)?.write(),
    ))
}

/// Result of [`global_map`]: files to write and a note for the wizard report.
pub struct CourseMap {
    pub files: Vec<(String, Vec<u8>)>,
    pub note: String,
}

/// Builds the overview map for a generated track in the course slot `root` (`…/<stem>.carc`).
pub fn global_map(
    s: &RomSession,
    root: &str,
    line: &[[f32; 3]],
    width: f32,
) -> Result<Option<CourseMap>> {
    let tex_root = root.replace(".carc", "Tex.carc");
    let read = |p: &str| -> Result<Vec<u8>> { crate::preview::plain(s.file(p)?) };
    let (Ok(tiles), Ok(screen)) = (
        read(&format!("{tex_root}/Map2D/global.NCGR")),
        read(&format!("{tex_root}/Map2D/global1.NSCR")),
    ) else {
        return Ok(None);
    };
    let tiles = Ncgr::parse(&tiles)?;
    let screen = Nscr::parse(&screen)?;
    let (fill, outline) = colours(&decode(&tiles, &screen));
    let measured = read(&format!("{root}/course_collision.kcl"))
        .ok()
        .and_then(|b| Kcl::parse(&b).ok())
        .and_then(|k| measure(&k, &tiles, &screen));
    let (t, note) = match measured {
        Some((t, score)) if score >= 0.5 => (
            t,
            format!(
                "Course map drawn with the slot's own map transform (match {:.0}%)",
                score * 100.
            ),
        ),
        _ => {
            let lo =
                [0, 2].map(|k| line.iter().map(|p| p[k]).fold(f32::INFINITY, f32::min) - width);
            let hi =
                [0, 2].map(|k| line.iter().map(|p| p[k]).fold(f32::NEG_INFINITY, f32::max) + width);
            (
                MapTransform::fit(false, [false; 2], [lo[0], lo[1]], [hi[0], hi[1]], [124., 32.], [244., 180.]),
                "Course map drawn without the slot's transform (its retail map could not be matched): kart icons \
                 may be offset; check the touch screen in your emulator"
                    .to_string(),
            )
        }
    };
    let img = draw(line, width, &t, fill, outline);
    let (tiles, screen) = encode(&img)?;
    Ok(Some(CourseMap {
        files: vec![
            (format!("{tex_root}/Map2D/global.NCGR"), tiles),
            (format!("{tex_root}/Map2D/global1.NSCR"), screen),
        ],
        note,
    }))
}

/// Road fill and outline colour indices of a retail map (defaults: 2 grey, 3 white).
fn colours(img: &[u8]) -> (u8, u8) {
    let mut counts = [0usize; 16];
    for v in &img[..W * VIEW_H] {
        counts[(*v & 15) as usize] += 1;
    }
    let fill = (1..16)
        .max_by_key(|i| counts[*i])
        .filter(|i| counts[*i] > 50)
        .unwrap_or(2) as u8;
    let mut edge = [0usize; 16];
    for y in 1..VIEW_H - 1 {
        for x in 1..W - 1 {
            let v = img[y * W + x] & 15;
            if v != 0
                && v != fill
                && [img[y * W + x - 1], img[y * W + x + 1]]
                    .iter()
                    .any(|n| n & 15 == fill)
            {
                edge[v as usize] += 1;
            }
        }
    }
    let outline = (1..16)
        .filter(|i| *i != fill as usize)
        .max_by_key(|i| edge[*i])
        .filter(|i| edge[*i] > 0);
    (fill, outline.unwrap_or(3) as u8)
}

/// Generated files: (ROM path, bytes).
pub type Files = Vec<(String, Vec<u8>)>;

/// Zoomed touch-screen map (the race's default view). `local2.NSCR` covers the slot's rectangle-table-2 square
/// (Figure-8: 6000 world units on 512x512, no rotation; 88% road overlap measured). `local3.NSCR`, when the slot has
/// one, covers the neighbouring square on the side where the retail road continues. Both share `local.NCGR` (8bpp,
/// at most 1024 tiles) and `local.NCLR`.
pub fn local_maps(
    s: &RomSession,
    root: &str,
    rect: [f32; 4],
    model: &nitro_formats::model::Model,
    textures: Option<&nitro_formats::texture::Nsbtx>,
    line: &[[f32; 3]],
    width: f32,
) -> Result<Option<Files>> {
    let tex_root = root.replace(".carc", "Tex.carc");
    let read = |p: &str| -> Result<Vec<u8>> { crate::preview::plain(s.file(p)?) };
    let Ok(first) = read(&format!("{tex_root}/Map2D/local2.NSCR")) else {
        return Ok(None);
    };
    let first = Nscr::parse(&first)?;
    let (sw, sh) = (first.width as usize, first.height as usize);
    if sw < 8 || sh < 8 || rect[2] <= rect[0] {
        return Ok(None);
    }
    let scale = sw as f32 / (rect[2] - rect[0]);
    let span = [sw as f32 / scale, sh as f32 / scale];
    let mut origins = vec![[rect[0], rect[1]]];
    if s.file(&format!("{tex_root}/Map2D/local3.NSCR")).is_ok() {
        // The side where most retail road lies.
        let road: Vec<[f32; 3]> = read(&format!("{root}/course_collision.kcl"))
            .ok()
            .and_then(|b| Kcl::parse(&b).ok())
            .and_then(|k| k.triangles().ok())
            .map(|t| {
                t.into_iter()
                    .filter(|t| ROAD.contains(&mkds_data::collision_type(t.attribute)))
                    .flat_map(|t| t.positions)
                    .collect()
            })
            .unwrap_or_default();
        let inside = |o: [f32; 2], p: &[f32; 3]| {
            p[0] >= o[0] && p[0] < o[0] + span[0] && p[2] >= o[1] && p[2] < o[1] + span[1]
        };
        let best = [[span[0], 0.], [-span[0], 0.], [0., span[1]], [0., -span[1]]]
            .iter()
            .map(|d| [rect[0] + d[0], rect[1] + d[1]])
            .max_by_key(|o| road.iter().filter(|p| inside(*o, p)).count())
            .unwrap_or([rect[0] + span[0], rect[1]]);
        origins.push(best);
    }
    // Flat material colours: each material's average texture colour times its diffuse colour.
    let colours: Vec<[f32; 3]> = model
        .materials
        .iter()
        .map(|m| {
            let mut c = [m.diffuse[0], m.diffuse[1], m.diffuse[2]];
            let texture = textures.zip(m.texture.as_ref()).and_then(|(bank, name)| {
                let t = bank.textures.iter().position(|t| &t.name == name)?;
                let p = m
                    .palette
                    .as_ref()
                    .and_then(|n| bank.palettes.iter().position(|p| &p.name == n))
                    .unwrap_or(0);
                bank.decode(t, p).ok()
            });
            if let Some(img) = texture {
                let (mut sum, mut n) = ([0f32; 3], 0f32);
                for px in img.rgba.chunks_exact(4).filter(|px| px[3] > 0) {
                    for k in 0..3 {
                        sum[k] += px[k] as f32 / 255.;
                    }
                    n += 1.;
                }
                if n > 0. {
                    c = [0, 1, 2].map(|k| c[k] * sum[k] / n);
                }
            }
            c
        })
        .collect();
    let (iw, ih) = (sw * origins.len(), sh);
    let mut rgb = vec![[0.30f32, 0.55, 0.25]; iw * ih];
    let mut depth = vec![f32::NEG_INFINITY; iw * ih];
    for (k, o) in origins.iter().enumerate() {
        let (x0, x1) = (k * sw, (k + 1) * sw);
        let px = |p: [f32; 3]| [(p[0] - o[0]) * scale + x0 as f32, (p[2] - o[1]) * scale];
        for mesh in &model.meshes {
            let colour = colours.get(mesh.material).copied().unwrap_or([0.6; 3]);
            for ids in mesh.indices.chunks_exact(3) {
                let v = [0, 1, 2].map(|i| mesh.positions[ids[i] as usize]);
                let height = (v[0][1] + v[1][1] + v[2][1]) / 3.;
                raster(&v.map(px), iw, ih, x0, x1, |i| {
                    if height >= depth[i] {
                        depth[i] = height;
                        rgb[i] = colour;
                    }
                });
            }
        }
        // The racing line as road with a light edge, as on retail maps.
        let pts: Vec<[f32; 2]> = line.iter().map(|p| px(*p)).collect();
        let core = (width * 0.5 * scale).max(3.);
        for (r, c) in [(core + 1.5, [0.95f32; 3]), (core, [0.42; 3])] {
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                let (nx, ny) = (-dy / len * r, dx / len * r);
                let q = [
                    [a[0] + nx, a[1] + ny],
                    [b[0] + nx, b[1] + ny],
                    [b[0] - nx, b[1] - ny],
                    [a[0] - nx, a[1] - ny],
                ];
                for tri in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
                    raster(&tri, iw, ih, x0, x1, |i| rgb[i] = c);
                }
                for j in 0..16 {
                    let at = |j: usize| {
                        let t = j as f32 / 16. * std::f32::consts::TAU;
                        [a[0] + r * t.cos(), a[1] + r * t.sin()]
                    };
                    raster(&[a, at(j), at(j + 1)], iw, ih, x0, x1, |i| rgb[i] = c);
                }
            }
        }
    }
    let image = nitro_formats::graphics::Image {
        width: iw as u32,
        height: ih as u32,
        rgba: rgb
            .iter()
            .flat_map(|c| {
                let [r, g, b] = c.map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
                [r, g, b, 255]
            })
            .collect(),
    };
    let (palette, indices) = crate::graphic::quantize(&image, 256, false, false);
    // One shared 8bpp tile set of at most 1024 tiles: rare tiles fall back to their closest kept tile.
    let per_row = sw / 8;
    let per = per_row * (sh / 8);
    let tile_at = |k: usize, t: usize| -> Vec<u8> {
        let (tx, ty) = (t % per_row, t / per_row);
        (0..64)
            .map(|p| indices[(ty * 8 + p / 8) * iw + k * sw + tx * 8 + p % 8])
            .collect()
    };
    let mut counts: std::collections::HashMap<Vec<u8>, usize> = Default::default();
    for k in 0..origins.len() {
        for t in 0..per {
            *counts.entry(tile_at(k, t)).or_default() += 1;
        }
    }
    let mut kept: Vec<(Vec<u8>, usize)> = counts.into_iter().collect();
    kept.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    kept.truncate(1024);
    let kept: Vec<Vec<u8>> = kept.into_iter().map(|k| k.0).collect();
    let lookup: std::collections::HashMap<&Vec<u8>, usize> =
        kept.iter().enumerate().map(|(i, t)| (t, i)).collect();
    let mut files = vec![];
    for (k, layer) in ["local2", "local3"].iter().enumerate().take(origins.len()) {
        let entries: Vec<u16> = (0..per)
            .map(|t| {
                let tile = tile_at(k, t);
                lookup.get(&tile).copied().unwrap_or_else(|| {
                    (0..kept.len())
                        .min_by_key(|i| kept[*i].iter().zip(&tile).filter(|(a, b)| a != b).count())
                        .unwrap_or(0)
                }) as u16
            })
            .collect();
        files.push((
            format!("{tex_root}/Map2D/{layer}.NSCR"),
            Nscr::new(sw as u16, sh as u16, &entries)?.write(),
        ));
    }
    let rows = kept.len().div_ceil(32).max(1);
    let mut data = kept.concat();
    data.resize(rows * 32 * 64, 0);
    files.push((
        format!("{tex_root}/Map2D/local.NCGR"),
        Ncgr::new(32, rows as u16, 4, false, data)?.write(),
    ));
    let mut colours = palette;
    colours.resize(256, 0);
    files.push((
        format!("{tex_root}/Map2D/local.NCLR"),
        nitro_formats::graphics::Nclr::new(&colours, true, false).write(),
    ));
    Ok(Some(files))
}

/// Calls `put` for each pixel centre inside the triangle, within columns `x0..x1` of a `w` x `h` image.
fn raster(
    tri: &[[f32; 2]; 3],
    w: usize,
    h: usize,
    x0: usize,
    x1: usize,
    mut put: impl FnMut(usize),
) {
    let lo = |k: usize| {
        tri.iter()
            .map(|p| p[k])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as usize
    };
    let hi = |k: usize| {
        tri.iter()
            .map(|p| p[k])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.) as usize
    };
    for y in lo(1)..hi(1).min(h) {
        for x in lo(0).max(x0)..hi(0).min(x1).min(w) {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let s = [0, 1, 2].map(|k| {
                let (a, b) = (tri[k], tri[(k + 1) % 3]);
                (b[0] - a[0]) * (py - a[1]) - (b[1] - a[1]) * (px - a[0])
            });
            if s.iter().all(|v| *v >= 0.) || s.iter().all(|v| *v <= 0.) {
                put(y * w + x);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn draws_a_closed_outlined_road_and_encodes_it() {
        let t = MapTransform {
            swap: false,
            flip: [false; 2],
            scale: 0.05,
            offset: [128., 96.],
        };
        let line = [
            [-1000., 0., -1000.],
            [1000., 0., -1000.],
            [1000., 0., 1000.],
            [-1000., 0., 1000.],
        ];
        let img = draw(&line, 200., &t, 2, 3);
        assert_eq!(img[96 * W + 128], 0, "inside of the loop stays empty");
        assert_eq!(img[46 * W + 128], 2, "road centre is filled");
        assert!(img.contains(&3), "road has an outline");
        let (tiles, screen) = encode(&img).unwrap();
        let (tiles, screen) = (Ncgr::parse(&tiles).unwrap(), Nscr::parse(&screen).unwrap());
        assert_eq!(screen.entries.len(), 1024);
        assert_eq!(decode(&tiles, &screen), img);
    }
    #[test]
    fn transform_fit_keeps_aspect_and_centres() {
        let t = MapTransform::fit(
            true,
            [false, true],
            [-100., -50.],
            [100., 50.],
            [0., 0.],
            [100., 100.],
        );
        let a = t.apply(-100., -50.);
        let b = t.apply(100., 50.);
        assert!(
            ((a[1] - b[1]).abs() - 100.).abs() < 1e-3,
            "long axis fills the box"
        );
        assert!(((a[0] - b[0]).abs() - 50.).abs() < 1e-3);
        assert!(((a[0] + b[0]) * 0.5 - 50.).abs() < 1e-3);
    }
}
