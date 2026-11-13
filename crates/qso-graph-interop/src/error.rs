// SPDX-License-Identifier: GPL-3.0-or-later
//! Family-independent failure categories, with typed family causes.
#![forbid(unsafe_code)]
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Category {
    Truncated,
    Oversized,
    Malformed,
    UnsupportedVersion,
    InvalidValue,
}

/// Display strings are diagnostic, not stable values to parse.
/// An atomic message failure. A family cause can be recovered with `source().downcast_ref()`.
#[derive(Debug)]
#[non_exhaustive]
pub struct Error {
    category: Category,
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}
impl Error {
    pub fn category(&self) -> Category {
        self.category
    }
    pub fn new(category: Category, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
            source: None,
        }
    }
    pub fn family(category: Category, source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            category,
            message: source.to_string(),
            source: Some(Box::new(source)),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn StdError + 'static))
    }
}
pub(crate) fn invalid(message: &str) -> Error {
    Error::new(Category::InvalidValue, message)
}
pub(crate) fn oversized(message: &str) -> Error {
    Error::new(Category::Oversized, message)
}
