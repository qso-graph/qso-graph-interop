<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Changelog

## [Unreleased]

### Fixed

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

- Rust workspace with the `qso-graph-udp` library, `qg-udp` listener and simulator, isolated
  decoder fuzzing, and token-based repository policy checks.
- Bidirectional WSJT-X 3.0.2 messages (the same protocol as 2.7.0) 0 through 16, schemas 1 through 3, optional historical
  tails, bounded Qt values, and graceful skipping of unknown types and newer schemas.
- Common logged-QSO, radio-state, decode, control, and lifecycle views that retain each original
  packet and its extensions.
- Independent wire fixtures, Python-reference sample vectors, edge cases, and CLI loopback tests.
- CI for Linux, macOS, and Windows, dependency checks, and fuzz smoke testing.
