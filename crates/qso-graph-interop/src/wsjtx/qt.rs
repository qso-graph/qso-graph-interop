// SPDX-License-Identifier: GPL-3.0-or-later
//! Qt 5 stream values, preserving wire data without consulting local time zones.
#![forbid(unsafe_code)]

use super::{Error, MAX_DATAGRAM, MAX_ID};
use serde::{Deserialize, Serialize};

/// A UTF-8 QByteArray on the wire. Invalid UTF-8 remains available as bytes.
/// `None` is null; `Some(vec![])` is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utf8 {
    pub bytes: Option<Vec<u8>>,
}

impl From<&str> for Utf8 {
    fn from(value: &str) -> Self {
        Self {
            bytes: Some(value.as_bytes().to_vec()),
        }
    }
}
impl From<String> for Utf8 {
    fn from(value: String) -> Self {
        Self {
            bytes: Some(value.into_bytes()),
        }
    }
}
impl Utf8 {
    pub fn as_str(&self) -> Option<&str> {
        self.bytes
            .as_deref()
            .and_then(|b| std::str::from_utf8(b).ok())
    }
}

/// QString's UTF-16 code units, including unpaired surrogates. Length is in bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QString(pub Option<Vec<u16>>);
impl From<&str> for QString {
    fn from(value: &str) -> Self {
        Self(Some(value.encode_utf16().collect()))
    }
}

/// Raw milliseconds since midnight. `u32::MAX` is Qt's null/invalid time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QTime(pub u32);
impl Default for QTime {
    fn default() -> Self {
        Self(u32::MAX)
    }
}

/// A Qt date/time, without lossy conversion to an epoch or the host time zone.
/// `i64::MIN` is Qt's null Julian day. Schema 1 stores a UTC clock value with
/// the original time spec (Qt's documented bug); no offset/zone bytes follow it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QDateTime {
    pub julian_day: i64,
    pub time: QTime,
    /// 0 local, 1 UTC, 2 offset, 3 named/custom time zone.
    pub time_spec: u8,
    pub offset_seconds: Option<i32>,
    pub time_zone: Option<TimeZone>,
}
impl Default for QDateTime {
    fn default() -> Self {
        Self {
            julian_day: i64::MIN,
            time: QTime::default(),
            time_spec: 0,
            offset_seconds: None,
            time_zone: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeZone {
    Iana(QString),
    /// Qt's `OffsetFromUtc` serialization, including every custom-zone field.
    Custom {
        id: QString,
        offset_seconds: i32,
        name: QString,
        abbreviation: QString,
        country: i32,
        comment: QString,
    },
}

/// QColor's spec byte and five 16-bit channels (including padding).
/// Spec 0 is invalid, 1 RGB, 2 HSV, 3 CMYK, 4 HSL. Channels are kept verbatim.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QColor {
    pub spec: u8,
    pub channels: [u16; 5],
}

pub(crate) struct Reader<'a> {
    pub data: &'a [u8],
    pub at: usize,
    pub schema: u32,
}
impl<'a> Reader<'a> {
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Length)?;
        let bytes = self
            .data
            .get(self.at..end)
            .ok_or(Error::Truncated { offset: self.at })?;
        self.at = end;
        Ok(bytes)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?.try_into().map_err(|_| Error::Length)
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.at
    }
    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }
    pub fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.array()?))
    }
    pub fn i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_be_bytes(self.array()?))
    }
    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_bits(self.u64()?))
    }
    pub fn boolean(&mut self) -> Result<bool, Error> {
        Ok(self.u8()? != 0)
    }
    pub fn qtime(&mut self) -> Result<QTime, Error> {
        Ok(QTime(self.u32()?))
    }
    pub fn utf8(&mut self) -> Result<Utf8, Error> {
        self.bytes(MAX_DATAGRAM)
    }
    pub fn id(&mut self) -> Result<Utf8, Error> {
        self.bytes(MAX_ID)
    }
    fn bytes(&mut self, limit: usize) -> Result<Utf8, Error> {
        let n = self.u32()?;
        if n == u32::MAX {
            return Ok(Utf8 { bytes: None });
        }
        let n = usize::try_from(n).map_err(|_| Error::Length)?;
        if n > limit {
            return Err(Error::Length);
        }
        Ok(Utf8 {
            bytes: Some(self.take(n)?.to_vec()),
        })
    }
    pub fn qstring(&mut self) -> Result<QString, Error> {
        let n = self.u32()?;
        if n == u32::MAX {
            return Ok(QString(None));
        }
        let n = usize::try_from(n).map_err(|_| Error::Length)?;
        if n > MAX_DATAGRAM || n % 2 != 0 {
            return Err(Error::Length);
        }
        let bytes = self.take(n)?;
        Ok(QString(Some(
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .collect(),
        )))
    }
    pub fn qdatetime(&mut self) -> Result<QDateTime, Error> {
        let julian_day = self.i64()?;
        let time = self.qtime()?;
        let time_spec = self.u8()?;
        if time_spec > 3 {
            return Err(Error::InvalidTimeSpec(time_spec));
        }
        let mut value = QDateTime {
            julian_day,
            time,
            time_spec,
            offset_seconds: None,
            time_zone: None,
        };
        if self.schema >= 2 {
            match time_spec {
                2 => value.offset_seconds = Some(self.i32()?),
                3 => value.time_zone = Some(self.timezone()?),
                _ => {}
            }
        }
        Ok(value)
    }
    fn timezone(&mut self) -> Result<TimeZone, Error> {
        let id = self.qstring()?;
        if id != QString::from("OffsetFromUtc") {
            return Ok(TimeZone::Iana(id));
        }
        Ok(TimeZone::Custom {
            id: self.qstring()?,
            offset_seconds: self.i32()?,
            name: self.qstring()?,
            abbreviation: self.qstring()?,
            country: self.i32()?,
            comment: self.qstring()?,
        })
    }
    pub fn qcolor(&mut self) -> Result<QColor, Error> {
        Ok(QColor {
            spec: self.u8()?,
            channels: [
                self.u16()?,
                self.u16()?,
                self.u16()?,
                self.u16()?,
                self.u16()?,
            ],
        })
    }
    pub fn optional<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        if self.remaining() == 0 {
            Ok(None)
        } else {
            read(self).map(Some)
        }
    }
}

pub(crate) struct Writer {
    pub data: Vec<u8>,
    pub schema: u32,
    pub omitted: bool,
}
impl Writer {
    pub fn new(schema: u32) -> Self {
        Self {
            data: Vec::new(),
            schema,
            omitted: false,
        }
    }
    pub fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > MAX_DATAGRAM.saturating_sub(self.data.len()) {
            return Err(Error::Length);
        }
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    pub fn u8(&mut self, v: u8) -> Result<(), Error> {
        self.put(&[v])
    }
    pub fn u16(&mut self, v: u16) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn u32(&mut self, v: u32) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn u64(&mut self, v: u64) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn i32(&mut self, v: i32) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn i64(&mut self, v: i64) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn f64(&mut self, v: f64) -> Result<(), Error> {
        self.u64(v.to_bits())
    }
    pub fn boolean(&mut self, v: bool) -> Result<(), Error> {
        self.u8(u8::from(v))
    }
    pub fn qtime(&mut self, v: &QTime) -> Result<(), Error> {
        self.u32(v.0)
    }
    pub fn utf8(&mut self, v: &Utf8) -> Result<(), Error> {
        match &v.bytes {
            None => self.u32(u32::MAX),
            Some(b) => {
                let n = u32::try_from(b.len()).map_err(|_| Error::Length)?;
                self.u32(n)?;
                self.put(b)
            }
        }
    }
    pub fn qstring(&mut self, v: &QString) -> Result<(), Error> {
        match &v.0 {
            None => self.u32(u32::MAX),
            Some(units) => {
                let n = units.len().checked_mul(2).ok_or(Error::Length)?;
                if n > MAX_DATAGRAM {
                    return Err(Error::Length);
                }
                self.u32(u32::try_from(n).map_err(|_| Error::Length)?)?;
                for unit in units {
                    self.u16(*unit)?;
                }
                Ok(())
            }
        }
    }
    pub fn qdatetime(&mut self, v: &QDateTime) -> Result<(), Error> {
        if v.time_spec > 3 {
            return Err(Error::InvalidTimeSpec(v.time_spec));
        }
        let offset = self.schema >= 2 && v.time_spec == 2;
        let zone = self.schema >= 2 && v.time_spec == 3;
        if offset != v.offset_seconds.is_some() || zone != v.time_zone.is_some() {
            return Err(Error::InconsistentDateTime);
        }
        self.i64(v.julian_day)?;
        self.qtime(&v.time)?;
        self.u8(v.time_spec)?;
        if let Some(s) = v.offset_seconds {
            self.i32(s)?;
        }
        if let Some(z) = &v.time_zone {
            self.timezone(z)?;
        }
        Ok(())
    }
    fn timezone(&mut self, v: &TimeZone) -> Result<(), Error> {
        match v {
            TimeZone::Iana(id) => {
                if id == &QString::from("OffsetFromUtc") {
                    return Err(Error::InconsistentDateTime);
                }
                self.qstring(id)
            }
            TimeZone::Custom {
                id,
                offset_seconds,
                name,
                abbreviation,
                country,
                comment,
            } => {
                self.qstring(&QString::from("OffsetFromUtc"))?;
                self.qstring(id)?;
                self.i32(*offset_seconds)?;
                self.qstring(name)?;
                self.qstring(abbreviation)?;
                self.i32(*country)?;
                self.qstring(comment)
            }
        }
    }
    pub fn qcolor(&mut self, v: &QColor) -> Result<(), Error> {
        self.u8(v.spec)?;
        for c in v.channels {
            self.u16(c)?;
        }
        Ok(())
    }
    pub fn optional<T>(
        &mut self,
        v: &Option<T>,
        write: impl FnOnce(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match v {
            None => {
                self.omitted = true;
                Ok(())
            }
            Some(_) if self.omitted => Err(Error::NonContiguousTail),
            Some(value) => write(self, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qstring_null_empty_unicode_and_unpaired_surrogates() {
        for (value, wire) in [
            (QString(None), vec![255, 255, 255, 255]),
            (QString(Some(vec![])), vec![0, 0, 0, 0]),
            (
                QString(Some(vec![0x41, 0xd800, 0x00e9])),
                vec![0, 0, 0, 6, 0, 0x41, 0xd8, 0, 0, 0xe9],
            ),
        ] {
            let mut w = Writer::new(3);
            w.qstring(&value).unwrap();
            assert_eq!(w.data, wire);
            let mut r = Reader {
                data: &wire,
                at: 0,
                schema: 3,
            };
            assert_eq!(r.qstring().unwrap(), value);
            assert_eq!(r.remaining(), 0);
        }
    }

    #[test]
    fn qstring_malicious_odd_oversized_and_truncated_lengths() {
        for n in [1_u32, 3, 65_508, 0xffff_fffe] {
            let bytes = n.to_be_bytes();
            let mut r = Reader {
                data: &bytes,
                at: 0,
                schema: 3,
            };
            assert_eq!(r.qstring(), Err(Error::Length));
        }
        let bytes = [0, 0, 0, 2, 0];
        let mut r = Reader {
            data: &bytes,
            at: 0,
            schema: 3,
        };
        assert!(matches!(r.qstring(), Err(Error::Truncated { .. })));
        let mut w = Writer::new(3);
        assert_eq!(
            w.qstring(&QString(Some(vec![0; MAX_DATAGRAM / 2 + 1]))),
            Err(Error::Length)
        );
    }

    #[test]
    fn qcolor_invalid_and_all_color_spaces_preserve_channels() {
        for spec in 0..=5 {
            let color = QColor {
                spec,
                channels: [0x0123, 0x4567, 0x89ab, 0xcdef, 0xfedc],
            };
            let mut w = Writer::new(3);
            w.qcolor(&color).unwrap();
            assert_eq!(
                w.data,
                [
                    spec, 1, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc
                ]
            );
            let mut r = Reader {
                data: &w.data,
                at: 0,
                schema: 3,
            };
            assert_eq!(r.qcolor().unwrap(), color);
        }
    }

    #[test]
    fn noncanonical_qt_booleans_are_true_and_encode_as_one() {
        for b in [1_u8, 2, 127, 255] {
            let bytes = [b];
            let mut r = Reader {
                data: &bytes,
                at: 0,
                schema: 3,
            };
            assert!(r.boolean().unwrap());
            let mut w = Writer::new(3);
            w.boolean(true).unwrap();
            assert_eq!(w.data, [1]);
        }
    }
}

impl QTime {
    /// Milliseconds since midnight, or None for an invalid Qt time.
    pub fn milliseconds_since_midnight(self) -> Option<u32> {
        (self.0 < 86_400_000).then_some(self.0)
    }
}

impl QDateTime {
    /// Unix milliseconds when the wire value identifies an unambiguous UTC instant.
    /// Schema 1 stores the UTC clock even when its time spec says otherwise.
    pub fn unix_milliseconds(&self, schema: u32) -> Option<i64> {
        let offset = if schema == 1 || self.time_spec == 1 {
            0_i64
        } else if self.time_spec == 2 {
            i64::from(self.offset_seconds?)
        } else if let Some(TimeZone::Custom { offset_seconds, .. }) = &self.time_zone {
            i64::from(*offset_seconds)
        } else {
            return None;
        };
        self.julian_day
            .checked_sub(2_440_588)?
            .checked_mul(86_400_000)?
            .checked_add(i64::from(self.time.milliseconds_since_midnight()?))?
            .checked_sub(offset.checked_mul(1000)?)
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    #[test]
    fn utc_conversion_rejects_ambiguous_and_invalid_values() {
        let mut date = QDateTime {
            julian_day: 2_440_588,
            time: QTime(3_600_123),
            time_spec: 2,
            offset_seconds: Some(3600),
            time_zone: None,
        };
        assert_eq!(date.unix_milliseconds(3), Some(123));
        assert_eq!(date.unix_milliseconds(1), Some(3_600_123));
        date.time_spec = 0;
        assert_eq!(date.unix_milliseconds(3), None);
        date.time_spec = 1;
        date.time = QTime(u32::MAX);
        assert_eq!(date.unix_milliseconds(3), None);
        assert_eq!(Utf8::from("W1AW").as_str(), Some("W1AW"));
        assert_eq!(
            Utf8 {
                bytes: Some(vec![255])
            }
            .as_str(),
            None
        );
    }
}
