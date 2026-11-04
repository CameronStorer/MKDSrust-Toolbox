use super::{header, wave::decode};
use crate::{error, put16, put32, slice, u16le, u32le, Result};
#[derive(Clone, Debug)]
pub struct Strm {
    pub encoding: u8,
    pub sample_rate: u16,
    pub loop_sample: Option<u32>,
    pub samples: u32,
    pub channels: Vec<Vec<Vec<u8>>>,
    raw: Vec<u8>,
}
impl Strm {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let h = header(b, b"STRM")?;
        if slice(h, 16, 4)? != b"HEAD" || slice(h, 96, 4)? != b"DATA" {
            return Err(error(16, "invalid STRM HEAD/DATA"));
        }
        slice(h, 24, 72)?;
        let encoding = h[24];
        let channels = h[26] as usize;
        let blocks = u32le(h, 44)? as usize;
        let bytes = u32le(h, 48)? as usize;
        let last = u32le(h, 56)? as usize;
        if encoding > 2 || channels == 0 || channels > 2 || blocks > 1_000_000 || u16le(h, 28)? == 0
        {
            return Err(error(24, "invalid STRM parameters"));
        }
        let mut out = vec![vec![]; channels];
        let mut at = 104;
        for i in 0..blocks {
            let len = if i + 1 == blocks { last } else { bytes }
                + if encoding == 2 && blocks == 1 { 4 } else { 0 };
            for c in &mut out {
                c.push(slice(h, at, len)?.to_vec());
                at += len;
            }
            at = (at + 3) & !3;
        }
        Ok(Self {
            encoding,
            sample_rate: u16le(h, 28)?,
            loop_sample: (h[25] != 0).then_some(u32le(h, 32)?),
            samples: u32le(h, 36)?,
            channels: out,
            raw: b.to_vec(),
        })
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        let original = Self::parse(&self.raw)?;
        if self.encoding == original.encoding
            && self.sample_rate == original.sample_rate
            && self.loop_sample == original.loop_sample
            && self.samples == original.samples
            && self.channels == original.channels
        {
            return Ok(self.raw.clone());
        }
        if self.channels.is_empty()
            || self.channels.len() > 2
            || self.encoding > 2
            || self.sample_rate == 0
            || self.loop_sample.is_some_and(|n| n >= self.samples)
        {
            return Err(error(0, "invalid STRM settings"));
        }
        let blocks = self.channels[0].len();
        if blocks == 0 || self.channels.iter().any(|c| c.len() != blocks) {
            return Err(error(0, "inconsistent stream blocks"));
        }
        let first = self.channels[0][0].len();
        let last = self.channels[0][blocks - 1].len();
        if self.channels.iter().any(|c| {
            c[..blocks - 1].iter().any(|b| b.len() != first) || c[blocks - 1].len() != last
        }) {
            return Err(error(0, "inconsistent stream block sizes"));
        }
        let adjust = if self.encoding == 2 && blocks == 1 {
            4
        } else {
            0
        };
        if first < adjust || last < adjust {
            return Err(error(0, "truncated ADPCM stream state"));
        }
        let mut b = self.raw[..104].to_vec();
        b[24] = self.encoding;
        b[25] = u8::from(self.loop_sample.is_some());
        b[26] = self.channels.len() as u8;
        put16(&mut b, 28, self.sample_rate);
        put16(
            &mut b,
            30,
            (16756991u32 / self.sample_rate as u32 / 32) as u16,
        );
        put32(&mut b, 32, self.loop_sample.unwrap_or(0));
        put32(&mut b, 36, self.samples);
        put32(&mut b, 44, blocks as u32);
        put32(&mut b, 48, (first - adjust) as u32);
        put32(&mut b, 56, (last - adjust) as u32);
        let sample_count = |bytes: usize| match self.encoding {
            0 => bytes,
            1 => bytes / 2,
            _ => bytes.saturating_sub(4) * 2 + 1,
        };
        put32(&mut b, 52, sample_count(first) as u32);
        put32(&mut b, 60, sample_count(last) as u32);
        for i in 0..blocks {
            for channel in &self.channels {
                b.extend(&channel[i]);
            }
            crate::g3d::align(&mut b, 4);
        }
        let len = b.len();
        put32(&mut b, 8, len as u32);
        put32(&mut b, 100, (len - 96) as u32);
        Ok(b)
    }
    pub fn pcm(&self) -> Result<Vec<Vec<i16>>> {
        self.channels
            .iter()
            .map(|blocks| {
                let mut pcm = vec![];
                for b in blocks {
                    pcm.extend(decode(self.encoding, b)?);
                }
                pcm.truncate(self.samples as usize);
                Ok(pcm)
            })
            .collect()
    }
    pub fn from_pcm(channels: &[Vec<i16>], rate: u16, loop_sample: Option<u32>) -> Result<Self> {
        if channels.is_empty()
            || channels.len() > 2
            || rate == 0
            || channels[0].is_empty()
            || channels.iter().any(|c| c.len() != channels[0].len())
            || loop_sample.is_some_and(|n| n as usize >= channels[0].len())
        {
            return Err(error(0, "invalid stream PCM"));
        }
        let n = channels[0].len();
        let mut b = vec![0; 104];
        b[..4].copy_from_slice(b"STRM");
        put16(&mut b, 4, 0xfeff);
        put16(&mut b, 6, 0x100);
        put16(&mut b, 12, 16);
        put16(&mut b, 14, 2);
        b[16..20].copy_from_slice(b"HEAD");
        put32(&mut b, 20, 80);
        b[24] = 1;
        b[25] = u8::from(loop_sample.is_some());
        b[26] = channels.len() as u8;
        put16(&mut b, 28, rate);
        put16(&mut b, 30, (16756991u32 / rate as u32 / 32) as u16);
        put32(&mut b, 32, loop_sample.unwrap_or(0));
        put32(&mut b, 36, n as u32);
        put32(&mut b, 40, 104);
        put32(&mut b, 44, 1);
        put32(&mut b, 48, (n * 2) as u32);
        put32(&mut b, 52, n as u32);
        put32(&mut b, 56, (n * 2) as u32);
        put32(&mut b, 60, n as u32);
        b[96..100].copy_from_slice(b"DATA");
        for c in channels {
            for s in c {
                b.extend(s.to_le_bytes());
            }
        }
        crate::g3d::align(&mut b, 4);
        let len = b.len();
        put32(&mut b, 8, len as u32);
        put32(&mut b, 100, (len - 96) as u32);
        Self::parse(&b)
    }
}
