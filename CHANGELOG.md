<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Changelog

## [Unreleased]

### Changed

- The release build checks that `qg-interop --version` reports the version being released.

## [0.2.0] - 2026-11-16

QG Interop 0.2.0 adds, since 0.1.0: IPv4 multicast for `listen` and the simulators;
PSK Reporter IPFIX (decoder, WSJT-X and fldigi submission encoders, `collect`, and a
simulator) and offline PSK Reporter MQTT capture decoding; live-peer captures from
WSJT-X 3.0.2, N1MM Logger+ 1.0.11480 and fldigi 4.2.13; `listen --human`; and
specification 0.2.0, which removes the reserved `js8call` family. WSJT-X support is
types 0–16 at the 3.0.2 general release (mapping revision 1).

### Removed

- WSJT-X 3.3 support from 0.2.0-beta.1 is not in 0.2.0: JTTY, InhibitStatus (type 17),
  TxInhibit (type 18) and the proposed JTTYText (type 19), with Annex B's provisional
  revisions 2 and 3, their vectors and fixtures, the beta6 and 3.3 pull-request-build
  UDP captures, and the simulator's `--mode JTTY`, `--inhibit-status` and
  `--jtty-text`. A release may not carry a
  provisional mapping, and both were pinned to pre-release or proposed WSJT-X source.
  WSJT-X 3.3 support returns in a later release after WSJT-X 3.3's general release.
  `listen --human` remains, with one generic line per event. The beta release gate
  and Annex B.8's marker rules remain for later betas.

### Added

- Every release attaches `qgi-spec-<version>.tar.gz`, the exact specification it
  implements, built by `git archive` from the release commit, listed in `SHA256SUMS.txt`
  and attested. The specification stays a draft until QG Interop 1.0 (`QGI-RI-0048`),
  when it becomes 1.0.0. New §10 states how it changes: by pull request, errata as PATCH
  releases, released versions immutable, the maintainer deciding adoption.

- Add a byte-exact N1MM Logger+ 1.0.11480 rotor capture: the `N1MMRotor` command
  N1MM sends to the rotor program on port 12040 (no XML declaration, no line
  breaks), decoded as an `unknown_type` skip beside 18 live broadcasts. Record that
  N1MM accepts and displays QG Interop's encoded Spectrum vectors and confirms kHz
  scope frequencies, and that External Callsign Lookup cannot be enabled in its
  Configurer. Annex D.8.1 answers Q10 and updates Q1 and Q6.

### Changed

- Close the issue #5 review gaps found after 0.2.0-beta.1: a MINOR release may not
  narrow an existing field's value domain (`QGI-GEN-0005`); an fldigi codec takes no
  method list (Annex E); every family's public failures now carry its own error as their
  source (`QGI-RI-0024`): PSK Reporter MQTT gains `MqttError`, PSK Reporter IPFIX wraps
  every `Codec`/`Profile` boundary and `encode_submission`, `recognize()` wraps
  projection failures in all six families, and `qgi::Error` is now public so callers can
  recover it;
  `QGI-RI-0012`, `0030` and `0042` now match the library (no transport features) and
  the enforced unsafe-code policy; `docs/NORMALIZATION.md` cites the requirements it
  describes.

### Fixed

- `qg-interop sim` (WSJT-X and N1MM Logger+) no longer drops a datagram that arrives as
  a receive times out on Windows: it polls a non-blocking socket instead of setting
  `SO_RCVTIMEO`, which leaves a Windows socket in an indeterminate state after a timeout.
  The N1MM simulator test's receiver does the same, which removes its intermittent
  Windows failure.

## [0.2.0-beta.1] - 2026-11-15

- Provisional mapping: `wsjtx/2`; pin: `tag:v3.3.0-beta6,tarball-sha256:cbbbfd213de2dabfb0d2bac626a3b2ebb9e7b1836bd2771f90d12800771694be`.
- Provisional mapping: `wsjtx/3`; pin: `repo:github.com/qso-graph/wsjtx,commit:53313c109194de1ce8e82e44f6846d8579645e56,patch-sha256:fb245ccea54033ace6b43fce6302bae03e1dc6518a08b455bdf080ef411679bf`.

- Add the byte-exact fldigi 4.2.13 live PSK Reporter IPFIX submission, LF transmitter
  log, SHA-256 provenance, and independent field oracle. Check receiver information,
  three PSK31 spots with literal zero-Hz frequencies and reception times, template
  IDs, evidence verification, and byte-exact fldigi submission encoding. Confirm
  Annex H's fldigi profile without codec changes and complete its QGI-RI-0063
  live evidence. Document sender setup, Spot gating, printable call delimiter,
  queue timing, and duplicate suppression separately from protocol requirements.
  Fix the fixture callsign scan to separate fldigi's variable fields from adjacent
  timestamps without discarding bytes; check lowercase `.txt` evidence too.
  Plant unauthorized calls, suffixes, mode text, and framing failures.

- Add byte-exact normal-operation WSJT-X 3.0.2 and 3.3.0-beta6 live FT8/IPFIX
  captures with SHA-256 provenance and independent field oracles. Verify all four
  submissions and 420 WSJT-X UDP messages, correlate six spots with Decode/Status
  messages, and check receiver-only template refreshes and exact submission encoding.
  Confirm Annex H's WSJT-X profiles without codec changes and mark live WSJT-X
  IPFIX evidence done for QGI-RI-0063. Exclude WSJT-X Decode scalar bytes from
  the recorder callsign scan while preserving text, ID, and extension checks;
  fix a Clippy error in the pinned-file storage check.

- Store byte-exact live WSJT-X, N1MM Logger+, and fldigi lab captures with SHA-256
  provenance, offline codec conformance tests, QSY/RadioInfo comparisons, and
  numbered Annex D/E findings. Preserve and test four truncated N1MM payloads;
  record fldigi's empty timeout, slow get_char_rates, and XML-RPC fault. Track
  live-peer evidence without claiming independent qgi captures. Confine the five
  owner-authorized FT8 calls to live fixtures and
  plant policy failures for other calls and locations. Existing mappings remain
  unchanged.

- Add PSK Reporter MQTT mapping revision 1 for offline captures, strict topic/payload
  agreement with capture-observed callsign `/` to topic `.` escaping, bounded JSON,
  shared spot and receiver-information events, a capture decoder, corrected capture
  vectors covering all 460 records, and MQTT decoder fuzzing. Reject ambiguous
  literal payload callsign periods and retain strict disagreement errors.

- Add provisional WSJT-X mapping revision 3 for proposed JTTYText type 19, with
  exact proposal pins, full-text updates, UTC starts, independent Windows/macOS
  captures, schema 2/3 vectors, listener human output, and `sim --jtty-text`.
- Permit provisional mappings only in exactly named beta pre-releases whose
  versioned CHANGELOG section lists every mapping and identical source pin;
  enforce this at all publication gates with planted refusal cases.

- Add PSK Reporter IPFIX mapping revision 1, bounded template decoding with record evidence, byte-exact WSJT-X and fldigi submission encoders, numeric unicast collector and simulator, source-derived vectors, and decoder fuzzing.


### Removed

- JS8Call family reservation and placeholder Annex C in specification 0.2.0.

### Changed

- Verify the packaged CLI against the current workspace library so an older
  registry crate at the development version cannot hide or reject new APIs.

- Advance QGI-SPEC-001 and generated conformance vectors to specification 0.2.0; retain the published 0.1.0 schema and add the 0.2.0 schema.

### Added

- Provisional WSJT-X mapping revision 2 pinned to `v3.3.0-beta6`: InhibitStatus
  snapshots, validated TxInhibit commands, source-verified JTTY Status and logging
  vectors, independent source fixtures, and JTTY / inhibit simulator snapshots. Preserve Decode mode markers.
- Offline verification uses cached cargo-vet and advisory data without fetching.
- Release guard for provisional family annexes and catalog entries, with planted
  failures that pass only after removing the provisional mark.

- IPv4 multicast receive and send on an explicit numeric local interface with
  `--interface`: shared-port `listen`, WSJT-X and N1MM simulators, and capture replay.
  Validate group and interface addresses, restrict multicast traffic to that interface,
  and test two listeners receiving the same stream in Linux, macOS, and Windows CI.

- Manual main-only release checks, signed CLI archives and C library bundles for
  four platforms, Linux packages, crates.io publication, provenance, and release
  setup instructions. Add local and CI release tests, package-content checks,
  Cargo packaging, and pinned workflow linting; keep embedded CLI inputs inside
  its package and distribute the FFI crate only as C library bundles.

- Complete the pre-publish public API review (API-1 through API-12): immutable typed
  event views; private typed options and per-call fldigi request association;
  canonical open, object-safe traits and validated external profile recognition;
  explicit stream progress and QGI carriage; bounded publication options and opaque
  native evidence; ordered nonempty command plans with response metadata;
  extensible payloads, immutable error categories, and value/equality contracts.
  Update CLI, FFI, fuzzing, conformance, tests, and documentation. Add the explicit
  `qgi_encode_event` C entry point while preserving existing ABI behavior and JSON.


- Run Rust FFI boundary tests under pinned-nightly Miri and the linked C caller under
  Rust AddressSanitizer plus C AddressSanitizer/UndefinedBehaviorSanitizer on
  Linux x86_64 GNU in verification and CI. Require both tools to catch isolated,
  cfg-gated heap overreads. Cover rejected lengths, stale handles, double frees,
  and round trips for every supported ABI family/transport.

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

- Verify the packaged CLI against the current workspace library so an older
  registry crate at the development version cannot hide or reject new APIs.

- Keep Miri focused on tiny C ABI inputs for every entry point and family,
  including multi-request fldigi output, ownership, invalid arguments, stream
  errors, and panic containment. Run vector sweeps and large stream frames only
  as ordinary tests; enable Miri isolation and retain strict provenance checks.
  Include system sanitizer runtime packages in ASan/UBSan installation hints.
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

- Preserve exported buffer pointer validity across registry insertion and removal
  by storing immutable shared allocations instead of moving unique `Box` owners
  after exporting their pointers. Add a registry-growth regression checked by Miri.
- Release contained panic payloads instead of leaking them, contain payload
  destructor panics, and test cleanup and handle invalidation under Miri.

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

- Provisional WSJT-X mapping revision 2 pinned to `v3.3.0-beta6`: InhibitStatus
  snapshots, validated TxInhibit commands, source-verified JTTY Status and logging
  vectors, independent source fixtures, and JTTY / inhibit simulator snapshots. Preserve Decode mode markers.
- Offline verification uses cached cargo-vet and advisory data without fetching.
- Release guard for provisional family annexes and catalog entries, with planted
  failures that pass only after removing the provisional mark.

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
