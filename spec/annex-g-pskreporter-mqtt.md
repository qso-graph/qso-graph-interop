<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex G: PSK Reporter MQTT, mapping revision 1

This annex defines `pskreporter-mqtt` for specification 0.2.0. It decodes one
already captured MQTT message: topic plus exact payload bytes, with retain and
optional QoS context. It implements no MQTT client, socket, subscription,
acknowledgment, discovery, lookup, publication, or receiver-metadata join.
Annex I already defines event carriage; the existing MQTT reservation is Annex G.
This is a published-format mapping, without a provisional designation.

## G.0 Source pins and authority

| Source | Location, retrieval and SHA-256 |
|---|---|
| S1 | Service page, <https://www.mqtt.pskreporter.info/>, retrieved 2026-11-14T17:28:27Z. File `www.mqtt.pskreporter.info_index.html`, SHA-256 `d5fd2ea058681ff5f4777d3e7383802e0fa8964ce9b7ad0acdced4c3eb47248d`. Line numbers below refer to that exact file. |
| S2 | Caller-supplied anonymized capture `mqtt-capture-q.json`, received 2026-11-14, re-anonymized to preserve callsign escaping. SHA-256 `221c57920393bd85a2e8386e28f952f7427db0939f76e7a7fcac504eebb1e09f`. 460 records: 400 spots and 60 receiver-information messages. Every record's topic and payload base64 is preserved independently in `crates/qso-graph-interop/tests/fixtures/pskreporter-mqtt.json`; the enclosing capture metadata is not wire evidence. |

S1 governs meanings, S2 establishes observed payload types. S1:1062–1063 links
`timing.html` and its receiver-information section. That document was not supplied,
was not retrieved for this work, and is not a source pin. In particular, its
contents MUST NOT be inferred from its URL or from payload field names.
The page and original capture are not redistributed. The fixture preserves the
anonymized records without rewriting topic or payload bytes.

## G.1 Input and exact topic agreement

**`QGI-PSM-0002` — Input.** `Codec::decode` receives the payload bytes. Its native
association contains `topic` (string), `retained` (boolean), and optional `qos`
(integer 0, 1, or 2). The capture adapter defines omitted `retain` as false;
this is an input-format default, not a claim about an omitted broker flag.
Omitted QoS remains absent. Unknown association members are `invalid_value`.
A caller needing exact delivered retain context MUST supply it.

**`QGI-PSM-0003` — Topic.** Admit exactly these topic forms (S1:1232, S2):

```text
pskr/filter/v2/<band>/<mode>/<sender call>/<receiver call>/<sender grid>/<receiver grid>/<sender DXCC>/<receiver DXCC>
pskr/rxinfo/<call>
```

Topic size is at most 512 UTF-8 bytes; each component is 1–128 ASCII bytes from
letters, digits, hyphen, period, and underscore. No wildcards, empty components,
trailing suffix, slash-containing component, or extra prefix are admitted.
Spot topic grids are exactly four characters: two uppercase letters A–R and two
digits. DXCC components are canonical decimal integers 0–65535, without leading
zeros except `0`. These finite limits are QGI admission policy, not published
service limits. Future canonical `vN` versions (u32, N > 2) in an isolated 11-component spot
message produce `future_version` skips after payload/topic/component size checks,
without interpreting payload or version-specific components. Older canonical
versions are `unsupported_version`; other layouts are `invalid_value`. `native_version` is `2` for spots and `rxinfo` for receiver info.

**`QGI-PSM-0004` — Agreement.** When a payload has a nonnull value also present in
the topic, compare band and mode exactly; compare DXCC numerically. For callsigns
(`sc`, `rc`, and rxinfo `call`), the topic segment MUST equal the payload call
with every `/` replaced by `.`. Decode topic periods to slashes for this comparison
only; never rewrite a payload call. Comparison is otherwise exact and case-sensitive.
Reject a literal `.` in a payload callsign as an atomic `invalid_value` disagreement:
it would make the mapping ambiguous. This rejection is QGI admission policy;
no payload callsign in S2 contains a literal period.
Compare the first four characters of each payload locator with the topic grid,
without regard to ASCII case. S1:1232 explicitly labels topic grids four characters;
S1:1336,1338 shows longer payload locators. Keep full payload locator text,
including case and lengths beyond eight characters. Missing or null values do
not assert equality and do not acquire a guessed payload value. A disagreement
is an atomic `invalid_value` error. Never strip portable-call suffixes, infer
aliases, or choose one callsign silently.

The callsign escaping rule is **observed capture evidence**, not documented by S1.
The caller reports that all 460 original, un-anonymized records satisfy it, with
41 spot records containing `/`; rxinfo topics follow the same rule. The corrected
S2 independently exhibits agreement in all 460 records, including 41 slash-bearing
spots and one slash-bearing rxinfo record. For example, record 72 pairs payload
sender `Q4AAH/0` with topic `Q4AAH.0`; record 188 pairs receiver `W/Q2AAT/Q3AAT`
with `W.Q2AAT.Q3AAT`; record 48 pairs rxinfo `Q8AAE/P` with `Q8AAE.P`.
Short received prefix/suffix parts are preserved. S2 yields 460 events and zero
errors. A bare call, dropped slash, or misplaced period that violates exact
agreement remains a disagreement error. Two lowercase payload locator prefixes
are equivalent to their uppercase topic grids and are admitted unchanged.

## G.2 Strict JSON and field meanings

**`QGI-PSM-0005` — JSON.** Admit exactly one UTF-8 JSON object, at most 16,384
bytes, 16 nested containers (root counts as one), 4,096 total object members and
array elements, 4,096 decoded UTF-8 bytes per key/string, and 128 bytes per
numeric token. Reject duplicate keys recursively, including escaped spellings,
trailing data, malformed Unicode, and non-JSON numbers as `malformed`. Size,
depth, element, string and token limits are `oversized`; known type/range errors
are `invalid_value`. Parsing is bounded even before the stricter family limits
are checked. Known fields are optional; null, absent and empty remain distinct
in exact evidence. Strings accept string or null; numbers accept integer tokens
or null, never booleans, fractional or exponential tokens, or numeric strings.
Unknown keys and all their values remain native extensions; never interpret them.

| Payload | Meaning, common field and exact evidence |
|---|---|
| `sq` | Sequence number (S1:1329); native only, integer 0–9007199254740991. No deduplication, ordering or wrap behavior is inferred. |
| `f` | Frequency in Hz (S1:1330) → `spot.frequency_hz`, integer 0–9007199254740991. |
| `md` | Mode (S1:1331) → `spot.mode`; no conversion to a guessed mode vocabulary. |
| `rp` | SNR report in dB (S1:1332) → `spot.snr_db`, integer −128–127. S2 has 59 null reports: retain null in native data and omit the integer common field. |
| `t` | Epoch time varying by software (S1:1333) → `spot.time`, UTC Unix seconds 0–253402300799 (through year 9999). This is not receipt time and makes no transmission-start claim. |
| `t_tx` | Derived transmission-start epoch, may be absent (S1:1334) → optional `spot.transmission_start`, same seconds range. Do not replace `time` with it. |
| `sc`, `sl` | Sender callsign and locator (S1:1335–1336) → `sender_call`, `sender_locator`. |
| `rc`, `rl` | Receiver callsign and locator (S1:1337–1338) → `receiver_call`, `receiver_locator`. |
| `sa`, `ra` | Sender/receiver ADIF DXCC (S1:1339–1340) → `sender_dxcc`, `receiver_dxcc`, integers 0–65535. No country lookup or validity claim. |
| `b` | Band (S1:1341) → `spot.band`; do not derive it from frequency. |
| rxinfo `call` | Observed string/null (S2), checked against the receiver call in the topic using `QGI-PSM-0004`. `receiver_information.receiver_call` comes from that topic with every `.` decoded to `/`, including when payload `call` is null or absent. Native payload null and absence remain unchanged. |
| rxinfo `antenna`, `rig`, `decoder`, `grid` | Observed string/null types (S2). S1 does not define these members. Native only: do not infer `antenna_information`, `rig_information`, `reporting_software`, or `receiver_locator` from their names. |
| rxinfo `time_start`, `ts` | Observed integer/null types (S2), admitted within ±9007199254740991. Meanings, units and enumerations are not stated in S1. Native only: neither a boolean interpretation nor Unix-time semantics is inferred. |

All numeric ranges above are explicit QGI resource/representation policies,
not service constraints. Null numeric values have no integer/instant common
mapping; their exact native JSON preserves null versus missing. Empty strings
remain empty (and disagree with a nonempty topic value when checked).
No radio-part mapping is established.

## G.3 Evidence, verification and output bounds

**`QGI-PSM-0006` — Events.** A spot yields one `spot`; receiver info yields one
`receiver_information`. Reuse Annex H's vocabulary wherever meaning is established.
Each event carries `native.format: "mqtt_publish"`, exact base64 payload `data`,
exact topic, retain flag, and optional QoS. Optional `native.message` contains
the complete parsed payload, including unknown extensions. There are no commands,
metadata joins, caches, wall clocks or transport state. Zero-length payloads are
malformed JSON; S1 does not establish deletion semantics. Exact payload re-encoding
and retrieval preserve whitespace, key order, numeric spellings and raw bytes.
The topic and flags travel separately as evidence, not as invented payload bytes.

**`QGI-PSM-0007` — Verification.** Reparse exact payload with its topic and flags;
compare kind, native version, all known common fields and any supplied structured
view. Mismatch is `invalid_value`. Native verification is unavailable for another
family, unsupported native version, or a command. Future additive common fields
follow `QGI-GEN-0005` and the core compatible view. No structured view overrides
raw evidence. No network access is required to decode, verify or re-encode.

**`QGI-PSM-0008` — Expansion.** For payload N ≤16,384, common escaped text occupies
at most 6N bytes, a structured copy at most 6N, and exact base64 at most
4×ceil(N/3). Topic, maximum 4,096-byte context, field names, times, flags and
envelope fit 16,384 more bytes. The bound is below **235,000 bytes**, safely
within the 1,048,576-byte event cap. Two payload views have at most 8,192 members/
elements; context and envelope keep the total below 16,384 and the core 65,536
limit. A structured payload depth of 16 plus the event/native containers remains
below 32. Exact output admission is still mandatory, with planted boundary failures.

## G.4 Capture adapter and conformance

`qg-interop decode --pskreporter-mqtt FILE` reads local capture JSON with a
`records` array, or NDJSON records `{topic,payload_b64[,retain,qos]}`. JSON files
are limited to 1,048,576 bytes and the core JSON parser limits. NDJSON streams
use a 32,768-byte line cap. Capture envelope metadata and per-record `received_at`
are observer information, not native payload fields, and are ignored. Output is
one QGI NDJSON event per accepted record. A failure stops the command with a
nonzero status; preceding output remains output. No invalid record becomes a spot.

The capture fixture, literal common-field oracles, hashed generated vectors and
planted negatives check both message kinds, captured modes/bands, 4/6/8/10-character
locators, null SNR, slash calls in both spot roles and rxinfo, literal payload
period rejection, every admission bound, duplicate keys, wrong types and all
agreement dimensions. The `mqtt` fuzz target mutates payloads, topics and capture
records, checking successful verification and exact re-encoding. Verification
runs entirely offline using cached tools and vulnerability data.
