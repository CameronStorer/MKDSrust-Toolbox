//! Indexed names, preserving BMG escape codes and the user's language selection.
use crate::{project::Project, session::RomSession, Error, Result};
use nitro_formats::bmg::{Bmg, Token};
use serde::Serialize;
use specta::Type;
#[derive(Clone, Debug, Serialize, Type)]
pub struct Name {
    pub group: String,
    pub path: String,
    pub index: u32,
    pub text: String,
}
pub fn list(p: Option<&Project>, s: &RomSession) -> Result<Vec<Name>> {
    let mut out = vec![];
    for n in s.nodes.iter().filter(|n| {
        n.format == "bmg"
            && (n.path.ends_with("/common.bmg") || n.path.ends_with("/kart_select.bmg"))
    }) {
        let bytes = match p {
            Some(p) => p.bytes(s, &n.path)?,
            None => s.file(&n.path)?,
        };
        let b = Bmg::parse(&crate::preview::plain(bytes)?)?;
        for (i, m) in b.messages.iter().enumerate() {
            let group = if n.path.ends_with("/common.bmg") {
                match i {
                    37..=44 => "Cups",
                    45..=76 => "Courses",
                    _ => continue,
                }
            } else {
                "Characters & karts"
            };
            let text = m
                .tokens
                .iter()
                .map(|t| match t {
                    Token::Text(s) => String::from_utf16_lossy(s),
                    Token::Code(_) => " ◇ ".into(),
                })
                .collect::<String>();
            out.push(Name {
                group: group.into(),
                path: n.path.clone(),
                index: i as u32,
                text,
            });
        }
    }
    Ok(out)
}
pub fn rename(p: &mut Project, s: &RomSession, path: &str, index: usize, text: &str) -> Result<()> {
    if text.encode_utf16().count() > 128 || text.contains('\0') {
        return Err(Error::Invalid(
            "name exceeds 128 UTF-16 units or contains NUL".into(),
        ));
    }
    let original = p.bytes(s, path)?;
    let mut b = Bmg::parse(&crate::preview::plain(original)?)?;
    let m = b
        .messages
        .get_mut(index)
        .ok_or_else(|| Error::Invalid("name index out of bounds".into()))?;
    if m.tokens.iter().any(|t| matches!(t, Token::Code(_))) {
        return Err(Error::Invalid(
            "this name includes special codes; use the Text editor to preserve their positions"
                .into(),
        ));
    }
    m.tokens = vec![Token::Text(text.encode_utf16().collect())];
    p.edit(path, b.write(), "Rename", s)
}
