//! RAM-addressed, typed editing of the loaded executable, never the compressed stream.
use crate::{project::Project, session::RomSession, Error, Result};
use nitro_formats::executable::ARM9;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    I32(i32),
    Fx32(f32),
}
fn range(bytes: &[u8], base: u32, address: u32, size: usize) -> Result<std::ops::Range<usize>> {
    let at = address
        .checked_sub(base)
        .ok_or_else(|| Error::Invalid("address precedes binary RAM base".into()))?
        as usize;
    let end = at
        .checked_add(size)
        .filter(|&end| end <= bytes.len())
        .ok_or_else(|| Error::Invalid("table extends outside loaded image".into()))?;
    Ok(at..end)
}
pub fn word(bytes: &[u8], address: u32) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes[range(bytes, mkds_data::ARM9_BASE, address, 4)?]
            .try_into()
            .unwrap(),
    ))
}
pub fn write(bytes: &mut [u8], base: u32, address: u32, values: &[Value]) -> Result<()> {
    let mut output = vec![];
    for value in values {
        match value {
            Value::U8(v) => output.push(*v),
            Value::U16(v) => output.extend(v.to_le_bytes()),
            Value::U32(v) => output.extend(v.to_le_bytes()),
            Value::I32(v) => output.extend(v.to_le_bytes()),
            Value::Fx32(v) => {
                let f = (*v as f64 * 4096.).round();
                if !f.is_finite() || f < i32::MIN as f64 || f > i32::MAX as f64 {
                    return Err(Error::Invalid("value exceeds fx20.12".into()));
                }
                output.extend((f as i32).to_le_bytes());
            }
        }
    }
    let r = range(bytes, base, address, output.len())?;
    bytes[r].copy_from_slice(&output);
    Ok(())
}
pub fn edit(
    p: &mut Project,
    s: &RomSession,
    overlay: Option<u32>,
    address: u32,
    values: &[Value],
) -> Result<()> {
    let (path, base) = if let Some(id) = overlay {
        let o = s
            .rom
            .arm9_overlays
            .iter()
            .find(|o| o.id == id)
            .ok_or_else(|| Error::Invalid("unknown overlay".into()))?;
        (nitro_formats::executable::overlay_path(id), o.ram_address)
    } else {
        (ARM9.into(), mkds_data::ARM9_BASE)
    };
    let mut bytes = p.bytes(s, &path)?.to_vec();
    write(&mut bytes, base, address, values)?;
    p.edit(&path, bytes, "Edit executable table", s)
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct CourseSlot {
    pub id: u32,
    pub stem: String,
    pub variants: bool,
    pub music_record: Option<u32>,
    pub sequence: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Type, Default)]
pub struct Catalog {
    pub courses: Vec<CourseSlot>,
    pub cups: Vec<Vec<u32>>,
}
pub fn catalog(p: Option<&Project>, s: &RomSession) -> Result<Catalog> {
    let bytes = match p {
        Some(p) => p.bytes(s, ARM9)?,
        None => s.file(ARM9)?,
    };
    let mut courses = vec![];
    for id in 1..55 {
        let at = mkds_data::COURSE_TABLE + id * 8;
        let ptr = word(bytes, at)?;
        let r = range(bytes, mkds_data::ARM9_BASE, ptr, 1)?;
        let tail = &bytes[r.start..];
        let end = tail
            .iter()
            .take(64)
            .position(|&b| b == 0)
            .ok_or_else(|| Error::Invalid("invalid course stem".into()))?;
        let stem = String::from_utf8_lossy(&tail[..end]).into_owned();
        if !stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(Error::Invalid("unsafe course stem".into()));
        }
        let music_record = music_record(bytes, id).ok();
        let sequence = music_record
            .and_then(|r| {
                bytes
                    .get((mkds_data::MUSIC_TABLE - mkds_data::ARM9_BASE + r * 4 + 1) as usize)
                    .copied()
            })
            .map(u32::from);
        courses.push(CourseSlot {
            id,
            stem,
            variants: word(bytes, at + 4)? == 1,
            music_record,
            sequence,
        });
    }
    let mut cups = vec![];
    for cup in 0..8 {
        cups.push(
            (0..4)
                .map(|i| word(bytes, mkds_data::CUP_TABLE + cup * 16 + i * 4))
                .collect::<Result<_>>()?,
        );
    }
    Ok(Catalog { courses, cups })
}
/// Decode the actual ARM branch and MOV-immediate instead of embedding retail table bytes.
pub fn music_record(bytes: &[u8], course: u32) -> Result<u32> {
    if course >= 54 {
        return Err(Error::Invalid("course has no music branch".into()));
    }
    let at = mkds_data::MUSIC_BRANCHES + course * 4;
    let branch = word(bytes, at)?;
    if branch >> 24 != 0xea {
        return Err(Error::Invalid("unsupported music dispatch branch".into()));
    }
    let delta = ((branch << 8) as i32 >> 6) as i64;
    let target = (at as i64 + 8 + delta) as u32;
    let mov = word(bytes, target)?;
    if mov & 0xffff_f000 != 0xe3a0_c000 {
        return Err(Error::Invalid(
            "unsupported music dispatch immediate".into(),
        ));
    }
    let index = (mov & 255).rotate_right(((mov >> 8) & 15) * 2);
    if index >= 51 {
        return Err(Error::Invalid("course has no music record".into()));
    }
    Ok(index)
}
pub fn set_cup(p: &mut Project, s: &RomSession, cup: u32, courses: [u32; 4]) -> Result<()> {
    if cup >= 8 {
        return Err(Error::Invalid("choose one of the eight cup slots".into()));
    }
    let c = catalog(Some(p), s)?;
    if courses.iter().any(|id| {
        !c.courses
            .iter()
            .any(|c| c.id == *id && c.music_record.is_some() && c.id <= 40)
    }) {
        return Err(Error::Invalid("choose four race courses".into()));
    }
    let mut arm = p.bytes(s, ARM9)?.to_vec();
    write(
        &mut arm,
        mkds_data::ARM9_BASE,
        mkds_data::CUP_TABLE + cup * 16,
        &courses.map(Value::U32),
    )?;
    let mut edits = std::collections::BTreeMap::from([(ARM9.into(), arm)]);
    for node in s.nodes.iter().filter(|n| n.path.ends_with("/common.bmg")) {
        let mut b = nitro_formats::bmg::Bmg::parse(p.bytes(s, &node.path)?)?;
        if b.messages.len() < 77 {
            continue;
        }
        let messages = b.messages.clone();
        for (race, id) in courses.iter().enumerate() {
            if let Some(pos) = c.cups.iter().flatten().position(|v| v == id) {
                b.messages[45 + cup as usize * 4 + race].tokens = messages[45 + pos].tokens.clone();
            }
        }
        edits.insert(node.path.clone(), b.write());
    }
    p.edit_many(edits, "Edit cup courses and names", s)
}
pub fn assign_music(p: &mut Project, s: &RomSession, course: u32, sequence: u8) -> Result<()> {
    let data = nitro_formats::audio::Sdat::parse(p.bytes(s, "data/Sound/sound_data.sdat")?)?;
    data.resource_file(0, sequence as usize)?;
    let record = music_record(p.bytes(s, ARM9)?, course)?;
    edit(
        p,
        s,
        None,
        mkds_data::MUSIC_TABLE + record * 4 + 1,
        &[Value::U8(sequence)],
    )
}
