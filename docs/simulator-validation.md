<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Simulator validation and Python migration

## Option compatibility with qg-sim 0.3.1

Replace the executable prefix `qg-sim` with `qg-interop sim` and keep the playback arguments.

| qg-sim option | qg-interop sim |
| --- | --- |
| `--wsjtx` | Same ALL.TXT selection; required unless `--capture` is supplied. |
| `--to IP:PORT` | Numeric IPv4/IPv6, port 1–65535. Accepts bracketed and unbracketed numeric IPv6 as Python does. Exact source port is also checked for incoming commands; Python checked only the IP. |
| `--file PATH` | Same local-file selection and UTF-8 replacement behavior; absent means the embedded synthetic sample. Adds explicit size/resource bounds. |
| `--from YYMMDD_HHMMSS` | Same shape validation, lexical inclusive filter, and file order; modes without established periods also enter the skip count. An invalid calendar date in a log is skipped; the option itself validates shape, as Python does. |
| `--id NAME` | Default WSJT-X; 1–64 Unicode characters, no newline. Explicit empty Id is rejected as required by this task; Python's generic validator accidentally accepted it. |
| `--call CALL` | Default empty, uppercase conversion, 0–13 ASCII letters/digits/slashes. |
| `--grid GRID` | Default empty; uppercase four- or six-character Maidenhead locator. |
| `--speed N` | Default 1; finite 0 < N ≤ 1000. Unrepresentably long deadlines fail before sending. |
| `--loop` | Repeat with continuous cycle stamps; Replay history persists until Clear or the resource limit. |
| `--version` | Binary version is available as `qg-interop --version`. |
| Read errors | Exit 1, `qg-interop: cannot read PATH: REASON`; the executable prefix changes. |
| Empty selection | Exit 1, `qg-interop: no received decodes in NAME[ from STAMP]`, plus the skip summary when modes lack a period; the executable prefix changes. |

The old `--dx-call`, `--mode`, and `--frequency` options remain available and select the original
immediate nine-message QSO script. They conflict with file and capture input. The station
default remains `N0CALL`; the default DX call is `Q1AW`. The timed ALL.TXT player is now the default.

All invented callsigns use the Q prefix, including built-in samples, generated contacts,
portable edits, independent captures, and specification vectors. Call substitutions preserve
lengths and distinct identities, so reply pairing, ADIF lengths, truncation offsets, and native
message-size boundaries remain unchanged. Caller-supplied calls and captured input retain their
original values. The `N0CALL` station default remains unchanged.

## Independent capture oracle

`crates/qso-graph-interop/tests/fixtures/sim-python-synthetic.json` derives from Desktop's
`extensions/wsjtx/tests/fixtures/qso-graph-sim-0.2.0.json`. The fixture records its original SHA-256.
A Python `struct` transformation replaced callsigns using matching original/synthetic ALL.TXT
line pairs and replaced the Status station with Q1ZZZ; it rewrote length prefixes. No Rust encoder
produced the oracle. The source capture contains seven cycles, 120 new decodes, one Replay of the
first 15 decodes followed by Status, and Clear/Close: 159 datagrams total.

`tests/simulator.rs` runs the binary against an ephemeral loopback receiver at speed 100. Each
received datagram must decode into exactly one event without skips and preserve its original
bytes through the library. Full Packet equality checks every field, optional suffix, extension,
schema, Id, and message order against the capture. Only the initial UTC Decode time base and the
Heartbeat version/revision strings are normalized. Relative Decode timestamps remain checked,
including Replay timestamps. The Rust heartbeat identifies qg-interop and its package version.

The test injects Replay from the same IP but a different port, a wrong Id from the target, and
malformed bytes before the valid Replay. Only the valid request produces output. Planted faults
swap Status/Decode order, mark a Replay Decode new, and insert an extra old Decode as an illicit
response to the foreign request. The same comparison rejects each fault.

Mixed-mode process checks verify FT8, FT4, JT4, JT65, and all five recorded Q65 periods,
including native mode markers, submodes, Status periods, relative timestamps, and exact skip
counts/names. Unknown-period lines before and between playable lines cannot abort playback or
create cycles; an all-unknown file fails with the exact expanded error. Fixed expected periods
and invalid Q65 period/submode combinations detect guessed settings and wrong timing. Parser
checks cover start filtering, repeated skipped modes, escaped terminal controls, and rejection
of WSPR/Echo as ordinary Decode layouts.

Other automated checks cover two FT4 cycles plus a loop restart at 7.5-second logical spacing;
Close, SIGINT, and SIGTERM cleanup; capture byte equality and qgi reception; recorded timing;
fixed-interval fallback; malformed/oversized input; calendar and low-confidence parsing; and
changed-state command responses. The libFuzzer target exercises the CLI's public byte-slice
ALL.TXT API alongside native and event parsers. It seeds the synthetic sample and parser edges.

## Reaction model

The timed player models receive-only operation with established cycle periods, with the active log cycle selecting
mode and dial frequency. Configure can change a fixed-period mode, Rx offset (0–5000 Hz), DX call/grid,
and watchdog state between cycles. Unsupported mode names and inactive optional controls are
ignored. The next recorded cycle sets its own mode and frequency. No rig frequency table, radio
transmission, standard-message generation, decode selection for Reply, highlighting UI, or named
configuration store is modeled. Consequently a command that depends on such state has no invented
success reply. Free Text stages text; selection changes the reported transmitted message only
if already transmitting. Staged text isn't a claim about encoded on-air content.

A Clear for Band Activity/both empties Replay history; an Rx-only Clear doesn't. Halt Tx disables
Auto Tx, and an immediate halt also clears transmitting state. These cause Status only if state
changes. Location accepts four-/six-character grids and updates Status. Close ends playback.
Heartbeat updates the supported schema for the next scheduled Heartbeat without an immediate reply. Header citations are in
[protocol evidence](PROTOCOL.md#simulator-timing-and-reactions).

Native capture playback is byte-preserving UDP transmission and has no command interpreter or
added cleanup datagrams. `--qgi` uses the reader profile to preserve additions and validates the
UDP carriage budget, including room for the receiving hop. It neither verifies an unsupported
family's native mapping nor executes carried commands. TCP remains outside the CLI policy.
Receive times are optional reader extensions; the specification and listener producer format
are unchanged. Cross-platform CI remains responsible for macOS and Windows runtime coverage;
local signal delivery tests run on Unix.

## Dependency boundary

Signal registration retains signal-hook 0.1.17's safe flag API with default features disabled.
The [dependency comparison](DEPENDENCIES.md#signal-handler-decision) records why the current
signal-hook and ctrlc releases fail the package-wide native-source rule, including counts and
Windows limitations. This is an explicit compatibility exception for an old release line,
not a claim that 0.1.17 is currently maintained. Its resolved signal-hook-registry is 1.4.8.

Transitive Rust-only OS bindings require FFI declarations but add no native source or assembly.
Policy admits FFI/link declarations only for the recorded exact versions of libc, errno,
windows-sys, and windows-link, still scanning all files for native sources and assembly. Own
unsafe code, macros, process creation, DNS, and non-UDP sockets remain forbidden. A planted
policy test checks that ordinary dependencies cannot use that binding exception. Six exact-version
cargo-vet exemptions are labeled **not audited yet**, matching the existing dependency policy.
Unix SIGINT/SIGTERM process tests verify Clear/Close cleanup. Windows uses CRT signals with
re-registration races; not every console Ctrl+C becomes SIGINT, and SIGTERM is not a Windows
console termination mechanism. Windows runtime behavior still requires platform testing.

## N1MM contest simulator

`sim --n1mm` is a separate scripted peer using the existing numeric connected UDP boundary.
The socket filters the complete configured target address, including the port, before commands
reach the decoder. The library remains stateless and opens no sockets. The peer sends one
`AppInfo`, one-second logical radio snapshots, and new contacts at `floor(n × 3600000 / rate)`
logical milliseconds. This cumulative schedule avoids accumulating interval-rounding drift.
The default is 12 new contacts; `--loop` removes that default limit, while an explicit `--count`
always bounds the run. Looping keeps the clock, serials, IDs, random sequence, and score moving
forward. A fixed UTC epoch and a wrapping 64-bit LCG make the stream reproducible without
system time, a platform random generator, or additional dependencies.
Generated contacts have the shape `Q1ABC`, with digits from 0 through 9 and three suffix
letters. The many-seed test checks contact, spot, and prior-call fields and CLI validation,
including `/P` edits, and rejects a planted non-Q prefix. Repository policy also rejects the
retired invented calls in source text, hexadecimal datagrams, and base64 native data.

SO2R clears the previous transmitter's reported state before setting the next transmitter.
Every fifth new contact changes callsign to `/P` and points from two to three; every seventh
is deleted, including when the same contact was just changed. Scores follow every mutation,
using retained QSO count and points without storing unbounded contact history. Spots are removed
under their original keys even when the contact is edited. Contact bands track the tuned
frequency on the supported HF/6 m bands; an unlisted frequency has an empty band label.

The [protocol evidence](PROTOCOL.md#n1mm-mapping-and-evidence) cites Annex D's pinned QSY and
Spectrum source. A recognized QSY updates only an existing selected radio. The next scheduled
report and later contact/spot traffic use its new frequency. The simulated rig has 10 Hz tuning
resolution: integer-Hz commands drop their single-Hz remainder. This explicitly chosen simulator
policy is necessary because native `Freq`, `TXFreq`, `rxfreq`, and `txfreq` are integer 10 Hz
values. It is not a claim about actual rigs or N1MM's precision. Spectrum input, invalid XML,
missing or ambiguous frequency/selector fields, negative frequencies, and absent radios change
nothing. No command causes an immediate response or acknowledgment.

The hand-authored `tests/fixtures/n1mm-simulator.json` in the library crate supplies initial
AppInfo/RadioInfo bytes, the first contact's fields, and the final score for a 35-contact script.
It follows Annex D independently of the simulator serializer and is explicitly synthetic,
not a live N1MM capture. The CLI's `tests/n1mm_simulator.rs` runs the binary over loopback and
decodes every datagram using the library, requiring exactly one event, no skips, and original
byte retention. A separate receiver ledger applies contact IDs, edits, and deletions and checks
every score snapshot. Latest per-radio state checks detect simultaneous transmitters and
require contacts to use the active transmitting radio. The tests check alternating radios,
focus bounds, a single running radio, frequency units, spot-key lifetimes, UTC stamps, serials,
counts, shutdown flags, seed reproducibility, and unchanged logical traffic at different speeds.

QSY tests command both radios, require the next report to reflect tuning, then inject commands
from a different source port and Spectrum from the target. The complete stream must equal a
run with only the valid QSY inputs, catching extra RadioInfo acknowledgments as well as frequency
changes. Planted valid-XML mutations introduce two transmitters, obedience to a foreign QSY,
a score inconsistent with contacts, and a contact on the wrong radio; the same oracles reject
all four. Option rejection tests cover protocol conflicts and numeric/string bounds before
sending. Unit checks cover UTC day/month/leap-year rollover, decimal kHz, sub-10-Hz QSY policy,
malformed/ambiguous commands, and selection of an absent radio. Unix process tests interrupt
indefinite looping with SIGINT and SIGTERM and require final non-transmitting radio reports.
