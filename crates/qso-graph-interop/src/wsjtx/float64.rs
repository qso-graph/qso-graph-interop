// SPDX-License-Identifier: GPL-3.0-or-later
//! Native binary64 values use the neutral QGI-SER representation.
#![forbid(unsafe_code)]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub fn serialize<S: Serializer>(value: &f64, s: S) -> Result<S::Ok, S::Error> {
    crate::model::Binary64::from(*value).serialize(s)
}
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    crate::model::Binary64::deserialize(d).map(|v| v.value())
}
