<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Changelog

## [Unreleased]

### Added

- Implementable draft core and WSJT-X profile for QGI-SPEC-001: Annexes A, B and F,
  independent conformance vectors and their deterministic generator; record issue #5 specification
  decisions and the first-release delivery order.

- Draft specification `spec/QGI-SPEC-001.md` (QG Interop, language-neutral, as ADIF is) and
  `spec/QGI-RI-001.md` (this library as its reference implementation, and the CLI, simulator and
  QSO Graph Desktop as its proofs). Initial review items are tracked in #5.

### Changed

- Revise QGI-SPEC-001 after the fourth review: composable `via` carriage, additive mapping
  compatibility and verification outcomes, open reader schema and strict producer validation,
  radio labels and provisional SO2R fields, explicit WSJT-X command effects and output budgets,
  multicast interface selection, receiver guidance, numbered rules and expanded vectors.

- Add the seventh draft family, `qgi`, with bounded JSON/NDJSON event carriage, nested native
  evidence and conformance vectors; close producer conformance. Specify all-family simulator
  playback and service responses, local PSK Reporter test services, the complete 1.0 class gate,
  program/service scope boundaries, published IPFIX submitter obligations, and C header generation
  with foreign-caller contracts remaining targets. No Rust implementation alignment is claimed.

- Revise QGI-SPEC-001 for commands built from fields, shared operations, MQTT publish envelopes,
  zero-or-more events and bounded template sessions; add five family placeholders and the
  informative Annex Z design check, result events, and command/schema regression vectors.
  The first release now covers six protocol families; implementation alignment remains separate.

- Renamed to QG Interop before its first release: the library is `qso-graph-interop`, the CLI
  package `qso-graph-interop-cli` and the command `qg-interop`. Nothing was published under the
  old names, so there is no alias.

### Fixed

- Require a supported native version for specification verification, enforce carriage parser
  bounds after appending `via`, and validate preserved newer mapping revisions with the reader
  profile while checking the new hop strictly. Add regression vectors and clarify that applications
  scope receiver instance labels by transport or session before associating receivers.

- Apply CLI process and DNS restrictions to package Rust sources outside `src/`, with planted
  external binary and build script checks.

- Preserve neutral client IDs and unknown direction in the common model; resolve Clear and Close
  only with caller context. Keep Reply and Configure target IDs separate from source identity.
  Support generic native families and request/response correlation for future protocol adapters.
- Preserve finite binary64 values through JSON with float_roundtrip, boundary/random regressions,
  and packet/common-model JSON fuzz coverage.
- Give independent fixtures distinct same-type sentinels and multiple boolean patterns; assert
  normalized fields and permanently check the review's paired Status Rx/Tx mutation.
- Reject CLI process aliases and alternative standard-library access. Confine socket operations
  to an audited numeric-address interface, with planted DNS and interface-policy failures.
- Align policy wording with enforcement: first-party Rust forbids unsafe and macro definitions;
  crates.io dependencies may use both, with native-code and supply-chain checks.
- Log simulated WSJT-X transmit frequency as dial plus TX offset, with checked arithmetic.
- Escape bidi controls, Unicode line separators, and selected invisible format controls in human
  output while preserving wire and JSON data.

### Added

- Support the WSJT-X UDP protocol through a single protocol API.

- Readable, terminal-safe listener output, structured JSON Lines, and UTC conversion helpers.

- Rust workspace with the `qso-graph-interop` library, `qg-interop` listener and simulator, isolated
  decoder fuzzing, and token-based repository policy checks.
- Bidirectional WSJT-X 3.0.2 messages (the same protocol as 2.7.0) 0 through 16, schemas 1 through 3, optional historical
  tails, bounded Qt values, and graceful skipping of unknown types and newer schemas.
- Common logged-QSO, radio-state, decode, control, and lifecycle views that retain each original
  packet and its extensions.
- Independent wire fixtures, Python-reference sample vectors, edge cases, and CLI loopback tests.
- CI for Linux, macOS, and Windows, dependency checks, and fuzz smoke testing.
