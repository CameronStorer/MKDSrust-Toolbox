//! Project sources contain changed values only; unchanged game data stays in the ROM.
use crate::{session::hash, Error, Result};
use nitro_formats::{
    bmg::{Bmg, Token},
    graphics::Nclr,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum SourceToken {
    Text(String),
    Utf16(Vec<u16>),
    Code(Vec<u8>),
}
impl From<&Token> for SourceToken {
    fn from(t: &Token) -> Self {
        match t {
            Token::Text(t) => match String::from_utf16(t) {
                Ok(s) => Self::Text(s),
                Err(_) => Self::Utf16(t.clone()),
            },
            Token::Code(c) => Self::Code(c.clone()),
        }
    }
}
impl From<SourceToken> for Token {
    fn from(t: SourceToken) -> Self {
        match t {
            SourceToken::Text(t) => Self::Text(t.encode_utf16().collect()),
            SourceToken::Utf16(t) => Self::Text(t),
            SourceToken::Code(c) => Self::Code(c),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Changes {
    Palette {
        colors: BTreeMap<String, u16>,
    },
    Bmg {
        messages: BTreeMap<String, Vec<SourceToken>>,
    },
    BinaryPatch {
        bsdiff: Vec<u8>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    toolbox_edit: u32,
    source_sha256: String,
    changes: Changes,
}
fn wrap(original: &[u8], plain: Vec<u8>) -> Vec<u8> {
    if original.first() == Some(&0x10) {
        nitro_formats::recompress(original, &plain)
    } else {
        plain
    }
}
pub fn encode(original: &[u8], edited: &[u8]) -> Result<Vec<u8>> {
    let a = crate::preview::plain(original)?;
    let b = crate::preview::plain(edited)?;
    let changes = if let (Ok(old), Ok(new)) = (Nclr::parse(&a), Nclr::parse(&b)) {
        if old.colors.len() == new.colors.len() {
            Some(Changes::Palette {
                colors: old
                    .colors
                    .iter()
                    .zip(&new.colors)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, (_, b))| (i.to_string(), *b))
                    .collect(),
            })
        } else {
            None
        }
    } else if let (Ok(old), Ok(new)) = (Bmg::parse(&a), Bmg::parse(&b)) {
        if old.messages.len() == new.messages.len() {
            Some(Changes::Bmg {
                messages: old
                    .messages
                    .iter()
                    .zip(&new.messages)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, (_, b))| {
                        (
                            i.to_string(),
                            b.tokens.iter().map(SourceToken::from).collect(),
                        )
                    })
                    .collect(),
            })
        } else {
            None
        }
    } else {
        None
    };
    if let Some(changes) = changes {
        let bytes = serde_json::to_vec_pretty(&Source {
            toolbox_edit: 1,
            source_sha256: hash(original),
            changes,
        })?;
        if decode(original, &bytes).is_ok_and(|b| b == edited) {
            return Ok(bytes);
        }
    }
    let mut bsdiff = vec![];
    qbsdiff::Bsdiff::new(original, edited).compare(&mut bsdiff)?;
    Ok(serde_json::to_vec_pretty(&Source {
        toolbox_edit: 1,
        source_sha256: hash(original),
        changes: Changes::BinaryPatch { bsdiff },
    })?)
}
pub fn decode(original: &[u8], bytes: &[u8]) -> Result<Vec<u8>> {
    let source: Source = serde_json::from_slice(bytes)?;
    if source.toolbox_edit != 1 || source.source_sha256 != hash(original) {
        return Err(Error::Invalid(
            "edit source version or original-file hash does not match".into(),
        ));
    }
    match source.changes {
        Changes::Palette { colors } => {
            let mut p = Nclr::parse(&crate::preview::plain(original)?)?;
            for (i, c) in colors {
                let i = i
                    .parse::<usize>()
                    .map_err(|_| Error::Invalid("invalid palette source index".into()))?;
                let dest = p
                    .colors
                    .get_mut(i)
                    .ok_or_else(|| Error::Invalid("palette source index out of bounds".into()))?;
                *dest = c;
            }
            Ok(wrap(original, p.write()))
        }
        Changes::Bmg { messages } => {
            let mut p = Bmg::parse(&crate::preview::plain(original)?)?;
            for (i, tokens) in messages {
                let i = i
                    .parse::<usize>()
                    .map_err(|_| Error::Invalid("invalid message source index".into()))?;
                let m = p
                    .messages
                    .get_mut(i)
                    .ok_or_else(|| Error::Invalid("message source index out of bounds".into()))?;
                m.tokens = tokens.into_iter().map(Token::from).collect();
            }
            let bytes = p.write();
            Bmg::parse(&bytes)?;
            Ok(wrap(original, bytes))
        }
        Changes::BinaryPatch { bsdiff } => {
            let patch = qbsdiff::Bspatch::new(&bsdiff)?;
            if patch.hint_target_size() > 64 * 1024 * 1024 {
                return Err(Error::Invalid("edit source target exceeds 64 MiB".into()));
            }
            let mut out = vec![];
            patch.apply(original, std::io::Cursor::new(&mut out))?;
            Ok(out)
        }
    }
}
