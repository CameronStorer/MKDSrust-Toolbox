use super::{data, file};
use crate::{error, put32, slice, u16le, u32le, Result};
#[derive(Clone, Debug)]
pub struct Sseq {
    pub events: Vec<u8>,
    prefix: Vec<u8>,
    raw: Vec<u8>,
    original: Vec<u8>,
}
impl Sseq {
    pub fn new(events: Vec<u8>) -> Self {
        Self {
            events,
            prefix: vec![],
            raw: vec![],
            original: vec![],
        }
    }
    pub fn parse(b: &[u8]) -> Result<Self> {
        let _ = data(b, b"SSEQ")?;
        let at = u32le(b, 24)? as usize;
        let end = u32le(b, 8)? as usize;
        if at < 28 || at > end {
            return Err(error(24, "invalid SSEQ event offset"));
        }
        let events = slice(b, at, end - at)?.to_vec();
        Ok(Self {
            original: events.clone(),
            events,
            prefix: b[..at].to_vec(),
            raw: b.to_vec(),
        })
    }
    pub fn write(&self) -> Vec<u8> {
        if self.events == self.original && !self.raw.is_empty() {
            return self.raw.clone();
        }
        let mut b = if self.prefix.is_empty() {
            let mut body = 28u32.to_le_bytes().to_vec();
            body.extend(&self.events);
            return file(b"SSEQ", &body);
        } else {
            self.prefix.clone()
        };
        b.extend(&self.events);
        let len = b.len();
        put32(&mut b, 8, len as u32);
        put32(&mut b, 20, (len - 16) as u32);
        b
    }
}
pub fn variable(value: u32) -> Vec<u8> {
    let mut out = vec![(value & 127) as u8];
    let mut n = value >> 7;
    while n > 0 {
        out.push((n as u8 & 127) | 128);
        n >>= 7;
    }
    out.reverse();
    out
}
fn read_var(b: &[u8], at: &mut usize) -> Result<u32> {
    let mut v = 0u32;
    for _ in 0..4 {
        let n = slice(b, *at, 1)?[0];
        *at += 1;
        v = (v << 7) | (n & 127) as u32;
        if n & 128 == 0 {
            return Ok(v);
        }
    }
    Err(error(*at, "sequence variable length exceeds four bytes"))
}
#[derive(Clone, Debug)]
pub struct SequenceNote {
    pub tick: u32,
    pub duration: u32,
    pub track: u8,
    pub key: u8,
    pub velocity: u8,
    pub program: u32,
    pub volume: u8,
    pub pan: u8,
    pub bend: f32,
}
#[derive(Clone, Debug, Default)]
pub struct SequenceTimeline {
    pub notes: Vec<SequenceNote>,
    pub tempos: Vec<(u32, u16)>,
    pub ticks: u32,
}
/// Basic bounded scheduler for auditioning sequences. Unsupported commands are
/// reported explicitly; the file parser still retains them byte for byte.
pub fn timeline(events: &[u8], max_ticks: u32) -> Result<SequenceTimeline> {
    schedule(events, max_ticks, false)
}
/// Audition a complete arrangement through its first infinite loop.
pub fn timeline_once(events: &[u8], max_ticks: u32) -> Result<SequenceTimeline> {
    schedule(events, max_ticks, true)
}
fn schedule(events: &[u8], max_ticks: u32, once: bool) -> Result<SequenceTimeline> {
    let mut out = SequenceTimeline {
        tempos: vec![(0, 120)],
        ..SequenceTimeline::default()
    };
    let mut tracks = vec![(0u8, 0usize)];
    let mut opened = std::collections::BTreeSet::from([0u8]);
    let mut work = 0;
    let mut variables = [0i16; 32];
    while let Some((track, mut at)) = tracks.pop() {
        let mut tick = 0u32;
        let mut wait = true;
        let mut program = 0;
        let mut volume = 127;
        let mut pan = 64;
        let mut transpose = 0i8;
        let mut bend = 0i8;
        let mut bend_range = 2u8;
        let mut stack = vec![];
        let mut loops = vec![];
        let mut condition = true;
        while at < events.len() && tick < max_ticks {
            work += 1;
            if work > 250_000 {
                return Err(error(at, "sequence preview work limit"));
            }
            let op = events[at];
            at += 1;
            match op {
                0..=127 => {
                    let velocity = slice(events, at, 1)?[0];
                    at += 1;
                    let duration = read_var(events, &mut at)?;
                    out.notes.push(SequenceNote {
                        tick,
                        duration: if duration == 0 {
                            max_ticks - tick
                        } else {
                            duration
                        },
                        track,
                        key: (op as i16 + transpose as i16).clamp(0, 127) as u8,
                        velocity,
                        program,
                        volume,
                        pan,
                        bend: bend as f32 / 128. * bend_range as f32,
                    });
                    if wait {
                        tick = tick.saturating_add(duration);
                    }
                }
                0x80 => tick = tick.saturating_add(read_var(events, &mut at)?),
                0xa2 => {
                    if !condition {
                        skip_event(events, &mut at)?;
                    }
                }
                0x81 => program = read_var(events, &mut at)?,
                0x93 => {
                    let p = slice(events, at, 4)?;
                    let id = p[0];
                    let offset = p[1] as usize | ((p[2] as usize) << 8) | ((p[3] as usize) << 16);
                    if id >= 16 {
                        return Err(error(at, "SSEQ exceeds 16 tracks"));
                    }
                    if opened.insert(id) {
                        tracks.push((id, offset));
                    }
                    at += 4;
                }
                0x94 | 0x95 => {
                    let p = slice(events, at, 3)?;
                    let target = p[0] as usize | ((p[1] as usize) << 8) | ((p[2] as usize) << 16);
                    if target >= events.len() {
                        return Err(error(at, "sequence branch out of bounds"));
                    }
                    at += 3;
                    if once && op == 0x94 && target < at {
                        break;
                    }
                    if op == 0x95 {
                        if stack.len() >= 16 {
                            return Err(error(at, "sequence call stack limit"));
                        }
                        stack.push(at);
                    }
                    at = target;
                }
                0xc0..=0xd6 => {
                    let value = slice(events, at, 1)?[0];
                    at += 1;
                    match op {
                        0xc0 => pan = value,
                        0xc1 => volume = value,
                        0xc3 => transpose = value as i8,
                        0xc4 => bend = value as i8,
                        0xc5 => bend_range = value,
                        0xc7 => wait = value != 0,
                        0xd4 => loops.push((at, value)),
                        _ => {}
                    }
                }
                0xe0 | 0xe1 | 0xe3 => {
                    let value = u16le(events, at)?;
                    at += 2;
                    if op == 0xe1 {
                        out.tempos.push((tick, value.max(1)));
                    }
                }
                0xfc => {
                    let Some((start, count)) = loops.last_mut() else {
                        return Err(error(at, "loop end without start"));
                    };
                    if *count == 0 {
                        if once {
                            break;
                        }
                        at = *start;
                    } else if *count > 1 {
                        *count -= 1;
                        at = *start;
                    } else {
                        loops.pop();
                    }
                }
                0xfd => {
                    let Some(target) = stack.pop() else {
                        break;
                    };
                    at = target;
                }
                0xfe => {
                    slice(events, at, 2)?;
                    at += 2;
                }
                0xff => break,
                0xb0..=0xbd => {
                    let data = slice(events, at, 3)?;
                    let id = data[0] as usize;
                    let value = i16::from_le_bytes([data[1], data[2]]);
                    let v = variables
                        .get_mut(id)
                        .ok_or_else(|| error(at, "sequence variable outside range"))?;
                    match op {
                        0xb0 => *v = value,
                        0xb1 => *v = v.wrapping_add(value),
                        0xb2 => *v = v.wrapping_sub(value),
                        0xb3 => *v = v.wrapping_mul(value),
                        0xb4 => {
                            if value != 0 {
                                *v = v.wrapping_div(value);
                            }
                        }
                        0xb5 => {
                            *v = if value >= 0 {
                                v.wrapping_shl(value as u32)
                            } else {
                                v.wrapping_shr(value.unsigned_abs() as u32)
                            }
                        }
                        0xb6 => *v = value / 2, // Stable audition of a game-controlled random variation.
                        0xb8 => condition = *v == value,
                        0xb9 => condition = *v >= value,
                        0xba => condition = *v > value,
                        0xbb => condition = *v <= value,
                        0xbc => condition = *v < value,
                        0xbd => condition = *v != value,
                        _ => {}
                    }
                    at += 3;
                }
                _ => {
                    return Err(error(
                        at - 1,
                        format!("SSEQ preview does not yet support command {op:02x}"),
                    ))
                }
            }
        }
        out.ticks = out.ticks.max(tick.min(max_ticks));
    }
    out.tempos.sort_by_key(|(tick, _)| *tick);
    out.notes.sort_by_key(|n| n.tick);
    Ok(out)
}
// Format-only lengths for conditional execution, per ndspy's sequence reference.
fn skip_event(events: &[u8], at: &mut usize) -> Result<()> {
    let op = slice(events, *at, 1)?[0];
    *at += 1;
    let bytes = match op {
        0..=127 => {
            slice(events, *at, 1)?;
            *at += 1;
            read_var(events, at)?;
            0
        }
        0x80 | 0x81 => {
            read_var(events, at)?;
            0
        }
        0x93 => 4,
        0x94 | 0x95 => 3,
        0xb0..=0xbd => 3,
        0xc0..=0xdf => 1,
        0xe0..=0xef | 0xfe => 2,
        0xfc | 0xfd | 0xff => 0,
        _ => return Err(error(*at - 1, "unsupported conditional sequence command")),
    };
    slice(events, *at, bytes)?;
    *at += bytes;
    Ok(())
}
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceArchiveEntry {
    pub offset: Option<u32>,
    pub bank: u16,
    pub volume: u8,
    pub priorities: [u8; 3],
}
#[derive(Clone, Debug)]
pub struct Ssar {
    pub entries: Vec<SequenceArchiveEntry>,
    pub events: Vec<u8>,
    raw: Vec<u8>,
}
impl Ssar {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let _ = data(b, b"SSAR")?;
        let at = u32le(b, 24)? as usize;
        let n = u32le(b, 28)? as usize;
        if n > 65535 || at < 32 + n * 12 || at > u32le(b, 8)? as usize {
            return Err(error(24, "invalid SSAR table"));
        }
        let mut entries = vec![];
        for i in 0..n {
            let p = 32 + i * 12;
            let v = u32le(b, p)?;
            slice(b, p, 12)?;
            if v != u32::MAX && at.checked_add(v as usize).is_none_or(|a| a >= b.len()) {
                return Err(error(p, "SSAR sequence offset outside events"));
            }
            entries.push(SequenceArchiveEntry {
                offset: (v != u32::MAX).then_some(v),
                bank: u16le(b, p + 4)?,
                volume: b[p + 6],
                priorities: [b[p + 7], b[p + 8], b[p + 9]],
            });
        }
        Ok(Self {
            entries,
            events: slice(b, at, u32le(b, 8)? as usize - at)?.to_vec(),
            raw: b.to_vec(),
        })
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        let original = Self::parse(&self.raw)?;
        if self.entries == original.entries && self.events == original.events {
            return Ok(self.raw.clone());
        }
        let mut body = vec![0; 8 + self.entries.len() * 12];
        put32(&mut body, 0, (32 + self.entries.len() * 12) as u32);
        put32(&mut body, 4, self.entries.len() as u32);
        for (i, e) in self.entries.iter().enumerate() {
            if e.offset.is_some_and(|v| v as usize >= self.events.len()) {
                return Err(error(i, "SSAR entry points outside events"));
            }
            let at = 8 + i * 12;
            put32(&mut body, at, e.offset.unwrap_or(u32::MAX));
            crate::put16(&mut body, at + 4, e.bank);
            body[at + 6] = e.volume;
            body[at + 7..at + 10].copy_from_slice(&e.priorities);
        }
        body.extend(&self.events);
        Ok(file(b"SSAR", &body))
    }
    pub fn new(entries: Vec<SequenceArchiveEntry>, events: Vec<u8>) -> Result<Self> {
        let mut body = vec![0; 8 + entries.len() * 12];
        put32(&mut body, 0, (32 + entries.len() * 12) as u32);
        put32(&mut body, 4, entries.len() as u32);
        for (i, e) in entries.iter().enumerate() {
            let p = 8 + i * 12;
            put32(&mut body, p, e.offset.unwrap_or(u32::MAX));
            crate::put16(&mut body, p + 4, e.bank);
            body[p + 6] = e.volume;
            body[p + 7..p + 10].copy_from_slice(&e.priorities);
        }
        body.extend(events);
        Self::parse(&file(b"SSAR", &body))
    }
}
