// SPDX-License-Identifier: GPL-3.0-or-later
//! Annex D admission for the shared bounded XML tokenizer.
#![forbid(unsafe_code)]
pub(super) use crate::xml::{Document, Node, legal};
pub(super) fn parse(bytes: &[u8]) -> Result<Document, crate::Error> {
    crate::xml::parse_with(
        bytes,
        crate::xml::XmlLimits {
            bytes: super::MAX_DATAGRAM,
            elements: 1024,
            declaration_eof_malformed: false,
        },
    )
}
