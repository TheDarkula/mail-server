use std::fmt::Display;

use store::{
    write::{BitmapFamily, DeserializeFrom, Operation, SerializeInto, ToBitmaps},
    Serialize, BM_TAG, TAG_STATIC, TAG_TEXT,
};
use utils::codec::leb128::{Leb128Iterator, Leb128Vec};

use crate::parser::{json::Parser, JsonObjectParser};

pub const SEEN: usize = 0;
pub const DRAFT: usize = 1;
pub const FLAGGED: usize = 2;
pub const ANSWERED: usize = 3;
pub const RECENT: usize = 4;
pub const IMPORTANT: usize = 5;
pub const PHISHING: usize = 6;
pub const JUNK: usize = 7;
pub const NOTJUNK: usize = 8;
pub const DELETED: usize = 9;
pub const FORWARDED: usize = 10;
pub const MDN_SENT: usize = 11;
pub const OTHER: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(untagged)]
pub enum Keyword {
    #[serde(rename(serialize = "$seen"))]
    Seen,
    #[serde(rename(serialize = "$draft"))]
    Draft,
    #[serde(rename(serialize = "$flagged"))]
    Flagged,
    #[serde(rename(serialize = "$answered"))]
    Answered,
    #[serde(rename(serialize = "$recent"))]
    Recent,
    #[serde(rename(serialize = "$important"))]
    Important,
    #[serde(rename(serialize = "$phishing"))]
    Phishing,
    #[serde(rename(serialize = "$junk"))]
    Junk,
    #[serde(rename(serialize = "$notjunk"))]
    NotJunk,
    #[serde(rename(serialize = "$deleted"))]
    Deleted,
    #[serde(rename(serialize = "$forwarded"))]
    Forwarded,
    #[serde(rename(serialize = "$mdnsent"))]
    MdnSent,
    Other(String),
}

impl JsonObjectParser for Keyword {
    fn parse(parser: &mut Parser<'_>) -> crate::parser::Result<Self>
    where
        Self: Sized,
    {
        let pos = parser.pos;
        if parser
            .next_unescaped()?
            .ok_or_else(|| parser.error_value())?
            == b'$'
        {
            let mut hash = 0;
            let mut shift = 0;

            while let Some(ch) = parser.next_unescaped()? {
                if shift < 128 {
                    hash |= (ch as u128) << shift;
                    shift += 8;
                } else {
                    break;
                }
            }

            match hash {
                0x6e65_6573 => return Ok(Keyword::Seen),
                0x0074_6661_7264 => return Ok(Keyword::Draft),
                0x6465_6767_616c_66 => return Ok(Keyword::Flagged),
                0x6465_7265_7773_6e61 => return Ok(Keyword::Answered),
                0x746e_6563_6572 => return Ok(Keyword::Recent),
                0x746e_6174_726f_706d_69 => return Ok(Keyword::Important),
                0x676e_6968_7369_6870 => return Ok(Keyword::Phishing),
                0x6b6e_756a => return Ok(Keyword::Junk),
                0x6b6e_756a_746f_6e => return Ok(Keyword::NotJunk),
                0x0064_6574_656c_6564 => return Ok(Keyword::Deleted),
                0x6465_6472_6177_726f_66 => return Ok(Keyword::Forwarded),
                0x746e_6573_6e64_6d => return Ok(Keyword::MdnSent),
                _ => (),
            }
        }

        if parser.is_eof || parser.skip_string() {
            Ok(Keyword::Other(
                String::from_utf8_lossy(parser.bytes[pos..parser.pos - 1].as_ref()).into_owned(),
            ))
        } else {
            Err(parser.error_unterminated())
        }
    }
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keyword::Seen => write!(f, "$seen"),
            Keyword::Draft => write!(f, "$draft"),
            Keyword::Flagged => write!(f, "$flagged"),
            Keyword::Answered => write!(f, "$answered"),
            Keyword::Recent => write!(f, "$recent"),
            Keyword::Important => write!(f, "$important"),
            Keyword::Phishing => write!(f, "$phishing"),
            Keyword::Junk => write!(f, "$junk"),
            Keyword::NotJunk => write!(f, "$notjunk"),
            Keyword::Deleted => write!(f, "$deleted"),
            Keyword::Forwarded => write!(f, "$forwarded"),
            Keyword::MdnSent => write!(f, "$mdnsent"),
            Keyword::Other(s) => write!(f, "{}", s),
        }
    }
}

impl BitmapFamily for Keyword {
    fn family(&self) -> u8 {
        if matches!(self, Keyword::Other(_)) {
            BM_TAG | TAG_TEXT
        } else {
            BM_TAG | TAG_STATIC
        }
    }
}

impl ToBitmaps for Keyword {
    fn to_bitmaps(&self, ops: &mut Vec<store::write::Operation>, field: u8, set: bool) {
        ops.push(Operation::Bitmap {
            family: self.family(),
            field,
            key: self.serialize(),
            set,
        });
    }
}

impl Serialize for Keyword {
    fn serialize(self) -> Vec<u8> {
        match self {
            Keyword::Seen => vec![SEEN as u8],
            Keyword::Draft => vec![DRAFT as u8],
            Keyword::Flagged => vec![FLAGGED as u8],
            Keyword::Answered => vec![ANSWERED as u8],
            Keyword::Recent => vec![RECENT as u8],
            Keyword::Important => vec![IMPORTANT as u8],
            Keyword::Phishing => vec![PHISHING as u8],
            Keyword::Junk => vec![JUNK as u8],
            Keyword::NotJunk => vec![NOTJUNK as u8],
            Keyword::Deleted => vec![DELETED as u8],
            Keyword::Forwarded => vec![FORWARDED as u8],
            Keyword::MdnSent => vec![MDN_SENT as u8],
            Keyword::Other(string) => string.into_bytes(),
        }
    }
}

impl Serialize for &Keyword {
    fn serialize(self) -> Vec<u8> {
        match self {
            Keyword::Seen => vec![SEEN as u8],
            Keyword::Draft => vec![DRAFT as u8],
            Keyword::Flagged => vec![FLAGGED as u8],
            Keyword::Answered => vec![ANSWERED as u8],
            Keyword::Recent => vec![RECENT as u8],
            Keyword::Important => vec![IMPORTANT as u8],
            Keyword::Phishing => vec![PHISHING as u8],
            Keyword::Junk => vec![JUNK as u8],
            Keyword::NotJunk => vec![NOTJUNK as u8],
            Keyword::Deleted => vec![DELETED as u8],
            Keyword::Forwarded => vec![FORWARDED as u8],
            Keyword::MdnSent => vec![MDN_SENT as u8],
            Keyword::Other(string) => string.as_bytes().to_vec(),
        }
    }
}

impl SerializeInto for Keyword {
    fn serialize_into(&self, buf: &mut Vec<u8>) {
        match self {
            Keyword::Seen => buf.push(SEEN as u8),
            Keyword::Draft => buf.push(DRAFT as u8),
            Keyword::Flagged => buf.push(FLAGGED as u8),
            Keyword::Answered => buf.push(ANSWERED as u8),
            Keyword::Recent => buf.push(RECENT as u8),
            Keyword::Important => buf.push(IMPORTANT as u8),
            Keyword::Phishing => buf.push(PHISHING as u8),
            Keyword::Junk => buf.push(JUNK as u8),
            Keyword::NotJunk => buf.push(NOTJUNK as u8),
            Keyword::Deleted => buf.push(DELETED as u8),
            Keyword::Forwarded => buf.push(FORWARDED as u8),
            Keyword::MdnSent => buf.push(MDN_SENT as u8),
            Keyword::Other(string) => {
                buf.push_leb128(OTHER + string.len());
                if !string.is_empty() {
                    buf.extend_from_slice(string.as_bytes())
                }
            }
        }
    }
}

impl DeserializeFrom for Keyword {
    fn deserialize_from(bytes: &mut std::slice::Iter<'_, u8>) -> Option<Self> {
        match bytes.next_leb128::<usize>()? {
            SEEN => Some(Keyword::Seen),
            DRAFT => Some(Keyword::Draft),
            FLAGGED => Some(Keyword::Flagged),
            ANSWERED => Some(Keyword::Answered),
            RECENT => Some(Keyword::Recent),
            IMPORTANT => Some(Keyword::Important),
            PHISHING => Some(Keyword::Phishing),
            JUNK => Some(Keyword::Junk),
            NOTJUNK => Some(Keyword::NotJunk),
            DELETED => Some(Keyword::Deleted),
            FORWARDED => Some(Keyword::Forwarded),
            MDN_SENT => Some(Keyword::MdnSent),
            other => {
                let len = other - OTHER;
                let mut keyword = Vec::with_capacity(len);
                for _ in 0..len {
                    keyword.push(*bytes.next()?);
                }
                Some(Keyword::Other(String::from_utf8(keyword).ok()?))
            }
        }
    }
}