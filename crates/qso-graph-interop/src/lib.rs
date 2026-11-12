// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded sans-I/O protocol codecs. Callers own sockets, destinations and control policy.
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
pub mod base64;
mod catalog;
pub mod codec;
pub mod error;
pub mod json;
pub mod model;
pub use codec::Codec;
pub use error::{Category, Error};
pub use model::{Context, Event, Text};
#[cfg(feature = "conformance")]
pub mod conformance;
#[cfg(feature = "qgi")]
pub mod qgi;
#[cfg(feature = "wsjtx")]
pub mod wsjtx;
