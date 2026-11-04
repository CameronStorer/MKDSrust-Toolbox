use super::{data, file};
use crate::{error, put16, put32, slice, u16le, u32le, Result};
#[derive(Clone, Debug, PartialEq)]
pub struct Swav {
    pub encoding: u8,
    pub looped: bool,
    pub sample_rate: u16,
    pub timer: u16,
    pub loop_words: u16,
    pub data: Vec<u8>,
}
impl Swav {
    pub fn parse(b: &[u8]) -> Result<Self> {
        Self::parse_info(data(b, b"SWAV")?)
    }
    pub(crate) fn parse_info(b: &[u8]) -> Result<Self> {
        let encoding = slice(b, 0, 1)?[0];
        let rate = u16le(b, 2)?;
        let loop_words = u16le(b, 6)?;
        let words = (loop_words as usize)
            .checked_add(u32le(b, 8)? as usize)
            .ok_or_else(|| error(8, "wave length overflow"))?;
        if encoding > 2 || rate == 0 || words > 32 * 1024 * 1024 {
            return Err(error(0, "invalid SWAV encoding, rate or length"));
        }
        let data = slice(b, 12, words * 4)?.to_vec();
        if encoding == 2 && (data.len() < 4 || data[2] > 88 || data[3] != 0) {
            return Err(error(12, "invalid ADPCM state"));
        }
        Ok(Self {
            encoding,
            looped: b[1] != 0,
            sample_rate: rate,
            timer: u16le(b, 4)?,
            loop_words,
            data,
        })
    }
    pub(crate) fn info(&self) -> Result<Vec<u8>> {
        if self.encoding > 2
            || self.sample_rate == 0
            || !self.data.len().is_multiple_of(4)
            || self.loop_words as usize > self.data.len() / 4
        {
            return Err(error(0, "invalid SWAV"));
        }
        let mut b = vec![0; 12];
        b[0] = self.encoding;
        b[1] = u8::from(self.looped);
        put16(&mut b, 2, self.sample_rate);
        put16(&mut b, 4, self.timer);
        put16(&mut b, 6, self.loop_words);
        put32(
            &mut b,
            8,
            (self.data.len() / 4 - self.loop_words as usize) as u32,
        );
        b.extend(&self.data);
        Ok(b)
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        Ok(file(b"SWAV", &self.info()?))
    }
    pub fn samples(&self) -> Result<Vec<i16>> {
        decode(self.encoding, &self.data)
    }
    pub fn loop_sample(&self) -> usize {
        match self.encoding {
            0 => self.loop_words as usize * 4,
            1 => self.loop_words as usize * 2,
            _ => (self.loop_words as usize * 8).saturating_sub(7),
        }
    }
    pub fn from_pcm(
        samples: &[i16],
        rate: u16,
        encoding: u8,
        loop_start: Option<usize>,
    ) -> Result<Self> {
        if rate == 0
            || samples.is_empty()
            || encoding > 2
            || loop_start.is_some_and(|n| n >= samples.len())
        {
            return Err(error(0, "invalid PCM wave or loop"));
        }
        let mut data = match encoding {
            0 => samples.iter().map(|s| (*s >> 8) as i8 as u8).collect(),
            1 => samples.iter().flat_map(|s| s.to_le_bytes()).collect(),
            _ => encode_adpcm(samples),
        };
        while !data.len().is_multiple_of(4) {
            data.push(0);
        }
        let loop_words = match encoding {
            0 => loop_start.unwrap_or(0) / 4,
            1 => loop_start.unwrap_or(0) / 2,
            _ => loop_start.map_or(0, |n| n.div_ceil(8)),
        };
        if loop_words > 65535 {
            return Err(error(0, "loop start exceeds SWAV address range"));
        }
        Ok(Self {
            encoding,
            looped: loop_start.is_some(),
            sample_rate: rate,
            timer: (16756991u32 / rate as u32).min(65535) as u16,
            loop_words: loop_words as u16,
            data,
        })
    }
}
const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
fn step(n: u8, predict: &mut i32, index: &mut i32) {
    let s = STEPS[*index as usize];
    let delta = s / 8
        + if n & 1 != 0 { s / 4 } else { 0 }
        + if n & 2 != 0 { s / 2 } else { 0 }
        + if n & 4 != 0 { s } else { 0 };
    *predict = (*predict + if n & 8 != 0 { -delta } else { delta }).clamp(-32768, 32767);
    *index = (*index + INDEX[(n & 7) as usize]).clamp(0, 88);
}
pub(crate) fn decode(encoding: u8, data: &[u8]) -> Result<Vec<i16>> {
    match encoding {
        0 => Ok(data.iter().map(|s| (*s as i8 as i16) << 8).collect()),
        1 => Ok(data
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect()),
        2 => {
            slice(data, 0, 4)?;
            let mut p = u16le(data, 0)? as i16 as i32;
            let mut idx = data[2] as i32;
            if idx > 88 {
                return Err(error(2, "ADPCM step index out of range"));
            }
            let mut out = vec![p as i16];
            for byte in &data[4..] {
                for n in [byte & 15, byte >> 4] {
                    step(n, &mut p, &mut idx);
                    out.push(p as i16);
                }
            }
            Ok(out)
        }
        _ => Err(error(0, "unsupported wave encoding")),
    }
}
fn encode_adpcm(samples: &[i16]) -> Vec<u8> {
    let mut p = samples[0] as i32;
    let mut index = 0;
    let mut out = p.to_le_bytes()[..2].to_vec();
    out.extend([0, 0]);
    let mut low = None;
    for &s in &samples[1..] {
        let diff = s as i32 - p;
        let step_size = STEPS[index as usize];
        let mut magnitude = diff.abs();
        let mut n = if diff < 0 { 8 } else { 0 };
        for (bit, factor) in [(4, 1), (2, 2), (1, 4)] {
            if magnitude >= step_size / factor {
                n |= bit;
                magnitude -= step_size / factor;
            }
        }
        step(n, &mut p, &mut index);
        if let Some(l) = low.take() {
            out.push(l | n << 4);
        } else {
            low = Some(n);
        }
    }
    if let Some(l) = low {
        out.push(l);
    }
    out
}
#[derive(Clone, Debug)]
pub struct Swar {
    pub waves: Vec<Swav>,
    raw: Vec<u8>,
    original: Vec<Swav>,
}
impl Swar {
    pub fn new(waves: Vec<Swav>) -> Self {
        Self {
            waves,
            raw: vec![],
            original: vec![],
        }
    }
    pub fn parse(b: &[u8]) -> Result<Self> {
        let _ = data(b, b"SWAR")?;
        let n = u32le(b, 56)? as usize;
        if n > 65535 {
            return Err(error(56, "too many SWAR waves"));
        }
        slice(b, 60, n * 4)?;
        let mut waves = vec![];
        for i in 0..n {
            let at = u32le(b, 60 + i * 4)? as usize;
            let end = if i + 1 < n {
                u32le(b, 64 + i * 4)? as usize
            } else {
                u32le(b, 8)? as usize
            };
            if end < at {
                return Err(error(at, "invalid SWAR offsets"));
            }
            waves.push(Swav::parse_info(slice(b, at, end - at)?)?);
        }
        Ok(Self {
            original: waves.clone(),
            waves,
            raw: b.to_vec(),
        })
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        if !self.raw.is_empty() && self.waves == self.original {
            return Ok(self.raw.clone());
        }
        let mut b = vec![0; 36 + self.waves.len() * 4];
        put32(&mut b, 32, self.waves.len() as u32);
        for (i, w) in self.waves.iter().enumerate() {
            let at = b.len() + 24;
            put32(&mut b, 36 + i * 4, at as u32);
            b.extend(w.info()?);
        }
        Ok(file(b"SWAR", &b))
    }
}
