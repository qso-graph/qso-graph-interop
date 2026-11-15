// SPDX-License-Identifier: GPL-3.0-or-later
//! RFC 7011 wire primitives. Lengths are admitted before slices or allocations.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Field {
    pub pen: u32,
    pub id: u16,
    pub len: u16,
}
#[derive(Clone, Debug)]
pub(super) struct Template {
    pub bytes: Vec<u8>,
    pub fields: Vec<Field>,
    pub options: bool,
    pub scope: u16,
    pub refreshed: u64,
}
impl Template {
    pub fn minimum(&self) -> usize {
        self.fields
            .iter()
            .map(|f| {
                if f.len == 65535 {
                    1
                } else {
                    usize::from(f.len)
                }
            })
            .sum()
    }
}
pub(super) struct Cursor<'a> {
    pub bytes: &'a [u8],
    pub pos: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    pub fn left(&self) -> usize {
        self.bytes.len() - self.pos
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if n > self.left() {
            return Err(failure(Category::Truncated, "truncated field or record"));
        }
        let start = self.pos;
        self.pos += n;
        Ok(&self.bytes[start..self.pos])
    }
    pub fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().expect("two")))
    }
    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().expect("four")))
    }
}
pub(super) fn templates(
    bytes: &[u8],
    options: bool,
    time: u64,
) -> Result<Vec<(u16, Option<Template>)>, Error> {
    let mut c = Cursor::new(bytes);
    let mut result = Vec::new();
    while c.left() >= 4 {
        let start = c.pos;
        let id = c.u16()?;
        let count = usize::from(c.u16()?);
        if count == 0 {
            if id < 256 && id != if options { 3 } else { 2 } {
                return Err(failure(Category::Malformed, "invalid withdrawal ID"));
            }
            // RFC 7011 §8.4: ignore withdrawals over UDP, including all-template withdrawals.
            result.push((id, None));
            continue;
        }
        if id < 256 {
            return Err(failure(Category::Malformed, "template ID below 256"));
        }
        if count > MAX_FIELDS {
            return Err(oversized("IPFIX template field limit"));
        }
        let scope = if options { c.u16()? } else { 0 };
        if usize::from(scope) > count {
            return Err(failure(
                Category::Malformed,
                "options scope exceeds field count",
            ));
        }
        let mut fields = Vec::with_capacity(count);
        for _ in 0..count {
            let raw = c.u16()?;
            let len = c.u16()?;
            let pen = if raw & 0x8000 != 0 { c.u32()? } else { 0 };
            fields.push(Field {
                pen,
                id: raw & 0x7fff,
                len,
            });
        }
        if options && scope == 0 && !legacy_fldigi(&fields) {
            return Err(failure(Category::Malformed, "zero options scope"));
        }
        let template = Template {
            bytes: bytes[start..c.pos].to_vec(),
            fields,
            options,
            scope,
            refreshed: time,
        };
        if template.minimum() == 0 {
            return Err(failure(Category::Malformed, "zero-length records"));
        }
        result.push((id, Some(template)));
    }
    // Nonzero padding is retained (RFC 7011 §9.1), and diagnosed by the collector.
    Ok(result)
}
fn legacy_fldigi(fields: &[Field]) -> bool {
    let long = [(2, 65535), (4, 65535), (12, 8), (8, 65535), (9, 65535)];
    let short = &long[..3];
    [long.as_slice(), short].iter().any(|layout| {
        fields.len() == layout.len()
            && fields
                .iter()
                .zip(*layout)
                .all(|(f, (id, len))| f.pen == PEN && f.id == *id && f.len == *len)
    })
}
pub(super) fn record<'a>(c: &mut Cursor<'a>, template: &Template) -> Result<Vec<&'a [u8]>, Error> {
    let mut values = Vec::with_capacity(template.fields.len());
    for f in &template.fields {
        let len = if f.len == 65535 {
            let first = c.take(1)?[0];
            if first == 255 {
                usize::from(c.u16()?)
            } else {
                usize::from(first)
            }
        } else {
            usize::from(f.len)
        };
        values.push(c.take(len)?);
    }
    Ok(values)
}
pub(super) fn uint(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() || bytes.len() > 8 {
        return None;
    }
    Some(bytes.iter().fold(0, |n, b| (n << 8) | u64::from(*b)))
}
pub(super) fn signed(bytes: &[u8]) -> Option<i64> {
    let n = uint(bytes)?;
    let shift = 64 - bytes.len() * 8;
    Some(((n << shift) as i64) >> shift)
}
pub(super) fn pad_set(id: u16, data: &[u8]) -> Vec<u8> {
    let length = (data.len() + 7) & !3;
    let mut bytes = Vec::with_capacity(length);
    bytes.extend(id.to_be_bytes());
    bytes.extend((length as u16).to_be_bytes());
    bytes.extend(data);
    bytes.resize(length, 0);
    bytes
}
