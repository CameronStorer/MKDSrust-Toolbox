use crate::{error, put32, slice, u16le, u32le, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Token {
    Text(Vec<u16>),
    Code(Vec<u8>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: u32,
    pub tokens: Vec<Token>,
}
#[derive(Clone, Debug)]
pub struct Bmg {
    raw: Vec<u8>,
    sections: Vec<Vec<u8>>,
    inf: usize,
    dat: usize,
    pub messages: Vec<Message>,
}
impl Bmg {
    pub fn parse(b: &[u8]) -> Result<Self> {
        if slice(b, 0, 8)? != b"MESGbmg1" {
            return Err(error(0, "expected MESGbmg1"));
        }
        let size = u32le(b, 8)? as usize;
        if size > b.len() || size < 32 {
            return Err(error(8, "invalid BMG size"));
        }
        if slice(b, 16, 1)?[0] != 2 {
            return Err(error(16, "only UTF-16 BMG supported"));
        }
        let mut sections = vec![];
        let mut p = 32;
        let mut inf = None;
        let mut dat = None;
        let mut mid = None;
        for i in 0..u32le(b, 12)? as usize {
            let len = u32le(b, p + 4)? as usize;
            if len < 8 || p + len > size {
                return Err(error(p + 4, "invalid BMG section size"));
            }
            let s = slice(b, p, len)?.to_vec();
            match &s[..4] {
                b"INF1" => inf = Some(i),
                b"DAT1" => dat = Some(i),
                b"MID1" => mid = Some(i),
                _ => {}
            }
            sections.push(s);
            p += len;
        }
        let inf = inf.ok_or_else(|| error(32, "missing INF1"))?;
        let dat = dat.ok_or_else(|| error(32, "missing DAT1"))?;
        let info = &sections[inf];
        let data = &sections[dat];
        let count = u16le(info, 8)? as usize;
        let stride = u16le(info, 10)? as usize;
        if stride < 4 {
            return Err(error(42, "INF1 entries too short"));
        }
        slice(info, 16, count * stride)?;
        if let Some(mid) = mid {
            if u16le(&sections[mid], 8)? as usize != count {
                return Err(error(32, "MID1 count differs from INF1"));
            }
            slice(&sections[mid], 16, count * 4)?;
        }
        let mut messages = vec![];
        for i in 0..count {
            let mut pos = 8 + u32le(info, 16 + i * stride)? as usize;
            let mut tokens = vec![];
            let mut text = vec![];
            loop {
                let ch = u16le(data, pos)?;
                if ch == 0 {
                    break;
                }
                if ch == 0x1a {
                    if !text.is_empty() {
                        tokens.push(Token::Text(std::mem::take(&mut text)));
                    }
                    let len = slice(data, pos + 2, 1)?[0] as usize;
                    if len < 4 || !len.is_multiple_of(2) {
                        return Err(error(pos + 2, "invalid BMG escape size"));
                    }
                    tokens.push(Token::Code(slice(data, pos, len)?.to_vec()));
                    pos += len;
                } else {
                    text.push(ch);
                    pos += 2;
                }
            }
            if !text.is_empty() {
                tokens.push(Token::Text(text));
            }
            messages.push(Message {
                id: match mid {
                    Some(m) => u32le(&sections[m], 16 + i * 4)?,
                    None => i as u32,
                },
                tokens,
            });
        }
        Ok(Self {
            raw: b.to_vec(),
            sections,
            inf,
            dat,
            messages,
        })
    }
    pub fn new(texts: &[&str]) -> Self {
        let mut inf = vec![0; 16 + texts.len() * 4];
        inf[..4].copy_from_slice(b"INF1");
        crate::put16(&mut inf, 8, texts.len() as u16);
        crate::put16(&mut inf, 10, 4);
        let mut dat = b"DAT1\0\0\0\0\0\0".to_vec();
        for (i, s) in texts.iter().enumerate() {
            let off = (dat.len() - 8) as u32;
            put32(&mut inf, 16 + i * 4, off);
            for c in s.encode_utf16() {
                dat.extend_from_slice(&c.to_le_bytes());
            }
            dat.extend_from_slice(&[0, 0]);
        }
        let mut b = vec![0; 32];
        b[..8].copy_from_slice(b"MESGbmg1");
        b[16] = 2;
        put32(&mut b, 12, 2);
        for mut section in [inf, dat] {
            while section.len() % 32 != 0 {
                section.push(0);
            }
            let len = section.len() as u32;
            put32(&mut section, 4, len);
            b.extend_from_slice(&section);
        }
        let len = b.len() as u32;
        put32(&mut b, 8, len);
        Self::parse(&b).expect("synthetic BMG constructor")
    }
    pub fn write(&self) -> Vec<u8> {
        if Self::parse(&self.raw).is_ok_and(|old| old.messages == self.messages) {
            return self.raw.clone();
        }
        let mut sections = self.sections.clone();
        let stride = u16le(&sections[self.inf], 10).unwrap_or(4) as usize;
        let mut data = b"DAT1\0\0\0\0\0\0".to_vec();
        for (i, m) in self.messages.iter().enumerate() {
            if 16 + i * stride + 4 > sections[self.inf].len() {
                continue;
            }
            put32(
                &mut sections[self.inf],
                16 + i * stride,
                (data.len() - 8) as u32,
            );
            for t in &m.tokens {
                match t {
                    Token::Text(s) => {
                        for c in s {
                            data.extend_from_slice(&c.to_le_bytes());
                        }
                    }
                    Token::Code(code) => data.extend_from_slice(code),
                }
            }
            data.extend_from_slice(&[0, 0]);
        }
        while !data.len().is_multiple_of(32) {
            data.push(0);
        }
        let len = data.len() as u32;
        put32(&mut data, 4, len);
        sections[self.dat] = data;
        let mut out = self.raw[..32].to_vec();
        for s in sections {
            out.extend_from_slice(&s);
        }
        let len = out.len() as u32;
        put32(&mut out, 8, len);
        out
    }
}
pub fn text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|t| match t {
            Token::Text(s) => String::from_utf16_lossy(s),
            Token::Code(_) => "〈code〉".into(),
        })
        .collect()
}
