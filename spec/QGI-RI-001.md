<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI-RI-001: the QG Interop reference implementation and its proofs

**Status: draft, 2026-11-12;** review items in issue #5.
The reference implementation of `QGI-SPEC-001`, in Rust, and the three deliveries that prove it:
the `qg-interop` CLI, its simulator, and QSO Graph Desktop. Evidence is this repository at `de34f6b`
(0.1.0, unpublished).

**Delivery order (KI7MT, 2026-11-12):** WSJT-X, then JS8Call-improved, then N1MM Logger+ and fldigi.

**Status column:** `Met` is true at `de34f6b`, with the evidence named; `Target` is required and not yet
true, and is a finding until it is `Met`. Verify methods as `QGI-SPEC-001`, plus `Static` (checked
without running) and `Demo` (observable by running).

## 1. Principles

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0001` | **The implementation follows the specification, never the reverse.** Where they disagree, the implementation has a defect or the specification changes first, by its own revision. | Inspection | Target |
| `QGI-RI-0002` | **Every delivery, ours or another team's, uses the library's public API.** No application has a private path; when one needs something the library lacks, the library gains it for everyone. | Inspection | Target (Desktop decodes WSJT-X itself, `qso-graph-desktop/extensions/wsjtx`) |
| `QGI-RI-0003` | **Rust's own patterns, not new ones:** serde's split of a stable core and formats, sans-I/O codecs, `#[non_exhaustive]`, the Rust API Guidelines. A novel mechanism needs a written reason. | Inspection | Target |
| `QGI-RI-0004` | **The library holds protocol, applications hold policy** (`QGI-GEN-0004`). | Inspection | Met (`lib.rs`: "Callers own sockets, destinations and control policy") |

## 2. Packages

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0010` | **The library's core** (the event model, the family trait, the error type and the conformance kit) **depends on no family module**; transport helpers sit outside it so the core stays free of I/O. There is no separate `-core` crate: a third party uses `qso-graph-interop` with `default-features = false`. | Static | Target (`model.rs` imports `wsjtx::{QDateTime, QTime, Utf8}`) |
| `QGI-RI-0011` | **The four official families** may ship as feature-gated modules of one crate, using only the core's public API; a family moves to its own crate when its release pace needs it. | Static | Target |
| `QGI-RI-0012` | **Features:** one per family, `serde`, one per transport helper, `conformance`. Defaults decided before 1.0. | Test | Target (`serde` is mandatory today) |
| `QGI-RI-0013` | **Crate names chosen before the first crates.io publish** (permanent), fitting the specification's scope, which is no longer UDP alone. | Inspection | Met (D1) |
| `QGI-RI-0014` | **A separate `-ffi` crate** for the C ABI; the CLI is its own crate. | Static | CLI Met; FFI Target |

## 3. Library

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0020` | **The event model in Rust uses neutral core types** for time, frequency, text and identifiers (`QGI-MOD-0006`); a family's wire types stay in its native message. | Static | Target (Qt types in `Decode`, `LoggedQso`) |
| `QGI-RI-0021` | **Every public enum and growable struct is `#[non_exhaustive]`.** | Static | Target (none today) |
| `QGI-RI-0022` | **A family is a type implementing an open public trait**: decode to an outcome (event, skipped with reason, or error), encode where the family sends, declared framing (datagram, stream, request and reply) and limits. Stream families decode from a caller-owned buffer, as `tokio-util`'s `Decoder` does, without depending on an async runtime. Another crate can implement it without forking. | Test | Target |
| `QGI-RI-0023` | **Codecs do no I/O** (no socket, file, process, clock, environment or logging). | Static, Test | Met for WSJT-X by reading; `cargo xtask policy` proves the network and process part only, not file, clock, environment or logging |
| `QGI-RI-0024` | **Family errors are their own types, wrapped by a `#[non_exhaustive]` core error; `Display` is a message, not a debug dump.** | Test | Target (`wsjtx::Error` prints `Debug`) |
| `QGI-RI-0025` | **Families are compile-time dependencies;** nothing loads plugins or dynamic libraries. | Static | Met |
| `QGI-RI-0026` | **An optional strict-validation helper** applies rules an application chooses (encoding, character set, lengths); the library never applies it by itself. | Test | Target |

## 4. Transport helpers (optional)

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0030` | **Optional features, never required to use a codec:** UDP receive and send, TCP client, HTTP XML-RPC client. | Static | Target (the UDP listener lives in the CLI) |
| `QGI-RI-0031` | **Numeric addresses supplied by the caller;** no DNS, no discovery, no broadcast search; a receiver binds exactly what it is given and nothing defaults to every interface. | Test | Met in the CLI (default `127.0.0.1:2237`) |
| `QGI-RI-0032` | **Bounded buffers and queues; drops counted by reason;** no sender filtering, rate limiting or replies unless the caller configures them. | Test | Target |

## 5. Quality, stability and supply chain

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0040` | **Passes every `QGI-SPEC-001` test vector for its four families**, both directions where defined. | Test | Target (WSJT-X has fixtures; spec vectors not yet published) |
| `QGI-RI-0041` | **Every decoder fuzzed with arbitrary input, the event-JSON reader included; fuzz targets assert no panic, hang or unbounded allocation.** | Test | Met for the WSJT-X binary decoder (`fuzz/fuzz_targets/wsjtx.rs`); the JSON reader only sees JSON made from decoded packets: Target |
| `QGI-RI-0042` | **Unsafe code is forbidden in every crate except the declared `-ffi` boundary crate,** which denies it and allows each unsafe construct individually with a `SAFETY` comment; proved by a planted case in policy tooling. | Static | Met for both crates; the FFI rule Target |
| `QGI-RI-0043` | **Semantic versioning checked by `cargo-semver-checks`** against the last published release, in CI. | Static | Target |
| `QGI-RI-0044` | **Every public item documented with an example compiled as a test.** | Static | Target |
| `QGI-RI-0045` | **MSRV stated and tested.** | Test | Partly Met (`rust-version = "1.99"`) |
| `QGI-RI-0046` | **Dependencies from crates.io with compatible version ranges and a committed `Cargo.lock`, no native code in what ships, through `cargo-vet`, `cargo-deny` and `cargo-audit`.** Exact pins only as a recorded exception. | Static | Partly Met: exact pins today (an exception to record or remove); the fuzz build contains native code and is outside the shipped graph by design (`docs/DEPENDENCIES.md`) |
| `QGI-RI-0047` | **The conformance kit is exported** behind `conformance`: the vector runner, round trip, truncation at every prefix, limits, and a fuzz-target template, so a third-party family meets the same standard. | Test | Target |
| `QGI-RI-0048` | **1.0 after the core model and trait have carried a datagram family (WSJT-X), a stream family and a request-and-response family (fldigi) without change,** including partial and multiple frames, faults and unavailable methods. | Inspection | Target |

## 6. Other languages

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0050` | **A C ABI** for Python, .NET and Node, so the library is not Rust-only; every pointer and length checked; returned memory freed only by the library. | Test | Target |

## 7. The proofs

Each delivery proves the specification works from a different side. Each uses the public API only
(`QGI-RI-0002`).

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0060` | **CLI, `qg-interop`:** for each family, `listen` prints events (newline-delimited JSON per `QGI-SER-0004`, or a readable line), and `decode` turns a captured file into events. Proves codec and serialization against real traffic. | Test, Demo | WSJT-X `listen` exists, but prints its own format (`docs/user/listen-json.md`), not specification events: Target. `decode` of a file: Target. Not yet run against a live WSJT-X (`SECURITY.md`) |
| `QGI-RI-0061` | **Simulator, `qg-interop sim`:** for each family, plays a script or a capture as the program would send it, so applications are tested without a radio or the program. Replacing `qso-graph-sim` in QSO Graph Desktop's suites is a coordinated change to `QGD-INT-0024`. | Test | Met for WSJT-X from its built-in script; capture playback and replacing Desktop's simulator: Target |
| `QGI-RI-0062` | **QSO Graph Desktop:** consumes the library's events for live monitoring (Desktop 0.8.0's monitor panel and tabs), with Desktop's own policy (transports, senders, display rules) in Desktop. Proves the specification in a real application. | Test, Demo | Target |
| `QGI-RI-0063` | **Interoperability is shown against the real programs,** not only vectors and the simulator: captures from WSJT-X, JS8Call-improved, N1MM Logger+ and fldigi are decoded, and for sending directions, the program's response is observed. | Demo | Target |

## 8. Decisions

| # | Decision | State |
|---|---|---|
| D1 | **Names.** Product and specification **QG Interop**; repository and crate `qso-graph-interop`, with `-cli` (command `qg-interop`) and later `-ffi`; family modules `wsjtx`, `js8call`, `n1mm`, `fldigi`. No `-core` crate. | Decided, KI7MT, 2026-11-11 |
| D2 | **JS8Call:** the JS8Call-improved line, delivered right after WSJT-X; the original v2.3.1 is compared when the family is mapped (`QGI-JS8-0001`). | Decided, KI7MT, 2026-11-12 |
| D3 | **Licence:** GPL-3.0-or-later for the library, the specification, its schema and its vectors, like everything in QSO Graph. A closed-source program can implement the specification; it cannot link the library. | Decided, KI7MT, 2026-11-11 |
| D4 | **Round trip:** never lossy; the native message is kept and common-field canonicalizations are listed (`QGI-SPEC-001` `QGI-MOD-0007`). | Decided, KI7MT, 2026-11-12 |
| D5 | **Default features:** what a plain `cargo add` brings. | Open |
| D6 | **Where the specification lives:** here, in `spec/`, beside its reference implementation. It can move to its own repository if its release pace ever differs from the code's. | Proposed |
