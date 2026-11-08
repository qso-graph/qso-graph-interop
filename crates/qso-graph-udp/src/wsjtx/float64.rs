// SPDX-License-Identifier: GPL-3.0-or-later
//! JSON keeps finite numbers numeric and represents nonfinite IEEE values by bits.
#![forbid(unsafe_code)]
use serde::{
    Deserializer, Serializer,
    de::{Error, Visitor},
};

pub fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if value.is_finite() {
        serializer.serialize_f64(*value)
    } else {
        serializer.serialize_str(&format!("ieee754:{:016x}", value.to_bits()))
    }
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    struct FloatVisitor;
    impl<'de> Visitor<'de> for FloatVisitor {
        type Value = f64;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a number or ieee754: followed by 16 hex digits")
        }
        fn visit_f64<E: Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }
        fn visit_i64<E: Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_u64<E: Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_str<E: Error>(self, v: &str) -> Result<f64, E> {
            let bits = v
                .strip_prefix("ieee754:")
                .filter(|s| s.len() == 16)
                .ok_or_else(|| E::custom("invalid IEEE value"))?;
            u64::from_str_radix(bits, 16)
                .map(f64::from_bits)
                .map_err(E::custom)
        }
    }
    deserializer.deserialize_any(FloatVisitor)
}
