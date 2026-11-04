use super::{data, file};
use crate::{error, put16, put32, slice, u16le, u32le, Result};
#[derive(Clone, Debug, PartialEq)]
pub struct NoteDefinition {
    pub kind: u16,
    pub wave: u16,
    pub archive: u16,
    pub key: u8,
    pub attack: u8,
    pub decay: u8,
    pub sustain: u8,
    pub release: u8,
    pub pan: u8,
}
impl NoteDefinition {
    pub fn sample(wave: u16, pan: u8) -> Self {
        Self {
            kind: 1,
            wave,
            archive: 0,
            key: 60,
            attack: 127,
            decay: 127,
            sustain: 127,
            release: 127,
            pan,
        }
    }
    fn parse(b: &[u8], kind: u16) -> Result<Self> {
        slice(b, 0, 10)?;
        Ok(Self {
            kind,
            wave: u16le(b, 0)?,
            archive: u16le(b, 2)?,
            key: b[4],
            attack: b[5],
            decay: b[6],
            sustain: b[7],
            release: b[8],
            pan: b[9],
        })
    }
    fn bytes(&self, typed: bool) -> Vec<u8> {
        let mut b = vec![];
        if typed {
            b.extend(self.kind.to_le_bytes());
        }
        b.extend(self.wave.to_le_bytes());
        b.extend(self.archive.to_le_bytes());
        b.extend([
            self.key,
            self.attack,
            self.decay,
            self.sustain,
            self.release,
            self.pan,
        ]);
        b
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Instrument {
    pub kind: u8,
    pub regions: Vec<(u8, u8, NoteDefinition)>,
}
impl Instrument {
    pub fn sample(wave: u16, pan: u8) -> Self {
        Self {
            kind: 1,
            regions: vec![(0, 127, NoteDefinition::sample(wave, pan))],
        }
    }
    pub fn note(&self, key: u8) -> Option<&NoteDefinition> {
        self.regions
            .iter()
            .find(|(lo, hi, _)| key >= *lo && key <= *hi)
            .map(|(_, _, n)| n)
    }
}
#[derive(Clone, Debug)]
pub struct Sbnk {
    pub instruments: Vec<Instrument>,
    raw: Vec<u8>,
    original: Vec<Instrument>,
}
impl Sbnk {
    pub fn new(instruments: Vec<Instrument>) -> Self {
        Self {
            instruments,
            raw: vec![],
            original: vec![],
        }
    }
    pub fn parse(b: &[u8]) -> Result<Self> {
        let _ = data(b, b"SBNK")?;
        let n = u32le(b, 56)? as usize;
        if n > 65535 {
            return Err(error(56, "instrument count exceeds limit"));
        }
        slice(b, 60, n * 4)?;
        let mut instruments = vec![];
        for i in 0..n {
            let kind = b[60 + i * 4];
            let at = u16le(b, 61 + i * 4)? as usize;
            let mut regions = vec![];
            match kind {
                0 => {}
                1..=15 => regions.push((
                    0,
                    127,
                    NoteDefinition::parse(slice(b, at, 10)?, kind as u16)?,
                )),
                16 => {
                    let first = slice(b, at, 2)?[0];
                    let last = b[at + 1];
                    if last < first {
                        return Err(error(at, "invalid instrument key range"));
                    }
                    for (j, key) in (first..=last).enumerate() {
                        let p = at + 2 + j * 12;
                        regions.push((
                            key,
                            key,
                            NoteDefinition::parse(slice(b, p + 2, 10)?, u16le(b, p)?)?,
                        ));
                    }
                }
                17 => {
                    let ends = slice(b, at, 8)?;
                    let mut first = 0;
                    for (j, &last) in ends.iter().enumerate() {
                        if j > 0 && last == 0 {
                            break;
                        }
                        if last < first {
                            return Err(error(at, "invalid split instrument"));
                        }
                        let p = at + 8 + j * 12;
                        regions.push((
                            first,
                            last,
                            NoteDefinition::parse(slice(b, p + 2, 10)?, u16le(b, p)?)?,
                        ));
                        if last == 127 {
                            break;
                        }
                        first = last + 1;
                    }
                }
                _ => return Err(error(at, "unknown instrument type")),
            }
            instruments.push(Instrument { kind, regions });
        }
        Ok(Self {
            original: instruments.clone(),
            instruments,
            raw: b.to_vec(),
        })
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        if !self.raw.is_empty() && self.instruments == self.original {
            return Ok(self.raw.clone());
        }
        let mut b = vec![0; 36 + self.instruments.len() * 4];
        put32(&mut b, 32, self.instruments.len() as u32);
        for (i, inst) in self.instruments.iter().enumerate() {
            b[36 + i * 4] = inst.kind;
            if inst.kind == 0 {
                continue;
            }
            let at = b.len() + 24;
            if at > 65535 {
                return Err(error(at, "SBNK instrument data exceeds 64 KiB"));
            }
            put16(&mut b, 37 + i * 4, at as u16);
            match inst.kind {
                1..=15 => {
                    let note = inst
                        .regions
                        .first()
                        .ok_or_else(|| error(i, "missing instrument note"))?;
                    b.extend(note.2.bytes(false));
                }
                16 => {
                    let first = inst
                        .regions
                        .first()
                        .ok_or_else(|| error(i, "empty key range"))?
                        .0;
                    let last = inst.regions.last().unwrap().1;
                    if inst.regions.len() != last as usize - first as usize + 1 {
                        return Err(error(i, "non-contiguous instrument range"));
                    }
                    b.extend([first, last]);
                    for (_, _, n) in &inst.regions {
                        b.extend(n.bytes(true));
                    }
                }
                17 => {
                    if inst.regions.is_empty() || inst.regions.len() > 8 {
                        return Err(error(i, "split bank requires 1–8 regions"));
                    }
                    let mut ends = [0; 8];
                    for (k, (_, end, _)) in inst.regions.iter().enumerate() {
                        ends[k] = *end;
                    }
                    b.extend(ends);
                    for (_, _, n) in &inst.regions {
                        b.extend(n.bytes(true));
                    }
                }
                _ => return Err(error(i, "unknown instrument type")),
            }
        }
        Ok(file(b"SBNK", &b))
    }
}
