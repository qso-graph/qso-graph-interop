<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Changelog

## [Unreleased]

### Added

- `qso-graph-interop-ffi` C ABI as shared and static libraries for WSJT-X, N1MM,
  fldigi, and QGI. Includes JSON outcomes, native command requests, re-encoding,
  caller-buffered stream decoding, owned buffers, thread-safe opaque handles,
  stable errors, and panic containment.
- Pinned cbindgen header generation and drift checks, Rust ABI vector tests,
  linked C consumer and C++ header checks in full verification and CI. Restrict
  unsafe Rust to the FFI package with checked safety comments and planted failures.

- `sim --fldigi [ADDRESS]`: loopback-only fldigi 4.2.13 XML-RPC stand-in serving all
  50 Annex E mappings, bundled empty-string results and unknown-method faults,
  mutable radio/contact state, timed queued transmission, and seeded synthetic RX text.
  Includes bounded concurrent HTTP connections, policy mutation checks, deterministic
  state tests, and serialized CLI poll/controller tests over loopback.

- Default-feature fldigi codec for Annex E revision 1, with all 50 mapped methods,
  typed XML-RPC values and faults, caller-supplied request association and correlation,
  ordered canonical command requests, and exact observed re-encoding.
- `decode --fldigi` with explicit response method or request file, and loopback-only
  `poll --fldigi` for state and RX text over bounded HTTP/1.1.
- Execute all 650 fldigi vectors, including 455 native re-encoding checks, and fuzz
  decoding, association, command encoding, and round trips from vector seeds in CI
  and `cargo xtask verify`.
- Reuse the bounded N1MM XML tokenizer with configurable family limits; preserve
  N1MM admission rules. Extend CLI policy with an audited loopback TCP boundary
  and planted DNS, external-connect, guard-removal, and timeout failures.

- Complete draft Annex E fldigi request/response profile pinned to released 4.2.13:
  all 174 method names, 50 common mappings, bounded XML-RPC values and faults,
  caller-supplied correlation, canonical ordered command bodies, and independent
  conformance vectors with strict schema and planted policy failures. Re-pins the
  source tarball and member hashes, audits every method change from 4.1.23 and
  manual/type discrepancies, and corrects sequence acknowledgment types.

- `qg-interop sim --n1mm` contest traffic with seeded synthetic contacts, UTC timing,
  periodic radio reports, SO2R handoffs, edits and deletions, spots, and matching scores.
  Adds target-filtered QSY reactions without acknowledgments, rate/speed/count/loop controls,
  independent synthetic expectations, loopback tests, and planted invariant failures.
- Default-feature N1MM Logger+ codec implementing Annex D revision 1: bounded XML,
  all documented roots, radio and SO2R state, exact frequency units, UTC timestamps,
  canonical frequency/spectrum commands, and byte-preserving observed re-encoding.
- `listen --n1mm` on loopback port 12060 and `decode --n1mm` for files or standard input.
- Manifest-driven execution of all 1,260 current N1MM vectors and 1,105 embedded re-encoding
  checks, independent radio evidence, and N1MM decoder/command fuzzing in CI and the verifier.
- Textual skip discriminators, N1MM-only event field validation, and raw negative-command
  instruction reading in the public conformance kit.

- Complete draft Annex D N1MM profile with dated source hashes, XML isolation policies,
  contact/radio/spot/score mappings, QSY and spectrum command writers,
  independent conformance vectors and numbered questions for live captures.
- Shared control radio selection and N1MM-only command argument schema capacity.

- Timed `qg-interop sim` ALL.TXT playback with an embedded synthetic sample, Python-compatible
  options, target-filtered Replay and WSJT-X state reactions, bounded local input, UDP NDJSON
  capture playback and qgi carriage, signal cleanup, and independent Desktop capture checks.

- Family-independent QGI-SPEC-001 event core, neutral values, open sans-I/O codec/profile traits,
  typed atomic errors, bounded event JSON, and exported conformance tools.
- WSJT-X revision 1 command construction, native consistency and re-encoding, all 17 types in
  schemas 1–3, radio labels, and qgi UDP/TCP carriage with hop admission and stream recovery.
- Manifest-driven acceptance of all 435 WSJT-X/qgi vectors and 94 embedded re-encoding checks;
  core-only feature tests, arbitrary event-JSON fuzzing, and CLI `decode` for files or standard input.
- `qg-interop listen` reports what it receives as coming from WSJT-X, so Close is a `lifecycle`
  event; `decode` keeps unknown direction.
- `cargo xtask verify` runs every check, including the fuzz smoke, and prints one pass/fail table.


- Implementable draft core and WSJT-X profile for QGI-SPEC-001: Annexes A, B and F,
  independent conformance vectors and their deterministic generator; record issue #5 specification
  decisions and the first-release delivery order.

- Draft specification `spec/QGI-SPEC-001.md` (QG Interop, language-neutral, as ADIF is) and
  `spec/QGI-RI-001.md` (this library as its reference implementation, and the CLI, simulator and
  QSO Graph Desktop as its proofs). Initial review items are tracked in #5.

### Changed

- Use Q-prefixed invented callsigns in simulators, samples, fixtures, specification vectors,
  and examples. Preserve distinct identities and byte lengths, retain the `N0CALL` station
  default, and guard against legacy synthetic calls with planted generator and policy failures.
- Map Annex D N1MM timestamps to UTC instants using pinned logging documentation.
  Preserve invalid timestamp text without rejecting the message, accept valid times
  as sent without clock checks or corrections, and cover contact, prior-contact,
  lookup, spot and score timestamps with independent vectors.
- Derive N1MM spot, QSY and Spectrum scope frequency units as kHz from the pinned
  examples. Map exact whole-Hz observations, encode integer-Hz commands as canonical
  decimal kHz without a unit option, and cover limits and 630 m/2200 m in vectors.
- Emit specification NDJSON from `listen`; replace the earlier CLI JSON projection.
- Use compatible dependency ranges and current workspace/fuzz lockfiles. Add optional pure Rust
  `sha2-const-stable` for conformance manifest hashes with a recorded local source review.
- Retire overlapping legacy protocol fixtures in favor of the specification vectors; retain the
  synthetic 120-message variation corpus, prefix checks, randomized floats and native helper tests.


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

- Skip and summarize ALL.TXT modes without established periods instead of aborting mixed-mode
  playback; retain skipped modes in the empty-selection error. Play released Q65 labels with
  recorded periods and submodes. Document all released mode periods and compare signal-handler
  dependencies against the package-wide native-source rule.

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
