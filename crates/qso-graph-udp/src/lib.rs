// SPDX-License-Identifier: GPL-3.0-or-later
//! Bidirectional, bounded codecs. Callers own sockets, destinations and control policy.
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub mod model;
pub mod wsjtx;
