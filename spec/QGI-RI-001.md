<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI-RI-001: the QG Interop reference implementation and its proofs

**Status: draft, 2026-11-12;** review items in issue #5.
The reference implementation of `QGI-SPEC-001`, in Rust, and the deliveries that prove it: the
`qg-interop` CLI, its simulator, the C ABI, and applications that use the library, QSO Graph Desktop
first. Evidence is this repository at `de34f6b` (0.1.0, unpublished).

**QG Interop is its own product, released on its own schedule.** Applications that use it depend on
its releases; no application's release is a condition of an Interop release.

**Delivery order:** WSJT-X, then N1MM Logger+, then fldigi, then JS8Call-improved,
followed by PSK Reporter MQTT and IPFIX, and `qgi` event carriage: seven official families. The first
release needs the first three (`QGI-RI-0049`); the others join it as they are ready, and all of them
gate 1.0 (`QGI-RI-0048`). “Peer” has the meaning defined in `QGI-GEN-0002`.

**Status column:** `Met` is true at `de34f6b`, with the evidence named; `Target` is required and not yet
true, and is a finding until it is `Met`. Verify methods as `QGI-SPEC-001`, plus `Static` (checked
without running) and `Demo` (observable by running).

## 1. Principles

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0001` | **The implementation follows the specification, never the reverse.** Where they disagree, the implementation has a defect or the specification changes first, by its own revision. | Inspection | Met for WSJT-X, N1MM, and qgi (1,695 specification vectors; independent manifests unchanged) |
| `QGI-RI-0002` | **Every delivery, ours or another team's, uses the library's public API.** No application has a private path; when one needs something the library lacks, the library gains it for everyone. | Inspection | Target (Desktop decodes WSJT-X itself, `qso-graph-desktop/extensions/wsjtx`) |
| `QGI-RI-0003` | **Rust's own patterns, not new ones:** serde's split of a stable core and formats, sans-I/O codecs, `#[non_exhaustive]`, the Rust API Guidelines. A novel mechanism needs a written reason. | Inspection | Met (public Codec/Profile, sans-I/O, serde, non-exhaustive types; bounded JSON rationale in docs/PROTOCOL.md) |
| `QGI-RI-0004` | **The library holds protocol, applications hold policy** (`QGI-GEN-0004`). | Inspection | Met (`lib.rs`: "Callers own sockets, destinations and control policy") |

## 2. Packages

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0010` | **The library's core** (the event model, the family trait, the error type and the conformance kit) **depends on no family module**; transport helpers sit outside it so the core stays free of I/O. There is no separate `-core` crate: a third party uses `qso-graph-interop` with `default-features = false`. | Static | Met (core imports no family; core-only build and tests) |
| `QGI-RI-0011` | **The seven official families** may ship as feature-gated modules of one crate, using only the core's public API; a family moves to its own crate when its release pace needs it. | Static | Partly Met (wsjtx, n1mm, and qgi feature-gated modules); four families Target |
| `QGI-RI-0012` | **Features:** one per family, `serde`, one per transport helper, `conformance`. Defaults decided before 1.0. | Test | Partly Met (wsjtx, n1mm, qgi, optional serde integration and conformance features); other families and transport helpers Target |
| `QGI-RI-0013` | **Crate names chosen before the first crates.io publish** (permanent), fitting the specification's scope, which is no longer UDP alone. | Inspection | Met (D1) |
| `QGI-RI-0014` | **A separate `-ffi` crate** for the C ABI; the CLI is its own crate. | Static | CLI Met; FFI Target |
| `QGI-RI-0015` | **The `-ffi` crate generates its C header with cbindgen;** the build and CI verify the generated header matches the ABI code so the two cannot drift. | Static, Test | Target |

## 3. Library

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0020` | **The event model in Rust uses neutral core types** for time, frequency, text and identifiers (`QGI-MOD-0006`); a family's wire types stay in its native message. | Static | Met (neutral integer, instant, time of day, text and binary64; Qt types confined to wsjtx) |
| `QGI-RI-0021` | **Every public enum and growable struct is `#[non_exhaustive]`.** | Static | Met (public enums and growable structs are non-exhaustive) |
| `QGI-RI-0022` | **A family is a type implementing an open public trait**: decode to an outcome (zero or more events and skip notices, or one atomic error), encode commands from fields where the family sends, separately re-encode observations from native evidence, and declare framing and limits (datagram, stream, request/reply, publish/subscribe or templated records). Stream families decode from a caller-owned buffer, as `tokio-util`'s `Decoder` does, without depending on an async runtime. Another crate can implement it without forking. | Test | Met (open Codec/Profile; external session-state implementation and caller-buffer stream tests) |
| `QGI-RI-0023` | **Codecs meet `QGI-SEC-0005`** (the specification owns the no-I/O rule). | Static, Test | Met for WSJT-X, N1MM, and qgi by inspection; cargo xtask policy checks network/process boundaries; N1MM tokenizer has no I/O or clock access |
| `QGI-RI-0024` | **Family errors are their own types, wrapped by a `#[non_exhaustive]` core error; `Display` is a message, not a debug dump.** | Test | Met (typed family sources in non-exhaustive core Error; prose Display) |
| `QGI-RI-0025` | **Families are compile-time dependencies;** nothing loads plugins or dynamic libraries. | Static | Met |
| `QGI-RI-0026` | **An optional strict-validation helper** applies rules an application chooses (encoding, character set, lengths); the library never applies it by itself. | Test | Target |

## 4. Transport helpers (optional)

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0030` | **Optional features, never required to use a codec:** UDP receive and send, TCP client, HTTP XML-RPC client. A UDP receive helper MAY join a multicast group only when the caller supplies both a numeric multicast group address and a numeric local interface address. It MUST validate those addresses, join only that interface, never all interfaces, and never choose a group itself. | Static | Target (the UDP listener lives in the CLI) |
| `QGI-RI-0031` | **Numeric addresses supplied by the caller;** no DNS, no discovery, no broadcast search; a receiver binds exactly what it is given and nothing defaults to every interface. | Test | Met in the CLI (numeric addresses; defaults 127.0.0.1:2237 for WSJT-X and 127.0.0.1:12060 for N1MM) |
| `QGI-RI-0032` | **Bounded buffers and queues; drops counted by reason;** no sender filtering, rate limiting or replies unless the caller configures them. | Test | Target |

## 5. Quality, stability and supply chain

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0040` | **Passes every `QGI-SPEC-001` test vector for its seven families**, both directions where defined. | Test | Partly Met (all 400 WSJT-X, 1,260 N1MM, and 35 qgi vectors, plus 1,199 embedded re-encoding checks); other families Target |
| `QGI-RI-0041` | **Every decoder fuzzed with arbitrary input, the event-JSON reader included; fuzz targets assert no panic, hang or unbounded allocation.** | Test | Met for WSJT-X, N1MM, qgi, and arbitrary event JSON (pinned-nightly public-API harnesses; N1MM commands and native round trips included); future families Target |
| `QGI-RI-0042` | **Unsafe code is forbidden in every crate except the declared `-ffi` boundary crate,** which denies it and allows each unsafe construct individually with a `SAFETY` comment; proved by a planted case in policy tooling. | Static | Met for both crates; the FFI rule Target |
| `QGI-RI-0043` | **Semantic versioning checked by `cargo-semver-checks`** against the last published release, in CI. | Static | Target |
| `QGI-RI-0044` | **Every public item documented with an example compiled as a test.** | Static | Target |
| `QGI-RI-0045` | **MSRV stated and tested.** | Test | Partly Met (`rust-version = "1.99"`) |
| `QGI-RI-0046` | **Dependencies from crates.io with compatible version ranges and a committed `Cargo.lock`, no native code in what ships, through `cargo-vet`, `cargo-deny` and `cargo-audit`.** Exact pins only as a recorded exception. | Static | Met (compatible ranges, committed lockfiles, pure Rust shipped graph, deny/vet/audit; optional conformance hash audit recorded) |
| `QGI-RI-0047` | **The conformance kit is exported** behind `conformance`: the vector runner, round trip, truncation at every prefix, limits, and a fuzz-target template, so a third-party family meets the same standard. | Test | Met (exported runner, hash, round trip, prefixes, limits and fuzz template) |
| `QGI-RI-0048` | **1.0 requires the core model and public codec interface to carry, without change, a release family of every protocol class and the `qgi` family:** datagram (WSJT-X and N1MM), stream (JS8Call), request/response (fldigi), publish/subscribe (PSK Reporter MQTT), templated (PSK Reporter IPFIX), and `qgi` event carriage over UDP and TCP. Evidence includes partial and multiple frames, faults, unavailable methods, retained publications, template/session boundaries and nested native evidence, as applicable. | Inspection, Test | Target |
| `QGI-RI-0049` | **The first release (0.x) requires, for WSJT-X, N1MM Logger+ and fldigi:** the codec passing every vector of its family in both directions where defined, `qg-interop listen` and `decode`, the simulator (`QGI-RI-0061`, `QGI-RI-0064`), and the C ABI (`QGI-RI-0050` to `QGI-RI-0054`), with the CLI and simulator shipped as signed standalone programs that need no Rust toolchain. Any further family shipped in it meets every requirement for that family. | Inspection | Target |

## 6. Other languages

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0050` | **A C ABI** for C and C++ programs, which can then use the library without changing their own design (WSJT-X and fldigi are C++), and through it Python, .NET and Node, so the library is not Rust-only; every pointer and length checked; returned memory freed only by the library. | Test | Target |
| `QGI-RI-0051` | **Foreign-caller ownership and lifetimes** are documented for every input, output and handle, including null/zero-length cases and the matching library release function. | Test, Inspection | Target |
| `QGI-RI-0052` | **Foreign-caller failures** use stable status codes and an explicit error-message lifetime; Rust panics never unwind across the ABI. | Test | Target |
| `QGI-RI-0053` | **Foreign-caller data representations** declare byte lengths, string encodings, absent/null/empty distinctions and ABI version negotiation. | Test, Inspection | Target |
| `QGI-RI-0054` | **Foreign-caller concurrency** declares handle thread safety, reentrancy and any callback lifetimes; callers are never required to infer these from Rust types. | Test, Inspection | Target |

## 7. The proofs

Each delivery proves the specification works from a different side. Each uses the public API only
(`QGI-RI-0002`).

The simulator is also a standalone product: developers in any language test their own programs
against the peers' real wire protocols, without linking the Rust library.

| ID | Requirement | Verify | Status |
|---|---|---|---|
| `QGI-RI-0060` | **CLI, `qg-interop`:** for each family, `listen` prints events (newline-delimited JSON per `QGI-SER-0004`, or a readable line), and `decode` turns a captured file into events. Proves codec and serialization against real traffic. | Test, Demo | Met for WSJT-X and N1MM listen and file/stdin decode specification NDJSON; other families and live-peer evidence Target |
| `QGI-RI-0061` | **Simulator playback, `qg-interop sim`:** for every family in the release, play scripted traffic and captures using the peer's real wire protocol as that peer would send it, including `qgi` UDP and TCP event carriage. | Test, Demo | Met for WSJT-X ALL.TXT/script and native UDP captures, plus qgi UDP event carriage; TCP and remaining family helpers: Target |
| `QGI-RI-0062` | **QSO Graph Desktop:** consumes the library's events for live monitoring (Desktop 0.8.0's monitor panel and tabs), with Desktop's own policy (transports, senders, display rules) in Desktop. Proves the specification in a real application. Evidence, not a release condition: Desktop depends on Interop, never the reverse. | Test, Demo | Target |
| `QGI-RI-0063` | **Interoperability is shown against real peers,** in addition to vectors and the simulator: captures from all release families are decoded, including WSJT-X, JS8Call-improved, N1MM Logger+, fldigi, PSK Reporter MQTT publications and IPFIX submissions, and independently produced `qgi` messages. Where a peer sends a response, that response is observed. PSK Reporter evidence uses existing operational captures; tests never connect to the service (`QGI-RI-0067`). | Demo | Target |
| `QGI-RI-0064` | **Simulator responses:** for every request/response or command-capable release family, answer or react as the peer would: fldigi XML-RPC replies and faults, JS8Call responses, WSJT-X reactions where it sends any, and other mapped family behavior. Do not invent an acknowledgment where the native protocol sends none. | Test, Demo | Met for WSJT-X receive-only fixed-period simulation: Replay and header-defined state reactions; other family responses: Target |
| `QGI-RI-0065` | **Local PSK Reporter IPFIX collector:** accept submissions as the service would and report, per record, malformed content and departures from the published submitter rules (`QGI-PSI-0002`). Diagnostics identify record/Set locations and distinguish parse errors, rule violations, recommendations and unconfirmed policy; message-level failures identify affected records when boundaries are known. Session timing and duplicate checks use scripted test state. | Test, Demo | Target |
| `QGI-RI-0066` | **Local PSK Reporter MQTT spot feed:** provide a local broker/feed that publishes the family's real MQTT topics, payloads and retain behavior to clients under test. | Test, Demo | Target |
| `QGI-RI-0067` | **Local PSK Reporter testing:** tests never connect to the real PSK Reporter, including its submission, diagnostic-listener and MQTT endpoints; all simulated service roles and test clients use local endpoints. | Test | Target |
| `QGI-RI-0068` | **Desktop simulator integration:** replacing `qso-graph-sim` in QSO Graph Desktop's suites is a coordinated change to `QGD-INT-0024`. | Test | Simulator side ready: drop-in options and independent Desktop capture checks; Desktop suite swap: Target in Desktop |

## 8. Decisions

| # | Decision | State |
|---|---|---|
| D1 | **Names.** Product and specification **QG Interop**; repository and crate `qso-graph-interop`, with `-cli` (command `qg-interop`) and later `-ffi`; family names `wsjtx`, `js8call`, `n1mm`, `fldigi`, `pskreporter-mqtt`, `pskreporter-ipfix`, `qgi`. No `-core` crate. | Decided 2026-11-11; qgi added 2026-11-12 |
| D2 | **JS8Call-improved, fourth; compare it with the original v2.3.1 when the family is mapped** (`QGI-JS8-0001`). | Decided 2026-11-12 |
| D3 | **Licence:** GPL-3.0-or-later for the library, the specification, its schema and its vectors, like everything in QSO Graph. A closed-source program can implement the specification; it cannot link the library. | Decided 2026-11-11 |
| D4 | **Round trip:** never lossy; the native message is kept and common-field canonicalizations are listed (`QGI-SPEC-001` `QGI-MOD-0007`). | Decided 2026-11-12 |
| D5 | **Default features:** what a plain `cargo add` brings. | Open |
| D6 | **Where the specification lives:** here, in `spec/`, beside its reference implementation. It can move to its own repository if its release pace ever differs from the code's. | Proposed |

## 9. Issue #5 review disposition

This task closes the specification review items for native protocol version versus mapping revision
(`QGI-MOD-0001`), a notification kind for Clear (`QGI-MOD-0003`, Annex B), distinct instant and
time-of-day types (`QGI-MOD-0006`), and required versus omitted versus null text (`QGI-SER-0002`,
Annex A). It supplies the core field catalog, WSJT-X mapping, event schema and independent versioned
vectors needed to implement those decisions. These are specification closures, not evidence that
the current implementation conforms. The second pass adds field-built commands, publish/subscribe
evidence, bounded caller-scoped template state and result events as specification targets; no Rust
implementation change is part of that pass. The third pass closes producer conformance through
`qgi` event carriage (`QGI-VEC-0004`, Annex I), specifies all-family simulator service roles and the
full 1.0 class gate, and records PSK Reporter submitter obligations. The generated C header and
foreign-caller contracts (`QGI-RI-0015`, `0050`–`0054`) remain Target. Annexes C, E, G and H still
have the profile work marked open in each annex; Annex D specifies N1MM with live-capture questions; implementation alignment and real-peer proofs
remain Target. D5 and D6 remain as recorded above.
