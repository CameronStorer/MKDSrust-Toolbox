//! Closed racing-line generation and deterministic top-view maps.
//! NKM field layouts: Mario Kart Toolbox (MIT), already credited in CREDITS.md.
use crate::{
    collision,
    project::Project,
    session::RomSession,
    wizard::{bounds, Report},
    Error, Result,
};
use nitro_formats::{
    graphics::{Ncgr, Nclr, Nscr},
    math,
    model::Nsbmd,
    nkm::Nkm,
};
use serde::Deserialize;
use specta::Type;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize, Type)]
pub struct TrackOptions {
    pub course: u32,
    pub source: String,
    /// Top-down polyline in normalized source bounds, X then Z. The final edge closes automatically.
    pub line: Vec<[f32; 2]>,
    pub width: f32,
    pub laps: u16,
    pub item_rows: u32,
}
fn err(s: &str) -> Error {
    Error::Invalid(s.into())
}
/// Runtime (expanded) file sizes, measured in memory from the owned clean ROM.
/// Keys include subdirectories: a Map2D member must not borrow a model's budget.
pub fn file_budgets(s: &RomSession) -> Result<BTreeMap<String, usize>> {
    if let Some(clean) = &s.clean {
        return file_budgets(&RomSession::from_bytes(clean.clone(), s.path.clone())?);
    }
    let mut limits = BTreeMap::<String, usize>::new();
    for n in &s.nodes {
        if let Some(role) = file_role(&n.path) {
            let bytes = crate::preview::plain(s.file(&n.path)?)?;
            let limit = limits.entry(role).or_default();
            *limit = (*limit).max(bytes.len());
        }
    }
    Ok(limits)
}
pub fn file_role(path: &str) -> Option<String> {
    path.strip_prefix("data/Course/")?
        .split_once(".carc/")
        .map(|(_, role)| role.to_ascii_lowercase())
}
pub fn check_file_size(path: &str, bytes: &[u8], limits: &BTreeMap<String, usize>) -> Result<()> {
    let Some(role) = file_role(path) else {
        return Ok(());
    };
    let limit = limits
        .get(&role)
        .ok_or_else(|| err(&format!("{path}: no retail size budget is available")))?;
    let size = crate::preview::plain(bytes)?.len();
    if size > *limit {
        return Err(err(&format!("{path}: {size} bytes exceeds the largest retail {role} ({limit} bytes); simplify the mesh, reduce textures or route/object counts")));
    }
    Ok(())
}
fn u16at(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn fx(b: &mut [u8], at: usize, v: f32) -> Result<()> {
    crate::tables::write(b, 0, at as u32, &[crate::tables::Value::Fx32(v)])
}
fn vec(b: &mut [u8], at: usize, p: [f32; 3]) -> Result<()> {
    for (k, v) in p.into_iter().enumerate() {
        fx(b, at + k * 4, v)?;
    }
    Ok(())
}
pub fn resample(line: &[[f32; 3]], count: usize) -> Result<Vec<[f32; 3]>> {
    if line.len() < 3
        || !(16..=128).contains(&count)
        || line.iter().flatten().any(|v| !v.is_finite())
    {
        return Err(err("draw a closed line with at least three finite points"));
    }
    let lengths: Vec<f32> = (0..line.len())
        .map(|i| {
            math::dot(
                std::array::from_fn(|k| line[(i + 1) % line.len()][k] - line[i][k]),
                std::array::from_fn(|k| line[(i + 1) % line.len()][k] - line[i][k]),
            )
            .sqrt()
        })
        .collect();
    if lengths.iter().any(|&d| d < 0.01) {
        return Err(err("remove duplicate racing-line points"));
    }
    let total: f32 = lengths.iter().sum();
    let mut result = vec![];
    for n in 0..count {
        let mut d = total * n as f32 / count as f32;
        let mut edge = 0;
        while edge + 1 < lengths.len() && d > lengths[edge] {
            d -= lengths[edge];
            edge += 1;
        }
        let t = d / lengths[edge];
        result.push(std::array::from_fn(|k| {
            line[edge][k] * (1. - t) + line[(edge + 1) % line.len()][k] * t
        }));
    }
    Ok(result)
}
fn yaw(a: [f32; 3], b: [f32; 3]) -> f32 {
    (b[0] - a[0]).atan2(b[2] - a[2]).to_degrees()
}
pub fn generate_nkm(
    course: u16,
    laps: u16,
    line: &[[f32; 3]],
    width: f32,
    item_rows: u32,
) -> Result<Nkm> {
    if laps == 0 || laps > 9 || !width.is_finite() || width <= 0. || item_rows > 32 {
        return Err(err(
            "choose 1..9 laps, positive route width and at most 32 item rows",
        ));
    }
    let line = resample(line, 64)?;
    let count = line.len();
    let mut start = vec![0; 28];
    vec(&mut start, 0, line[0])?;
    fx(&mut start, 16, yaw(line[0], line[1]))?;
    start[24..28].fill(255);
    let mut stag = vec![0; 40];
    u16at(&mut stag, 0, course);
    u16at(&mut stag, 2, laps);
    for k in 0..4 {
        u16at(&mut stag, 24 + k * 2, 0x7fff);
    }
    fx(&mut stag, 32, 10000.)?;
    fx(&mut stag, 36, 20000.)?;
    let mut checkpoints = vec![];
    let mut enemy = vec![];
    let mut items = vec![];
    let mut respawns = vec![];
    let mut objects = vec![];
    for (i, &p) in line.iter().enumerate() {
        let next = line[(i + 1) % count];
        let tangent = math::normalize([next[0] - p[0], 0., next[2] - p[2]]);
        let side = [tangent[2], 0., -tangent[0]];
        let left = [p[0] + side[0] * width / 2., p[2] + side[2] * width / 2.];
        let right = [p[0] - side[0] * width / 2., p[2] - side[2] * width / 2.];
        let mut cp = vec![0; 36];
        for (k, v) in [
            left[0],
            left[1],
            right[0],
            right[1],
            -side[2],
            side[0],
            if i + 1 == count {
                -1.
            } else {
                math::dot(
                    [next[0] - p[0], next[1] - p[1], next[2] - p[2]],
                    [next[0] - p[0], next[1] - p[1], next[2] - p[2]],
                )
                .sqrt()
            },
        ]
        .into_iter()
        .enumerate()
        {
            fx(&mut cp, k * 4, v)?;
        }
        u16at(&mut cp, 28, if i + 1 == count { 0 } else { u16::MAX });
        u16at(&mut cp, 30, if i == 0 { 0 } else { u16::MAX });
        u16at(
            &mut cp,
            32,
            if i == 0 {
                0
            } else if i == count / 2 {
                1
            } else {
                u16::MAX
            },
        );
        cp[34] = (i / 8) as u8;
        checkpoints.push(cp);
        let mut ep = vec![0; 24];
        vec(&mut ep, 0, p)?;
        fx(&mut ep, 12, width * 0.4)?;
        enemy.push(ep);
        let mut ip = vec![0; 20];
        vec(&mut ip, 0, p)?;
        fx(&mut ip, 12, width * 0.35)?;
        ip[16] = 255;
        items.push(ip);
        if i % 8 == 0 {
            let mut r = vec![0; 32];
            let mut rp = p;
            rp[1] += 10.;
            vec(&mut r, 0, rp)?;
            fx(&mut r, 16, yaw(p, next))?;
            u16at(&mut r, 24, i as u16);
            u16at(&mut r, 26, i as u16);
            r[28..32].copy_from_slice(&((i / 8) as u32).to_le_bytes());
            respawns.push(r);
        }
    }
    for row in 0..item_rows {
        let i = ((row + 1) as usize * count / (item_rows + 1) as usize) % count;
        let p = line[i];
        let next = line[(i + 1) % count];
        let side = math::normalize([next[2] - p[2], 0., p[0] - next[0]]);
        for j in -1..=1 {
            let mut b = vec![0; 60];
            vec(
                &mut b,
                0,
                [
                    p[0] + side[0] * j as f32 * width / 4.,
                    p[1] + 12.,
                    p[2] + side[2] * j as f32 * width / 4.,
                ],
            )?;
            vec(&mut b, 24, [1.; 3])?;
            u16at(&mut b, 36, 0x65);
            u16at(&mut b, 38, u16::MAX);
            objects.push(b);
        }
    }
    let mut path = vec![255; 12];
    u16at(&mut path, 0, 0);
    u16at(&mut path, 2, count as u16);
    path[4] = 0;
    path[7] = 0;
    path[10..12].fill(0);
    let mut camera = vec![0; 76];
    vec(
        &mut camera,
        0,
        [line[0][0], line[0][1] + 200., line[0][2] - 400.],
    )?;
    vec(&mut camera, 24, line[0])?;
    vec(&mut camera, 36, line[1])?;
    for at in [48, 54] {
        u16at(&mut camera, at, 45);
        u16at(&mut camera, at + 2, 2896);
        u16at(&mut camera, at + 4, 2896);
    }
    u16at(&mut camera, 64, u16::MAX);
    u16at(&mut camera, 70, 180);
    u16at(&mut camera, 72, u16::MAX);
    Nkm::new(&[
        ("OBJI", objects),
        ("PATH", vec![]),
        ("POIT", vec![]),
        ("STAG", vec![stag]),
        ("KTPS", vec![start.clone()]),
        ("KTPJ", respawns),
        ("KTP2", vec![start]),
        ("KTPC", vec![]),
        ("KTPM", vec![]),
        ("CPOI", checkpoints),
        ("CPAT", vec![path.clone()]),
        ("IPOI", items),
        ("IPAT", vec![path.clone()]),
        ("EPOI", enemy),
        ("EPAT", vec![path]),
        ("AREA", vec![]),
        ("CAME", vec![camera]),
    ])
    .map_err(Into::into)
}
pub fn validate(n: &Nkm) -> Vec<String> {
    let mut errors = vec![];
    for (tag, min) in [
        ("STAG", 1),
        ("KTPS", 1),
        ("KTP2", 1),
        ("KTPJ", 2),
        ("CPOI", 16),
        ("CPAT", 1),
        ("IPOI", 1),
        ("IPAT", 1),
        ("EPOI", 1),
        ("EPAT", 1),
        ("CAME", 1),
    ] {
        if n.section(tag).is_none_or(|s| s.entries.len() < min) {
            errors.push(format!("{tag}: requires at least {min} records"));
        }
    }
    for tag in ["CPAT", "IPAT", "EPAT"] {
        if let Some(s) = n.section(tag) {
            for r in &s.entries {
                if r[4] == 255
                    || r[7] == 255
                    || r[4] as usize >= s.entries.len()
                    || r[7] as usize >= s.entries.len()
                {
                    errors.push(format!("{tag}: route is not closed"));
                }
                let points = n
                    .section(match tag {
                        "CPAT" => "CPOI",
                        "IPAT" => "IPOI",
                        _ => "EPOI",
                    })
                    .map_or(0, |s| s.entries.len());
                let start = u16::from_le_bytes([r[0], r[1]]) as usize;
                let count = u16::from_le_bytes([r[2], r[3]]) as usize;
                if count == 0 || start + count > points {
                    errors.push(format!("{tag}: point range is invalid"));
                }
            }
        }
    }
    errors
}
/// Repairs only missing/undersized required sections and broken route tables.
/// Existing valid points, objects and camera records remain untouched.
pub fn repair(n: &mut Nkm) -> Result<()> {
    let route = n
        .section("EPOI")
        .or_else(|| n.section("IPOI"))
        .filter(|s| s.entries.len() >= 3)
        .ok_or_else(|| err("draw at least three CPU or item route points before repairing"))?;
    let points = route
        .entries
        .iter()
        .map(|b| nitro_formats::nkm::vec3(b, 0))
        .collect::<nitro_formats::Result<Vec<_>>>()?;
    let stag = n.section("STAG").and_then(|s| s.entries.first());
    let course = stag.map_or(20, |b| u16::from_le_bytes([b[0], b[1]]));
    let laps = stag.map_or(3, |b| u16::from_le_bytes([b[2], b[3]]).clamp(1, 9));
    let generated = generate_nkm(course, laps, &points, 150., 0)?;
    for mut section in generated.sections {
        let size = nitro_formats::nkm::stride(&section.kind, n.version).unwrap();
        for row in &mut section.entries {
            row.truncate(size);
        }
        let minimum = match section.kind.as_str() {
            "CPOI" => 16,
            "KTPJ" => 2,
            "STAG" | "KTPS" | "KTP2" | "IPOI" | "EPOI" | "CAME" => 1,
            _ => 0,
        };
        if let Some(old) = n.sections.iter_mut().find(|s| s.kind == section.kind) {
            if old.entries.len() < minimum {
                *old = section;
            }
        } else {
            n.sections.push(section);
        }
    }
    const ORDER: [&str; 19] = [
        "OBJI", "PATH", "POIT", "STAG", "KTPS", "KTPJ", "KTP2", "KTPC", "KTPM", "CPOI", "CPAT",
        "IPOI", "IPAT", "EPOI", "EPAT", "AREA", "CAME", "MEPO", "MEPA",
    ];
    n.sections.sort_by_key(|s| {
        ORDER
            .iter()
            .position(|k| *k == s.kind)
            .unwrap_or(ORDER.len())
    });
    for (path, points) in [("CPAT", "CPOI"), ("IPAT", "IPOI"), ("EPAT", "EPOI")] {
        let count = n.section(points).unwrap().entries.len();
        // Collapse an invalid graph into one closed route; valid graphs survive.
        if validate(n).iter().any(|e| e.starts_with(path)) {
            let mut row = vec![255; 12];
            u16at(&mut row, 0, 0);
            u16at(&mut row, 2, count as u16);
            row[4] = 0;
            row[7] = 0;
            row[10..12].fill(0);
            n.sections
                .iter_mut()
                .find(|s| s.kind == path)
                .unwrap()
                .entries = vec![row];
        }
    }
    Ok(())
}
pub fn move_checkpoint(n: &mut Nkm, index: usize, left: [f32; 2], right: [f32; 2]) -> Result<()> {
    let section = n
        .sections
        .iter_mut()
        .find(|s| s.kind == "CPOI")
        .ok_or_else(|| err("no checkpoints"))?;
    let record = section
        .entries
        .get_mut(index)
        .ok_or_else(|| err("checkpoint index outside section"))?;
    let dx = left[0] - right[0];
    let dz = left[1] - right[1];
    let length = dx.hypot(dz);
    if !length.is_finite() || length < 0.01 {
        return Err(err("checkpoint gate must have a finite nonzero width"));
    }
    for (k, v) in [
        left[0],
        left[1],
        right[0],
        right[1],
        -dz / length,
        dx / length,
    ]
    .iter()
    .enumerate()
    {
        fx(record, k * 4, *v)?;
    }
    for i in 0..section.entries.len() {
        let a = &section.entries[i];
        let b = &section.entries[(i + 1) % section.entries.len()];
        let goto = i16::from_le_bytes(a[28..30].try_into().unwrap());
        let center = |b: &[u8]| -> Result<[f32; 2]> {
            Ok([
                (nitro_formats::nkm::fx(b, 0)? + nitro_formats::nkm::fx(b, 8)?) * 0.5,
                (nitro_formats::nkm::fx(b, 4)? + nitro_formats::nkm::fx(b, 12)?) * 0.5,
            ])
        };
        let ca = center(a)?;
        let cb = center(b)?;
        let distance = if goto == -1 {
            (ca[0] - cb[0]).hypot(ca[1] - cb[1])
        } else {
            -1.
        };
        fx(&mut section.entries[i], 24, distance)?;
    }
    Ok(())
}
/// The two 55x4 s16 world rectangles are read from the user's executable.
pub fn map_rectangle(arm9: &[u8], course: u32, second: bool) -> Result<[f32; 4]> {
    if course >= 55 {
        return Err(err("invalid minimap course id"));
    }
    let at = (if second { 0x0016_8754 } else { 0x0016_890c }) + course as usize * 8;
    let b = arm9
        .get(at..at + 8)
        .ok_or_else(|| err("minimap table outside ARM9"))?;
    let r = std::array::from_fn(|i| i16::from_le_bytes([b[i * 2], b[i * 2 + 1]]) as f32);
    if r[2] <= r[0] || r[3] <= r[1] {
        return Err(err("course minimap rectangle is empty"));
    }
    Ok(r)
}
fn minimap(m: &nitro_formats::model::Model, rect: [f32; 4]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let mut pixels = vec![0u8; 256 * 256];
    let project = |p: [f32; 3]| {
        [
            (p[0] - rect[0]) / (rect[2] - rect[0]) * 255.,
            (p[2] - rect[1]) / (rect[3] - rect[1]) * 255.,
        ]
    };
    for mesh in &m.meshes {
        for tri in mesh.indices.chunks_exact(3) {
            let p = tri
                .iter()
                .map(|&i| project(mesh.positions[i as usize]))
                .collect::<Vec<_>>();
            let lo = std::array::from_fn::<_, 2, _>(|k| {
                p.iter()
                    .map(|p| p[k])
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .clamp(0., 255.) as usize
            });
            let hi = std::array::from_fn::<_, 2, _>(|k| {
                p.iter()
                    .map(|p| p[k])
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .clamp(0., 255.) as usize
            });
            for y in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    let mut signs = [0.; 3];
                    for k in 0..3 {
                        let a = p[k];
                        let b = p[(k + 1) % 3];
                        signs[k] =
                            (b[0] - a[0]) * (y as f32 - a[1]) - (b[1] - a[1]) * (x as f32 - a[0]);
                    }
                    if signs.iter().all(|&v| v >= 0.) || signs.iter().all(|&v| v <= 0.) {
                        pixels[y * 256 + x] = 1;
                    }
                }
            }
        }
    }
    let tiled = (0..1024)
        .flat_map(|tile| {
            (0..64).map(move |i| (tile / 32 * 8 + i / 8) * 256 + tile % 32 * 8 + i % 8)
        })
        .map(|i| pixels[i])
        .collect();
    let mut colors = vec![0; 256];
    colors[1] = 0x7fff;
    Ok((
        Ncgr::new(32, 32, 4, false, tiled)?.write(),
        Nclr::new(&colors, true, false).write(),
        Nscr::new(256, 256, &(0..1024).collect::<Vec<u16>>())?.write(),
    ))
}
pub fn surface_height(m: &nitro_formats::model::Model, x: f32, z: f32) -> Result<f32> {
    let mut height: Option<f32> = None;
    for mesh in &m.meshes {
        for ids in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [
                mesh.positions[ids[0] as usize],
                mesh.positions[ids[1] as usize],
                mesh.positions[ids[2] as usize],
            ];
            let determinant = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
            if determinant.abs() < 1e-6 {
                continue;
            }
            let u = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / determinant;
            let v = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / determinant;
            if u >= -1e-5 && v >= -1e-5 && u + v <= 1.00001 {
                let y = u * a[1] + v * b[1] + (1. - u - v) * c[1];
                height = Some(height.map_or(y, |h| h.max(y)));
            }
        }
    }
    height.ok_or_else(|| {
        err("racing line or item row is outside the map surface; move it onto the road")
    })
}
pub fn place_object(
    p: &mut Project,
    s: &RomSession,
    path: &str,
    donor: Option<&str>,
    index: usize,
    position: [f32; 3],
) -> Result<()> {
    let mut n = Nkm::parse(&crate::preview::plain(p.bytes(s, path)?)?)?;
    let mut edits = BTreeMap::new();
    let mut row = if let Some(donor) = donor {
        let source = Nkm::parse(&crate::preview::plain(p.bytes(s, donor)?)?)?;
        let row = source
            .section("OBJI")
            .and_then(|s| s.entries.get(index))
            .ok_or_else(|| err("choose an object from the donor course"))?
            .clone();
        if u16::from_le_bytes([row[38], row[39]]) != u16::MAX {
            return Err(err("this object uses a route; copy its PATH/POIT in the Track editor before placing it"));
        }
        let from = donor
            .rsplit_once('/')
            .ok_or_else(|| err("invalid donor course"))?
            .0;
        let to = path
            .rsplit_once('/')
            .ok_or_else(|| err("invalid destination course"))?
            .0;
        // The handler-to-file map is incomplete. Copy the full donor MapObj
        // family, including texture/animation companions, instead of guessing.
        for (from, to) in [
            (from.to_string(), to.to_string()),
            (
                from.replace(".carc", "Tex.carc"),
                to.replace(".carc", "Tex.carc"),
            ),
        ] {
            for node in &s.nodes {
                if let Some(suffix) = node.path.strip_prefix(&format!("{from}/MapObj/")) {
                    let dest = format!("{to}/MapObj/{suffix}");
                    let bytes = p.bytes(s, &node.path)?.to_vec();
                    if p.bytes(s, &dest).is_ok_and(|b| b != bytes) {
                        return Err(err("donor object resources conflict with this course; choose another donor"));
                    }
                    edits.insert(dest, bytes);
                }
            }
        }
        row
    } else {
        let mut row = vec![0; 60];
        vec(&mut row, 24, [1.; 3])?;
        u16at(&mut row, 36, 0x65);
        u16at(&mut row, 38, u16::MAX);
        row
    };
    vec(&mut row, 0, position)?;
    n.sections
        .iter_mut()
        .find(|s| s.kind == "OBJI")
        .ok_or_else(|| err("repair the missing object section first"))?
        .entries
        .push(row);
    edits.insert(path.into(), n.write()?);
    let limits = file_budgets(s)?;
    for (path, bytes) in &edits {
        check_file_size(path, bytes, &limits)?;
    }
    p.edit_many(edits, "Place track object and resources", s)
}
/// Height of the lowest point of an imported map.
pub const MAP_FLOOR_Y: f32 = 600.;
pub fn track(p: &mut Project, s: &RomSession, o: &TrackOptions) -> Result<Report> {
    let model = crate::model_import::import(std::path::Path::new(&o.source))?;
    track_from(p, s, o, model)
}
pub fn track_from(
    p: &mut Project,
    s: &RomSession,
    o: &TrackOptions,
    mut source: Nsbmd,
) -> Result<Report> {
    let catalog = crate::tables::catalog(Some(p), s)?;
    let c = catalog
        .courses
        .iter()
        .find(|c| c.id == o.course)
        .ok_or_else(|| err("choose a course slot"))?;
    let arm = p.bytes(s, nitro_formats::executable::ARM9)?;
    let rect = map_rectangle(arm, o.course, false)?;
    let m = source
        .models
        .first_mut()
        .ok_or_else(|| err("source has no model"))?;
    let (lo, hi) = bounds(m)?;
    if hi[0] - lo[0] < 0.01 || hi[2] - lo[2] < 0.01 {
        return Err(err("map needs a nonzero XZ footprint"));
    }
    let scale =
        ((rect[2] - rect[0]) / (hi[0] - lo[0])).min((rect[3] - rect[1]) / (hi[2] - lo[2])) * 0.85;
    let t = [
        (rect[0] + rect[2] - (lo[0] + hi[0]) * scale) * 0.5,
        // Keep the whole collision area above y = 0: with the map's floor at 0 (area minimum below zero) karts
        // fall through it in game (verified in MKDSrust). Retail courses sit well above zero (Figure-8: about 535).
        MAP_FLOOR_Y - lo[1] * scale,
        (rect[1] + rect[3] - (lo[2] + hi[2]) * scale) * 0.5,
    ];
    let matrix = math::trs(t, [0., 0., 0., 1.], [scale; 3]);
    for mesh in &mut m.meshes {
        nitro_formats::model::transform(mesh, matrix)?;
        mesh.joints.clear();
        mesh.weights.clear();
    }
    m.bones = vec![nitro_formats::model::Bone {
        name: "world_root".into(),
        parent: None,
        translation: [0.; 3],
        rotation: [0., 0., 0., 1.],
        scale: [1.; 3],
    }];
    m.matrix_count = 1;
    let line = o
        .line
        .iter()
        .map(|p| {
            if p.iter().any(|v| !v.is_finite() || !(0. ..=1.).contains(v)) {
                return Err(err("line points must be within the map preview"));
            }
            Ok(math::point(
                matrix,
                [
                    lo[0] + p[0] * (hi[0] - lo[0]),
                    lo[1] + 0.01,
                    lo[2] + p[1] * (hi[2] - lo[2]),
                ],
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut nkm = generate_nkm(o.course as u16, o.laps, &line, o.width, o.item_rows)?;
    for (tag, lift) in [
        ("KTPS", 10.),
        ("KTP2", 10.),
        ("KTPJ", 10.),
        ("EPOI", 1.),
        ("IPOI", 1.),
        ("OBJI", 12.),
    ] {
        for row in &mut nkm
            .sections
            .iter_mut()
            .find(|s| s.kind == tag)
            .unwrap()
            .entries
        {
            let p = nitro_formats::nkm::vec3(row, 0)?;
            fx(row, 4, surface_height(m, p[0], p[2])? + lift)?;
        }
    }
    let collision = collision::generate(m, &collision::AttributeTable::default())?;
    let polygons = m.meshes.iter().map(|m| m.indices.len() as u32 / 3).sum();
    if polygons > 2000 {
        return Err(err(
            "track exceeds 2000 triangles; decimate it in Model first",
        ));
    }
    if let Some(tx) = source.textures.take() {
        let mut textures = vec![];
        let mut palettes = vec![];
        for (i, texture) in tx.textures.iter().enumerate() {
            let palette = source.models[0]
                .materials
                .iter()
                .find(|m| m.texture.as_deref() == Some(&texture.name))
                .and_then(|m| m.palette.as_ref())
                .and_then(|name| tx.palettes.iter().position(|p| &p.name == name))
                .unwrap_or(i.min(tx.palettes.len().saturating_sub(1)));
            let (converted, mut colors) =
                crate::model::convert_texture(&tx.decode(i, palette)?, &texture.name, 3, 128)?;
            colors.name = format!("map_pal_{i}");
            for material in &mut source.models[0].materials {
                if material.texture.as_deref() == Some(&texture.name) {
                    material.palette = Some(colors.name.clone());
                    material.width = converted.width as u16;
                    material.height = converted.height as u16;
                }
            }
            textures.push(converted);
            palettes.push(colors);
        }
        source.textures = Some(nitro_formats::texture::Nsbtx::new(textures, palettes)?);
    }
    let mut edits = BTreeMap::new();
    // Update D variants as well, so every race mode sees the replacement.
    for variant in if c.variants { vec!["", "D"] } else { vec![""] } {
        let suffix = format!("/{}{}.carc", c.stem, variant).to_ascii_lowercase();
        let root = s
            .rom
            .files
            .keys()
            .find(|p| p.to_ascii_lowercase().ends_with(&suffix))
            .ok_or_else(|| err("course archive absent"))?;
        let tex_root = root.replace(".carc", "Tex.carc");
        let texture_path = format!("{tex_root}/course_model.nsbtx");
        let mut course_model = source.clone();
        if s.file(&texture_path).is_ok() {
            course_model.textures = None;
        }
        edits.insert(format!("{root}/course_model.nsbmd"), course_model.write()?);
        edits.insert(format!("{root}/course_collision.kcl"), collision.write());
        edits.insert(format!("{root}/course_map.nkm"), nkm.write()?);
        if let Some(tx) = &source.textures {
            let path = texture_path;
            if s.file(&path).is_ok() {
                edits.insert(path, tx.write());
            }
        }
        let (tiles, palette, screen) = minimap(&source.models[0], rect)?;
        edits.insert(format!("{tex_root}/Map2D/local.NCGR"), tiles);
        edits.insert(format!("{tex_root}/Map2D/local.NCLR"), palette);
        edits.insert(format!("{tex_root}/Map2D/local2.NSCR"), screen);
        let second = format!("{tex_root}/Map2D/local3.NSCR");
        if s.file(&second).is_ok() {
            edits.insert(second, Nscr::new(256, 256, &vec![0; 1024])?.write());
        }
    }
    let limits = file_budgets(s)?;
    for (path, bytes) in &edits {
        check_file_size(path, bytes, &limits)?;
    }
    let mut warnings = vec![
        "Single camera and no area triggers: verify the intro and goal views in your emulator"
            .into(),
        format!(
            "Checked {} generated track files against retail size limits; KCL {} / {} bytes",
            edits.len(),
            collision.write().len(),
            nitro_formats::kcl::MAX_BYTES
        ),
    ];
    warnings.extend(collision.budget_warning());
    let report = Report {
        files: edits.keys().cloned().collect(),
        polygons,
        warnings,
    };
    p.edit_many(edits, "New track", s)?;
    Ok(report)
}
