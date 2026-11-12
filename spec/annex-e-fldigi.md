<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex E: fldigi, mapping revision 1

## E.1 Evidence and scope

This profile covers the request/response family `fldigi` in specification `0.1.0`.
The retrieval date is **2026-11-12**. The latest released version found that day is
**4.2.13**. The [SourceForge release listing](https://sourceforge.net/projects/fldigi/files/fldigi/)
shows `fldigi-4.2.13.tar.gz` modified **2026-08-03 14:16:04 UTC**; the
[author's download listing](https://www.w1hkj.org/files/fldigi/) shows the same
filename dated **2026-08-03 09:14** (listing time zone unspecified). Neither
listing contains a later release. The author's tarball was retrieved successfully;
no git-tag fallback is used, and no unverified tag or commit is attributed to it.

- Release source: [fldigi-4.2.13.tar.gz](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz).
- Retrieval: **2026-11-12**; SHA-256: `a1e8d990359ce9c0cce3ceb5116fd0cf72c95528969766898640ca6ca2dba8d4`.
- Archive root: `fldigi-4.2.13/`; `configure.ac` (S16) declares 4.2.13.
- S2–S6 and S8–S16 below identify complete files inside that archive, relative to
  its root. Their links identify the containing tarball, not loose current files.
- S1 and S7 retain the evidence retrieved earlier on the same day. Hashes cover
  complete downloaded bytes, not rendered text. No upstream source is bundled
  with the synthetic vectors.

| Source | URL / archive member | SHA-256 |
|---|---|---|
| S1 | [Published 4.2.00 method reference](https://www.w1hkj.org/FldigiHelp/xmlrpc_control_page.html) | `ab38dad09415a939d5b48a4bd17830012fc05993aae6e0eab3cca4cd164da00d` |
| S2 | [src/network/xmlrpc.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `43b6f600a9236b792e07614066f774027140f637a9233666e56d89b3d4e81cab` |
| S3 | [src/xmlrpcpp/XmlRpcValue.cpp](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `25d2ff6a7d4df84768b8091a75f1b2e9c1f4f8236093715cae19d2341444626f` |
| S4 | [src/include/Fl_Text_Buffer_mod.H](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `9c42d81485007555105f89cbe3af8df2506234fdb20840b2c42e229fc4c63840` |
| S5 | [src/trx/modem.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `9a6206e4d744c462a4b986c2de3fc7323fea95ede55ea4361cb49ec990a95e4e` |
| S6 | [src/dialogs/fl_digi.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `0fa533b090a0ab6d83758d6edf5c8b67823950eb32201fba20c527cc6e730e2e` |
| S7 | [XML-RPC specification, canonical HTTPS location](https://xmlrpc.com/spec.md) | `0d53559da22741400cffedc23fa911f5cea386d22f30428fc9eb7b8200adf765` |
| S8 | [src/logbook/qso_db.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `e508c092ec0c3d7ee122e41b41f3358e52847e174d4a8875cf5c39368f8f3606` |
| S9 | [src/logbook/lookupcall.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `ad590c37c8bae0f1749703fe9e0008e9c241a6e001872c6c7f74ada5435788d4` |
| S10 | [src/psk/psk.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `2571daabd66026db324bada3acd998c5ff13c8d3af28ef619afe3daf876ecdc9` |
| S11 | [src/xmlrpcpp/XmlRpcValue.h](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `0a7e10aa10cfcbe80debd786f2316e74c92073da8ef4f98947f413ef2692a4af` |
| S12 | [src/xmlrpcpp/XmlRpcServer.cpp](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `5596e7385e896d43b411aa47214b185bd6fa0f190ad3b088849cbe6bf722e8ff` |
| S13 | [src/globals/globals.cxx](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `e4f4663432419ab757058d45d75dbf19fc4eb09afa4b3cb8c87390e3992462f8` |
| S14 | [src/include/globals.h](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `65fa9d98cb6477edc9f52c89b8a1f987a0922ab05391c16225d2a83ddbe25ec5` |
| S15 | [src/Makefile.am](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `10c6ff4ab161497c5fe01ec59086f200177f304549ecc26624c26fc5c6dee9a7` |
| S16 | [configure.ac](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz) | `d2fb2013db9915736d4a656746c833797734715f369fe536ff3694048c848994` |

S2's METHOD_LIST and named handler constructors/bodies are the authority for
method spelling, arguments, advertised signatures and effects. S1 is the 4.2.00
manual, not a description of every handler in 4.2.13. S3 establishes the bundled
value parser; S4 establishes byte units where S1/S2 help says “characters.” S15
and S16 permit an external libflxmlrpc backend instead of the bundled sources;
its parser/serializer behavior is not established by these bundled-file hashes.
The profile's acceptance/construction rules are explicit policies, independent
of backend permissiveness. S3's self-closing number defaults and permissive
numeric conversion are not adopted.

**`QGI-FLD-0068` — Authority.** A codec MUST use S2's released handlers in preference
to conflicting S1 documentation; QGI acceptance and construction policies below
are profile rules, not claims about every build's acceptance limits.

The Hz assignment is a source derivation: S9 QsoHelper passes wf->rfcarrier()
unchanged to S8 cQsoRec::setFrequency, explicitly documented as Hertz and divided
by 1,000,000 for the log frequency. S2's RF methods access that same carrier. S10's
PSK AFC adjusts the modem frequency by samplerate/sfft_size, with a 1 Hz bin/drift
comment, establishing audio Hz independently of RF. S6 qsy keeps those quantities
separate. No sideband addition is performed in this profile.

### E.1.1 Complete change audit from 4.1.23

The baseline is the previously retrieved `v4.1.23` source, commit
`61b97f4133c488063f3de1795c894d22d5032e8a` (old S2 SHA-256
`ac255d2999f6ce3b914a4d8787e454749ba64df4f48faea13947c2b11e70fc2d`).
Comparing both complete METHOD_LISTs and all registered handler classes, including
inherited constructors/bodies, gives **172 → 174 distinct names**, **2 additions**,
**0 removals**, **0 signature changes**. Duplicate `log.set_rst_in/out`
registrations remain; moving `log.set_serial_number` within the table does not
remove or change that method. The added methods remain native in both directions.

| Added method | Declared signature | Released handler behavior |
|---|---|---|
| `modem.get_io_names` | `A:n` | Modem_get_io_names returns an array of `sname` strings for entries whose `iface_io != DISABLED_IO`, in mode-table order. |
| `log.set_contest_counter` | `n:s` | Log_set_contest_counter reads a string, converts it with `atol` into `contest_start_number`, queues setting the start-number widget and `progdefaults.ContestStart`, calls `cb_ResetSerNbr`, and marks defaults changed. The log message misspells the name as `log.set_counter_counter`; that is not a registered method. |

Every changed existing S2 handler is listed here; inherited aliases are explicit.
All other registered handler bodies/constructors are unchanged in S2.

| Method(s) | 4.1.23 → 4.2.13 change | Mapping effect |
|---|---|---|
| `modem.get_bandwidth` | `get_bw_val` returns an int instead of a widget pointer: Hell slider value, `progdefaults.CWbandwidth` for CW, zero without CAP_BW/for other modes. The old logging dereference is gone. | Native; integer result. |
| `modem.set_bandwidth`, `modem.inc_bandwidth` | Setter/increment bodies are commented out; only XMLRPC_LOCK remains. Neither reads its parameter, changes bandwidth nor assigns retval. Advertised `i:i` remains. | Native; no invented integer acknowledgment. |
| `main.set_frequency` | RF input cast changes from signed long long to unsigned long long; old RF snapshot explicitly cast to double. | Still `set_frequency`; returns old RF, not the requested RF. |
| `main.inc_frequency` | RF sum explicitly casts the carrier to double and passes unsigned long long to qsy; log format changes. | Native; returns computed RF sum as double. |
| `rig.set_frequency`, inherited `main.set_rig_frequency` | Input cast changes from signed long through unsigned long to unsigned long long; old RF snapshot explicitly cast to double; log formatting changes. | First method mapped, alias native; old RF result unchanged. |
| `rig.get_frequency`, inherited `main.get_frequency` | Explicit double cast and log format change. | Same dial_frequency_hz mapping. |
| `main.get_trx_status` | Reads `trx_state`, returning `tx` for STATE_TX, `tune` for STATE_TUNE, otherwise `rx`; formerly inspected tune and waterfall GUI buttons. | Same string/boolean mapping, now reflects engine state rather than potentially pending GUI state. |
| `rig.set_pwrmeter` | Sets an existing power-meter widget even without smeter/meters enabled; returns early if power-level group visible, otherwise conditionally shows/redraws meter and hides an existing smeter. | Native; acknowledgment expression unchanged. |
| `rig.get_bandwidths` | Log label corrected from `rig.get_modes` to `rig.get_bandwidths`. | Native; array result unchanged. |
| `logbook.all_records` | Windows-specific unsigned size format in log. | String mapping unchanged. |

These are **11 directly changed existing handler classes**, affecting **13 names**
after two inherited aliases; two of those classes change only logging. The helper
`xmlrpc_set_qsy` also changes from signed long long narrowed to unsigned long to
unsigned long long without narrowing. In S6, qsy changes to unsigned long long,
keeps the current RF only when the unsigned request is zero, and stops narrowing
the fallback qso_selectFreq argument to long. Routing remains flrig, rigCAT,
optional Hamlib, then qso_selectFreq; the optional audio `fmid` path remains
separate, including lock/sideband adjustment and the RTTY FSK adjustment. Both
mapped setters call qsy with RF only. Negative/out-of-range floating-to-unsigned
conversion has no portable result established here. QGI's existing conservative
writer bounds remain unchanged, and received doubles are preserved.

S3 differs only in its copyright comment; value parsing/serialization is unchanged.
S4 is byte-for-byte unchanged. S8 changes cQsoRec::setFrequency's argument to
unsigned long long but retains its Hertz-to-MHz conversion; S9 still supplies the
same RF carrier. S10 retains the samplerate/sfft_size AFC Hz derivation. S5 retains
separate RX/TX audio frequencies, now also persists `tx_lock` in set_freqlock;
get_txfreq_woffset no longer adds the test-signal window's extra offset. Neither
change authorizes a tx_offset_hz inference. Other DSP/UI changes do not establish
a new wire field or codec normalization.

S13/S14 expose additional changes to methods whose S2 bodies are unchanged:

- Ten mode entries are added: `SCAMPFSK`, `SCAMPOOK`, `SCFSKFST`, `SCFSKSLW`,
  `SCOOKSLW`, `SCFSKVSL` (export mode MFSK, matching export submode), and `THOR25`,
  `THOR32`, `THOR44`, `THOR56` (export mode THOR, empty export submode).
- The `sname` for DominoEX Micro changes `DOMEX Micro` → `DOMEXM`; THOR Micro
  changes `THOR Micro` → `THORM`. This affects get_names/get_name/set_by_name.
- SLOWHELL, HELLX5 and HELLX9 gain matching nonempty export submodes. Their
  `modem.get_submode` replies now carry those strings. FSKH245, FSKH105 and HELL80
  change `adif_name` to HELL, but their already-HELL export_mode and existing
  export_submode are unchanged, so the mapped getters do not change.
- OFDM2000F/OFDM2000 entries move in the table; no mode is removed. Added modes
  and reordered entries change list order, numeric IDs and maximum ID. Numeric
  IDs remain native; the profile never translates them into mode names.

The 50 common method mappings keep their operations/types. Changed engine status
selection, renamed modem labels, added mode labels and newly nonempty Hell
submodes are the observable mapped behavior differences. No version-based gating,
sideband arithmetic, session state or new common field is introduced.

### E.1.2 Advertised types, actual values and the manual

All of these discrepancies are present in the pinned release; except for the
bandwidth no-op change they already existed in 4.1.23. S2 signatures do not
validate handler argument types or guarantee a returned value.

| Method(s) | Advertised | Handler behavior / actual value |
|---|---|---|
| `rig.get_notch` | `s:n` | Returns int notch_frequency. |
| `rig.set_notch`, `rig.enable_qsy` | `n:i` | Return int (old notch, supplied enable value, respectively). |
| `main.flmsg_available`, `flmsg.available` | `n:n` | Return int byte size of flmsg_data. |
| `main.flmsg_transfer`, `flmsg.transfer` | `n:n` | Return string, then clear flmsg_data. |
| `main.get_tx_timing` | `n:s` | Normal path reads base64 bytes, returns string; excluded modes return a string before reading an argument. |
| `main.get_char_timing` | `n:i` | Normal path reads base64 bytes containing decimal character text, returns string; excluded modes return a string before reading an argument. |
| `main.get_rig_bandwidths` | `n:A` | Inherits Rig_set_bandwidths, consumes an array and invokes the setter; it is not a getter. |
| `modem.{get,set,inc}_afc_search_range` | `i:n`, `i:i`, `i:i` | Return string “Operation not supported by modem” without CAP_AFC; otherwise int. |
| `modem.set_bandwidth`, `modem.inc_bandwidth` | `i:i` | Do not assign retval in 4.2.13. |

For **47** nominal-nil names, the handler either assigns `value_nil()` (**45**,
including inherited aliases) or leaves retval untouched (`main.flmsg_online`,
`flmsg.online`). Crucially, S2 aliases value_nil to XmlRpcValue, whose default
constructor is TypeInvalid in S11, not TypeNil. S3 serializes TypeInvalid to an
empty string, and S12 wraps that in an empty param. The two no-op bandwidth
handlers likewise leave the default result invalid. This is a derivation for the
**bundled backend**, not a claim about externally linked libflxmlrpc builds.
Every nominal-nil name outside the eight int/string exceptions above belongs to
that 47-name set. E.4's nil response mappings are acceptance policy for actual
well-formed nil replies; the vectors using nil are synthetic profile cases, not
captures claimed from this bundled backend. Empty source-generated params are
malformed under E.3 and cannot advance a multi-request operation.

S1 and S2 disagree on **10 shared signatures** (manual → released declaration):

| Method(s) | S1 | S2 |
|---|---|---|
| `main.get_char_timing` | n:6 | n:i |
| `main.get_tx_timing` | n:6 | n:s |
| `modem.inc_afc_search_range`, `modem.set_afc_search_range`, `modem.inc_bandwidth`, `modem.set_bandwidth` | n:i | i:i |
| `spot.set_auto` | n:b | b:b |
| `spot.toggle_auto` | n:b | b:n |
| `text.get_rx` | 6:i | 6:ii |
| `wefax.send_file` | s:i | s:si |

The timing argument descriptions in S1 are closer to the bodies than S2's
declarations, but both advertise nil where the handlers return strings. The
native catalog retains S2's declarations and tests actual type alternatives.
S1 alone lists `mmain.set_txid`, `rig.take_control`, `rig.release_control`;
S2 instead registers `main.set_txid` and neither control method. S1 omits these
**23** S2 names: flmsg.available, flmsg.get_data, flmsg.online, flmsg.squelch,
flmsg.transfer, log.get_date_off, log.get_date_on, log.set_contest_counter,
logbook.all_records, logbook.last_record, main.flmsg_available, main.flmsg_online,
main.flmsg_squelch, main.flmsg_transfer, main.set_txid, modem.get_io_names,
modem.get_mode, modem.get_submode, rig.enable_qsy, rig.get_frequency, rig.set_notch,
text.add_tx_queu, wefax.start_manual_reception. The log spelling
`text.add_tx_queue` is not a registered wire name.
S1/S2's “characters” means bytes for text-buffer ranges (S4); RF “carrier” in the
frequency setter help means wf->rfcarrier, not modem audio. The type/behavior
discrepancies above and S1's claimed bandwidth setters cannot override the bodies.

## E.2 Boundary, association, version and outcomes

**`QGI-FLD-0002` — Body.** One native input MUST be exactly one complete HTTP request
body or response body, excluding headers, transfer framing and content decoding
performed by the transport (S7 Payload format; `QGI-CLS-0003`, `0006`).

**`QGI-FLD-0003` — Evidence.** Each successful input MUST produce exactly one event
with `family: "fldigi"`, `mapping: 1`, `native.format: "fldigi_xmlrpc"` and the
complete unchanged body in `native.data`.

**`QGI-FLD-0004` — Context.** A codec MUST preserve the caller's context, including
unknown direction/exchange and any correlation ID, without inferring it from XML,
method spelling, polling order or program name (`QGI-CLS-0005`).

**`QGI-FLD-0005` — Association.** A response MUST use only the caller's supplied
`request.method` to select a method mapping; without it the response remains
`native`, except a valid fault is always an error `result` (S7 Response format).

**`QGI-FLD-0006` — Caller arguments.** For `text.get_rx`, response mapping MUST also
receive the associated request's two integers in `request.arguments`; other
response mappings need only `request.method`.

**`QGI-FLD-0007` — Association validation.** A supplied association MUST have exactly
`method` and optional `arguments` (an array of at most 32 values); its method obeys
`0025`, and a supplied method disagreeing with a decoded methodCall MUST fail as
`invalid_value`.

**`QGI-FLD-0008` — No echoed method.** A response MUST NOT be checked against an
invented echoed method: a wrong caller association with a compatible return type
cannot be detected from the response (S7 methodResponse has no methodName).

**`QGI-FLD-0009` — HTTP metadata.** HTTP status and content type, if supplied, MUST
remain transport metadata outside the event and MUST NOT change body decoding;
this codec neither treats an HTTP error as an XML-RPC fault nor rejects a valid
fault because of status/content type (S7 Response format distinguishes HTTP and
application errors).

**`QGI-FLD-0010` — Version query.** An application identifying a peer SHOULD query
`fldigi.version`, whose string result is PACKAGE_VERSION (S2 Fldigi_version_string).

**`QGI-FLD-0011` — Native version.** Only an associated successful `fldigi.version`
string response MUST set `native_version` to that returned valid, nonempty text,
using that envelope member instead of duplicating it in result.text;
empty/invalid version text maps to result.text instead; other bodies omit
native_version, including faults, `fldigi.version_struct`, invalid text and
unassociated version-looking strings (`QGI-MOD-0034`).

**`QGI-FLD-0012` — No session inference.** A codec MUST NOT copy a prior version
query into later events, identify a protocol revision from an XML declaration,
or infer a method's availability from a version label.

**`QGI-FLD-0013` — Outcome dispatch.** After E.3 validation, a reader MUST apply this
table; no outcome updates implicit session state.

| Input | Outcome |
|---|---|
| Cataloged method with matching argument/return types | E.4 mapping |
| Cataloged mapped method with different valid types or request arity | `native`; retain method name in E.7 view |
| Any other method, including a name absent from S2 | `native`, in either direction; no unknown-method skip |
| Response without association, or range read without valid range association | `native` |
| Fault with or without association | E.5 error `result` |
| Newer peer with known method | Same mapping; version label does not disable it |
| Unknown root or unknown value type | `native`, after complete XML validation |
| Malformed XML/XML-RPC container | `malformed` |
| Empty body or unfinished XML at EOF | `truncated` |
| Unsupported XML version/encoding | `unsupported_version` |
| Invalid known scalar / invalid association | `invalid_value` |
| Input, parser, context or output bound exceeded | `oversized`, atomically |

## E.3 XML and XML-RPC acceptance

These are QGI isolation policies (`QGI-SEC-0006`, `0007`), independent of the
upstream XML library's permissiveness. XML whitespace means space, tab, CR and LF.
References to Annex D select existing normative rules, not a second definition.

**`QGI-FLD-0014` — Body bound.** A reader MUST reject bodies exceeding **71,155 bytes**
before allocating XML structures.

**`QGI-FLD-0015` — XML substrate.** A reader MUST apply `QGI-N1M-0017`, `0018`,
`0020`–`0023`, `0025`, and `0052`'s XML syntax, encoding, invalid-unit and text
processing rules, substituting E.3's body/text limits and E.3's scalar rules for
Annex D's datagram/mapped-number rules.

**`QGI-FLD-0016` — XML resources.** The total processed text and attribute-value
budget MUST be **71,155 bytes**, measured after entity and line-ending processing
as UTF-8 for valid characters and original bytes for invalid code units.

**`QGI-FLD-0017` — Value resources.** A reader MUST bound XML-RPC params, array
values and struct members to **32, 256 and 256**, respectively, in each container.
XML depth 16, 1,024 elements, 16 attributes per element, 256 attributes per document
and 128-byte lexical names remain as specified by `QGI-N1M-0018`.

**`QGI-FLD-0018` — Numeric resources.** Every known int/i4, boolean or double value,
including inside native-only arrays/structs and unknown methods, MUST have at most
128 characters after surrounding XML whitespace is removed, before conversion.

**`QGI-FLD-0019` — Containers.** A recognized XML-RPC document MUST follow the
following grammar (S7 Payload/Response/Structs/Arrays), ignoring comments and
processing instructions and allowing only XML whitespace between child elements:
`methodCall = methodName, params?`; `params = param*`; `param = value`;
`methodResponse = params | fault`, with exactly one response param;
`fault = value`; `array = data`; `data = value*`;
`struct = member*`; `member = name, value`.

**`QGI-FLD-0020` — Structural uniqueness.** Recognized structural and type elements
MUST be unqualified and have no attributes other than namespace declarations
that leave their expanded name unqualified; duplicates, extra children, mixed
nonwhitespace container text and conflicting params/fault MUST be `malformed`.

**`QGI-FLD-0021` — Extension values.** A value with exactly one unknown type element
MUST make the entire message native-only, without interpreting descendants as
XML-RPC values; XML isolation and XML resource limits still apply to that subtree.

**`QGI-FLD-0022` — Typed values.** A `value` MUST contain either character data alone
(an untyped string, including empty), or exactly one type child with only XML
whitespace outside that child (S7 Scalar values).

**`QGI-FLD-0023` — String conversion.** String/untyped values and struct names MUST
preserve all processed text without trimming, with invalid text represented by the
core UTF-8/UTF-16 wrappers; a typed string or name containing elements is malformed.

**`QGI-FLD-0024` — Scalar conversion.** A reader MUST use this table for known types;
a scalar containing elements is malformed, and a nonmatching known scalar is
`invalid_value`, even in a message destined for kind native.

| Type | Acceptance and semantic value | Evidence |
|---|---|---|
| int, i4 | Optional ASCII sign then one or more ASCII digits; signed 32-bit range | S7 scalar table; lexical strictness is QGI policy |
| boolean | Exactly `0` or `1`, false/true | S7 scalar table |
| string / untyped | Text under `0023`; empty is empty, never null | S7 default string; S3 stringFromXml |
| double | `[+-]?(digits(\.digits*)?|\.digits+)([eE][+-]?digits+)?`; correctly rounded finite binary64 only | S7 double; grammar/finite restriction is QGI policy |
| dateTime.iso8601 | Exactly `YYYYMMDDTHH:MM:SS`, year 0001–9999 and valid Gregorian calendar/clock, seconds 00–59; retained as local/unresolved text, no common instant | S7 example and timezone discussion; grammar is QGI policy |
| base64 | Remove only XML whitespace, then canonical padded RFC 4648 base64 with zero pad bits, including empty; yields arbitrary bytes | S7 binary; canonical acceptance is QGI policy |
| nil | Empty element or whitespace-only content; explicit null, never empty string | S3 NIL_TAG/fromXml/toXml, S2 value_nil |
| struct | Ordered members; names are strings, duplicate names retained without selecting a winner | S7 struct; duplicate retention is QGI policy |
| array | Ordered values inside one data element, including empty | S7 array |

**`QGI-FLD-0025` — Method names.** A methodName or associated method MUST be valid
text matching `[A-Za-z0-9_.:/]+`, at most 128 UTF-8 bytes, with no trimming;
a bad grammar is `invalid_value` and an excessive length is `oversized` (S7 methodName
character set; length is QGI policy).

**`QGI-FLD-0026` — Scalar whitespace.** A reader MUST remove only surrounding XML
whitespace for numeric, datetime, nil and base64 conversion; string, untyped,
methodName, name and faultString text retain theirs.

**`QGI-FLD-0027` — Error precedence.** A reader MUST return only the first failure
from these ordered stages, with no events or state changes:

1. Body size; encoding detection; incomplete UTF-16 unit (`truncated`).
2. Declaration syntax (`malformed`); XML version support; encoding-label support
   (`unsupported_version`); encoding agreement (`malformed`).
3. Actual DOCTYPE/entity declarations (`malformed`), before structure/resource faults
   anywhere in document order; comments/CDATA/PI text cannot introduce declarations.
4. XML syntax and nonnumeric XML resources in document order, exactly the construct
   validation and resource ordering of `QGI-N1M-0053` stage 4.
5. Complete XML-RPC container grammar, then params/array/member bounds, then scalar
   numeric bounds, then scalar conversion, all in preorder within each stage.
   An unknown root bypasses XML-RPC stages; an unknown type bypasses only its subtree.
6. Method name, then supplied association validation, then E.4 dispatch and common
   conversion; only afterward check actual compact event size (`QGI-SER-0004`).

**`QGI-FLD-0028` — Fault shape.** A recognized fault MUST contain a struct with exactly
one member named `faultCode` of type int/i4 and exactly one `faultString` of string
or untyped type; other members are retained, duplicate required names or wrong
required types are `malformed` (S7 Fault example; uniqueness is QGI policy).

## E.4 Complete mapping catalog

S2's METHOD_LIST (lines 4195–4401) registers 174 distinct names, with duplicate
registrations of log.set_rst_in/out counted once. Every row below cites its S2
handler by class name. `n` in the argument position denotes **zero params**, not
a nil argument. The signature symbols are S1's: i int, b boolean, d double,
s string, 6 base64 bytes, A array, S struct, n nil/zero arguments.

**`QGI-FLD-0029` — Operations.** The family token for a method MUST be `fldigi.` plus
its name with dots replaced by underscores, except a leading `fldigi.` is removed
instead (thus `fldigi.version`, `fldigi.rig_get_frequency`, `fldigi.text_get_rx`).
Shared-operation exceptions appear in the mapped table.

**`QGI-FLD-0030` — Catalog.** For nonfault messages, a reader MUST apply the mapped
table below in both directions; all names in the subsequent native table have kind `native` in both
directions, irrespective of the observer's direction/exchange.

**`QGI-FLD-0031` — Mapped requests.** A matching mapped methodCall MUST use kind
`control`, its `0029` operation (or the table's shared exception), and only the
listed request arguments mapped positionally; a getter with no arguments carries
only operation.

**`QGI-FLD-0032` — Mapped results.** A matching mapped methodResponse MUST use the
table's kind and common fields; a `result` additionally carries `status: "ok"` and
the associated operation, while `radio_state` and `decode` carry no operation.

**`QGI-FLD-0033` — Snapshot isolation.** A codec MUST NOT combine polls into a QSO,
fill missing state from earlier responses, parse call/grid from RX text or ADIF,
or claim a log-field read is a newly logged contact (S2 Log_get_* read UI fields).

**`QGI-FLD-0034` — Frequency conversion.** A finite double MUST map to integer Hz
only if its binary64 value is an exact integer in the common range; otherwise
only that common member is omitted, without rounding or changing native bytes.

**`QGI-FLD-0035` — Frequency meaning.** `rig.get_frequency` and deprecated alias
`main.get_frequency` MUST map `wf->rfcarrier()` to dial_frequency_hz; their setters'
returned old value maps to `result.frequency_hz`, never confirmation of the new
frequency (S2 Rig_get_freq/Rig_set_frequency/Main_get_freq/Main_set_freq).

**`QGI-FLD-0036` — Audio offset.** `modem.get_carrier` MUST map only rx_offset_hz;
no tx_offset_hz is inferred because frequency lock can decouple it from transmit
(S2 Modem_get_carrier; S5 modem::set_freq/set_freqlock).

**`QGI-FLD-0037` — Units.** Frequency and carrier integers/doubles in this table
MUST be interpreted as Hz, with no magnitude-based unit selection (S2 RF handlers; S8–S10 unit derivation above; S6 qsy).

**`QGI-FLD-0038` — Transmit state.** `main.get_trx_state` MUST map `TX` to transmitting
true and `RX` to false, omitting it for `OTHER` or any other text; `main.get_trx_status`
MUST map `tx`/`tune` to true and `rx` to false, omitting other values, with no trimming
(S2 Main_get_trx_state/Main_get_trx_status; the latter now reads engine trx_state).

**`QGI-FLD-0039` — Native labels.** This profile MUST omit wire_id, radio, station_call
and station_grid: S2 exposes no station call/locator getter, and log.get_call/locator
are the contact fields, not station identity.

| Method | Signature | Request arguments / operation exception | Response kind / common member | Evidence (S2 handler) |
|---|---|---|---|---|
| `fldigi.version` | `s:n` | none | `result` / native_version under 0011 | Fldigi_version_string |
| `rig.get_frequency` | `d:n` | none | `radio_state` / dial_frequency_hz | Rig_get_freq |
| `main.get_frequency` | `d:n` | none | `radio_state` / dial_frequency_hz | Main_get_freq |
| `rig.set_frequency` | `d:d` | frequency_hz; `set_frequency` | `result` / frequency_hz | Rig_set_frequency |
| `main.set_frequency` | `d:d` | frequency_hz; `set_frequency` | `result` / frequency_hz | Main_set_freq |
| `modem.get_carrier` | `i:n` | none | `radio_state` / rx_offset_hz | Modem_get_carrier |
| `modem.set_carrier` | `i:i` | audio_offset_hz | `result` / no extra common member | Modem_set_carrier |
| `modem.get_name` | `s:n` | none | `radio_state` / mode | Modem_get_name |
| `modem.get_mode` | `s:n` | none | `result` / text | Modem_get_mode |
| `modem.get_submode` | `s:n` | none | `radio_state` / sub_mode | Modem_get_submode |
| `modem.set_by_name` | `s:s` | mode | `result` / text | Modem_set_by_name |
| `rig.get_mode` | `s:n` | none | `result` / text | Rig_get_mode |
| `rig.set_mode` | `n:s` | mode | `result` / no extra common member | Rig_set_mode |
| `main.get_trx_state` | `s:n` | none | `radio_state` / transmitting under 0038 | Main_get_trx_state |
| `main.get_trx_status` | `s:n` | none | `radio_state` / transmitting under 0038 | Main_get_trx_status |
| `main.tx` | `n:n` | none | `result` / no extra common member | Main_tx |
| `main.rx` | `n:n` | none | `result` / no extra common member | Main_rx |
| `main.abort` | `n:n` | none; `halt_tx` | `result` / no extra common member | Main_abort |
| `main.tune` | `n:n` | none | `result` / no extra common member | Main_tune |
| `rx.get_data` | `6:n` | none | `decode` / text under 0040 | RX_get_data |
| `text.get_rx` | `6:ii` | rx_start rx_length | `decode` / text under 0040 | Text_get_rx |
| `text.get_rx_length` | `i:n` | none | `result` / no extra common member | Text_get_rx_length |
| `text.add_tx` | `n:s` | text; `send_text` | `result` / no extra common member | Text_add_tx |
| `text.clear_rx` | `n:n` | none | `result` / no extra common member | Text_clear_rx |
| `text.clear_tx` | `n:n` | none | `result` / no extra common member | Text_clear_tx |
| `log.get_frequency` | `s:n` | none | `result` / text | Log_get_freq |
| `log.get_time_on` | `s:n` | none | `result` / text | Log_get_time_on |
| `log.get_time_off` | `s:n` | none | `result` / text | Log_get_time_off |
| `log.get_date_on` | `s:n` | none | `result` / text | Log_get_date_on |
| `log.get_date_off` | `s:n` | none | `result` / text | Log_get_date_off |
| `log.get_call` | `s:n` | none | `radio_state` / dx_call | Log_get_call |
| `log.get_name` | `s:n` | none | `result` / text | Log_get_name |
| `log.get_rst_in` | `s:n` | none | `result` / text | Log_get_rst_in |
| `log.get_rst_out` | `s:n` | none | `result` / text | Log_get_rst_out |
| `log.get_serial_number` | `s:n` | none | `result` / text | Log_get_serial_number |
| `log.get_serial_number_sent` | `s:n` | none | `result` / text | Log_get_serial_number_sent |
| `log.get_exchange` | `s:n` | none | `result` / text | Log_get_exchange |
| `log.get_state` | `s:n` | none | `result` / text | Log_get_state |
| `log.get_province` | `s:n` | none | `result` / text | Log_get_province |
| `log.get_country` | `s:n` | none | `result` / text | Log_get_country |
| `log.get_qth` | `s:n` | none | `result` / text | Log_get_qth |
| `log.get_band` | `s:n` | none | `result` / text | Log_get_band |
| `log.get_sideband` | `s:n` | none | `result` / text | Log_get_sb |
| `log.get_notes` | `s:n` | none | `result` / text | Log_get_notes |
| `log.get_locator` | `s:n` | none | `radio_state` / dx_grid | Log_get_locator |
| `log.get_az` | `s:n` | none | `result` / text | Log_get_az |
| `log.set_call` | `n:s` | dx_call | `result` / no extra common member | Log_set_call |
| `log.set_locator` | `n:s` | dx_grid | `result` / no extra common member | Log_set_locator |
| `logbook.last_record` | `s:n` | none | `result` / text | Logbook_last_record |
| `logbook.all_records` | `s:n` | none | `result` / text | Logbook_all_records |

The length result remains in native evidence (Annex A has no buffer-length result
member). Log frequency, dates, clocks, sideband, reports, serials, exchange, location
and notes use `result.text` distinguished by operation; no units/time zone are
guessed, and leading zeros survive. ADIF logbook reads use verbatim result text,
not QSO event generation. Modem name and ADIF mode are distinct S2 getters.

### Remaining pinned methods: native in both directions (QGI-FLD-0030)

| Method | Declared signature | Evidence (S2 handler) |
|---|---|---|
| `fldigi.list` | `A:n` | Fldigi_list |
| `fldigi.name` | `s:n` | Fldigi_name |
| `fldigi.version_struct` | `S:n` | Fldigi_version_struct |
| `fldigi.name_version` | `s:n` | Fldigi_name_version |
| `fldigi.config_dir` | `s:n` | Fldigi_config_dir |
| `fldigi.terminate` | `n:i` | Fldigi_terminate |
| `modem.get_io_names` | `A:n` | Modem_get_io_names |
| `modem.get_names` | `A:n` | Modem_get_names |
| `modem.get_id` | `i:n` | Modem_get_id |
| `modem.get_max_id` | `i:n` | Modem_get_max_id |
| `modem.set_by_id` | `i:i` | Modem_set_by_id |
| `modem.inc_carrier` | `i:i` | Modem_inc_carrier |
| `modem.get_afc_search_range` | `i:n` | Modem_get_afc_sr |
| `modem.set_afc_search_range` | `i:i` | Modem_set_afc_sr |
| `modem.inc_afc_search_range` | `i:i` | Modem_inc_afc_sr |
| `modem.get_bandwidth` | `i:n` | Modem_get_bw |
| `modem.set_bandwidth` | `i:i` | Modem_set_bw |
| `modem.inc_bandwidth` | `i:i` | Modem_inc_bw |
| `modem.get_quality` | `d:n` | Modem_get_quality |
| `modem.search_up` | `n:n` | Modem_search_up |
| `modem.search_down` | `n:n` | Modem_search_down |
| `modem.olivia.set_bandwidth` | `n:i` | Modem_olivia_set_bandwidth |
| `modem.olivia.get_bandwidth` | `i:n` | Modem_olivia_get_bandwidth |
| `modem.olivia.set_tones` | `n:i` | Modem_olivia_set_tones |
| `modem.olivia.get_tones` | `i:n` | Modem_olivia_get_tones |
| `main.get_status1` | `s:n` | Main_get_status1 |
| `main.get_status2` | `s:n` | Main_get_status2 |
| `main.get_sideband` | `s:n` | Main_get_sb |
| `main.set_sideband` | `n:s` | Main_set_sb |
| `main.get_wf_sideband` | `s:n` | Main_get_wf_sideband |
| `main.set_wf_sideband` | `n:s` | Main_set_wf_sideband |
| `main.inc_frequency` | `d:d` | Main_inc_freq |
| `main.get_afc` | `b:n` | Main_get_afc |
| `main.set_afc` | `b:b` | Main_set_afc |
| `main.toggle_afc` | `b:n` | Main_toggle_afc |
| `main.get_squelch` | `b:n` | Main_get_sql |
| `main.set_squelch` | `b:b` | Main_set_sql |
| `main.toggle_squelch` | `b:n` | Main_toggle_sql |
| `main.get_squelch_level` | `d:n` | Main_get_sql_level |
| `main.set_squelch_level` | `d:d` | Main_set_sql_level |
| `main.inc_squelch_level` | `d:d` | Main_inc_sql_level |
| `main.get_reverse` | `b:n` | Main_get_rev |
| `main.set_reverse` | `b:b` | Main_set_rev |
| `main.toggle_reverse` | `b:n` | Main_toggle_rev |
| `main.get_lock` | `b:n` | Main_get_lock |
| `main.set_lock` | `b:b` | Main_set_lock |
| `main.toggle_lock` | `b:n` | Main_toggle_lock |
| `main.get_txid` | `b:n` | Main_get_txid |
| `main.set_txid` | `b:b` | Main_set_txid |
| `main.toggle_txid` | `b:n` | Main_toggle_txid |
| `main.get_rsid` | `b:n` | Main_get_rsid |
| `main.set_rsid` | `b:b` | Main_set_rsid |
| `main.toggle_rsid` | `b:n` | Main_toggle_rsid |
| `main.rsid` | `n:n` | Main_rsid |
| `main.rx_tx` | `n:n` | Main_rx_tx |
| `main.rx_only` | `n:n` | Main_rx_only |
| `main.get_tx_timing` | `n:s` | Main_get_tx_timing |
| `main.get_char_rates` | `s:n` | Main_get_char_rates |
| `main.get_char_timing` | `n:i` | Main_get_char_timing |
| `main.set_rig_name` | `n:s` | Main_set_rig_name |
| `main.set_rig_frequency` | `d:d` | Main_set_rig_frequency |
| `main.set_rig_modes` | `n:A` | Main_set_rig_modes |
| `main.set_rig_mode` | `n:s` | Main_set_rig_mode |
| `main.get_rig_modes` | `A:n` | Main_get_rig_modes |
| `main.get_rig_mode` | `s:n` | Main_get_rig_mode |
| `main.set_rig_bandwidths` | `n:A` | Main_set_rig_bandwidths |
| `main.set_rig_bandwidth` | `n:s` | Main_set_rig_bandwidth |
| `main.get_rig_bandwidth` | `s:n` | Main_get_rig_bandwidth |
| `main.get_rig_bandwidths` | `n:A` | Main_get_rig_bandwidths |
| `main.run_macro` | `n:i` | Main_run_macro |
| `main.get_max_macro_id` | `i:n` | Main_get_max_macro_id |
| `rig.set_name` | `n:s` | Rig_set_name |
| `rig.get_name` | `s:n` | Rig_get_name |
| `rig.set_smeter` | `n:i` | Rig_set_smeter |
| `rig.set_pwrmeter` | `n:i` | Rig_set_pwrmeter |
| `rig.set_modes` | `n:A` | Rig_set_modes |
| `rig.get_modes` | `A:n` | Rig_get_modes |
| `rig.set_bandwidths` | `n:A` | Rig_set_bandwidths |
| `rig.set_bandwidth` | `n:s` | Rig_set_bandwidth |
| `rig.get_bandwidth` | `s:n` | Rig_get_bandwidth |
| `rig.get_bandwidths` | `A:n` | Rig_get_bandwidths |
| `rig.get_notch` | `s:n` | Rig_get_notch |
| `rig.set_notch` | `n:i` | Rig_set_notch |
| `rig.enable_qsy` | `n:i` | Rig_enable_qsy |
| `log.set_rst_in` | `n:s` | Log_set_rst_in |
| `log.set_rst_out` | `n:s` | Log_set_rst_out |
| `log.set_serial_number` | `n:s` | Log_set_serial_number |
| `log.set_contest_counter` | `n:s` | Log_set_contest_counter |
| `log.set_exchange` | `n:s` | Log_set_exchange |
| `log.clear` | `n:n` | Log_clear |
| `log.set_name` | `n:s` | Log_set_name |
| `log.set_qth` | `n:s` | Log_set_qth |
| `main.flmsg_online` | `n:n` | Main_flmsg_online |
| `main.flmsg_available` | `n:n` | Main_flmsg_available |
| `main.flmsg_transfer` | `n:n` | Main_flmsg_transfer |
| `main.flmsg_squelch` | `b:n` | Main_flmsg_squelch |
| `flmsg.online` | `n:n` | flmsg_online |
| `flmsg.available` | `n:n` | flmsg_available |
| `flmsg.transfer` | `n:n` | flmsg_transfer |
| `flmsg.squelch` | `b:n` | flmsg_squelch |
| `flmsg.get_data` | `6:n` | flmsg_get_data |
| `io.in_use` | `s:n` | Io_in_use |
| `io.enable_kiss` | `n:n` | Io_enable_kiss |
| `io.enable_arq` | `n:n` | Io_enable_arq |
| `text.add_tx_queu` | `n:s` | Text_add_tx_queu |
| `text.add_tx_bytes` | `n:6` | Text_add_tx_bytes |
| `rxtx.get_data` | `6:n` | RXTX_get_data |
| `tx.get_data` | `6:n` | TX_get_data |
| `spot.get_auto` | `b:n` | Spot_get_auto |
| `spot.set_auto` | `b:b` | Spot_set_auto |
| `spot.toggle_auto` | `b:n` | Spot_toggle_auto |
| `spot.pskrep.get_count` | `i:n` | Spot_pskrep_get_count |
| `wefax.state_string` | `s:n` | Wefax_state_string |
| `wefax.skip_apt` | `s:n` | Wefax_skip_apt |
| `wefax.skip_phasing` | `s:n` | Wefax_skip_phasing |
| `wefax.set_tx_abort_flag` | `s:n` | Wefax_set_tx_abort_flag |
| `wefax.end_reception` | `s:n` | Wefax_end_reception |
| `wefax.start_manual_reception` | `s:n` | Wefax_start_manual_reception |
| `wefax.set_adif_log` | `s:b` | Wefax_set_adif_log |
| `wefax.set_max_lines` | `s:i` | Wefax_set_max_lines |
| `wefax.get_received_file` | `s:i` | Wefax_get_received_file |
| `wefax.send_file` | `s:si` | Wefax_send_file |
| `navtex.get_message` | `s:i` | Navtex_get_message |
| `navtex.send_message` | `s:s` | Navtex_send_message |

## E.5 RX chunks and faults

**`QGI-FLD-0040` — Decode unit.** Each associated `rx.get_data` or `text.get_rx`
base64 response MUST produce one `decode` containing `text` equal to the returned
bytes interpreted as UTF-8, or their entire `utf8_bytes` wrapper if invalid;
empty bytes produce text `""` (S2 RX_get_data/Text_get_rx; S4 UTF-8 text buffer).

**`QGI-FLD-0041` — Stream boundaries.** A codec MUST NOT join chunks, split lines,
retain incomplete characters, deduplicate overlapping reads or infer callsigns,
mode, timestamps or RF frequency from a chunk; a decode here is a poll's text
fragment, not a demodulator frame or completed QSO.

**`QGI-FLD-0042` — Range semantics.** `text.get_rx` request integers MUST map in order
to family control arguments `rx_start` and `rx_length`, expressed as zero-based
byte offset and byte count; −1 in observed length retains S2's special request
for available text, without guessing the returned byte count (S2 Text_get_rx;
S4 length/text_range). No offsets are added to the decode fields.

**`QGI-FLD-0043` — Range association.** A range response MUST map to decode only when
its caller arguments are exactly two signed-32-bit integers, start at least zero
and length at least −1; return length disagreement does not modify returned bytes.

**`QGI-FLD-0044` — Polling responsibility.** Applications maintaining RX cursors
SHOULD use returned byte lengths and explicitly handle buffer clearing/shrinking,
concurrent changes and overlap; the codec supplies no cursor or exactly-once
claim (S2 Text_clear_rx, RX_get_data's since-last-query behavior).

**`QGI-FLD-0045` — Split text.** Text spanning calls MUST remain separate observations,
including separate raw wrappers for a UTF-8 character split between two calls;
an application may join original byte sequences outside the mapping.

**`QGI-FLD-0046` — Fault mapping.** A valid fault MUST produce `result` with
`status: "error"`, `native_code` equal to faultCode's processed lexical text
(including sign, zeros and surrounding whitespace), and `text` equal to
faultString under `0023` (S7 Fault example).

**`QGI-FLD-0047` — Fault operation.** A fault MUST include operation only if the caller
supplies an associated method, using its shared mapped exception or `0029` token
when that token matches Annex A's operation grammar; otherwise operation is omitted.

## E.6 Commands from fields and exact request bodies

**`QGI-FLD-0048` — Writer scope.** A command writer MUST support the mapped operations
in E.4 only, with the exact positional fields there; other operations or missing,
extra or wrongly typed arguments are `invalid_value`.

**`QGI-FLD-0049` — Writer envelope.** A writer MUST reject unsupported mapping revisions
as `unsupported_version` before argument checks, and reject native evidence,
native_version, wire_id, radio selectors and all nonempty encoding options as
`invalid_value`.

**`QGI-FLD-0050` — Shared frequency.** `set_frequency` MUST require integer frequency_hz
in 1–2,147,483,647 and optionally integer rx_offset_hz in 1–2,147,483,647,
encoding `rig.set_frequency` first and `modem.set_carrier` second only if the offset
is supplied; these conservative construction ranges are QGI policies for differing
platform/backend differences, not a claim that a rig accepts every value.

**`QGI-FLD-0051` — Shared text.** `send_text` MUST require valid XML string text and
boolean transmit, encoding `text.add_tx(text)` first and `main.tx()` second only
when transmit is true, including when text is empty.

**`QGI-FLD-0052` — Shared halt.** `halt_tx` MUST encode `main.abort()` with no arguments;
auto_tx_only is unsupported (S2 Main_abort stops TX/tune and requests AbortARQ).

**`QGI-FLD-0053` — Family arguments.** Family setter operations MUST require E.4's
single string argument, except `fldigi.modem_set_carrier` requires integer
audio_offset_hz in 1–2,147,483,647 and `fldigi.text_get_rx` requires integers
rx_start/rx_length in 0–2,147,483,647; all remaining mapped family operations take
no arguments. The writer deliberately excludes S2's −1 read shortcut.

**`QGI-FLD-0054` — Sequence result.** Encoding MUST return an ordered collection of
separate complete bodies, never a concatenated XML document, HTTP message or
system.multicall envelope.

**`QGI-FLD-0055` — Orchestration.** A caller executing a multi-request operation MUST
send each following request only after the preceding request has a successful
matching response of its E.4 type (double for rig.set_frequency, nil for
text.add_tx), stopping on a fault, wrong type, transport error or lost
association; it supplies separate request associations/correlation IDs and gets no
aggregate result, atomicity, rollback or retry guarantee from the codec.

**`QGI-FLD-0056` — Side effects.** Applications MUST interpret buffering/transmit effects
as S2 defines them: text.add_tx appends to the TX widget, main.tx requests transmit,
main.rx requests receive, main.tune requests tune and main.abort aborts TX/tune;
encoding or decoding performs none of these effects. Empty append does not clear
previous text, and appended text is not guaranteed on-air content.

**`QGI-FLD-0057` — Canonical document.** Every constructed body MUST be UTF-8 with no BOM,
exact declaration `<?xml version="1.0" encoding="utf-8"?>`, followed immediately by
`<methodCall><methodName>NAME</methodName><params>PARAMS</params></methodCall>`;
zero arguments use paired `<params></params>` and there is no trailing delimiter,
formatting newline or interelement whitespace.

**`QGI-FLD-0058` — Canonical params.** Each argument MUST be
`<param><value><TYPE>CONTENT</TYPE></value></param>` in the table's positional order,
using `string` for strings, `double` for frequency_hz and `int` for all other
writer integers; all tags are paired, including empty strings.

**`QGI-FLD-0059` — Canonical numbers.** A writer MUST write nonnegative ASCII decimal
integers without a sign or leading zeros except zero itself; a double writes the
same integer digits followed by exactly `.0`, never exponent notation.

**`QGI-FLD-0060` — Canonical text.** A writer MUST escape ampersand, less-than,
greater-than and CR as `&amp;`, `&lt;`, `&gt;` and `&#13;`, respectively, in one pass;
quotes, tabs and LF remain literal, with no Unicode normalization.

**`QGI-FLD-0061` — Command text.** A writer MUST reject null, raw wrappers, surrogates
and XML-forbidden characters in strings as `invalid_value`.

**`QGI-FLD-0062` — Command budget.** A writer MUST validate each constructed body's
E.3 limits before returning the collection, failing atomically as `oversized`.

For example, `send_text`, text `CQ Q1AAA`, transmit true produces exactly these
two UTF-8 bodies, separately and in this order (no final LF):

```xml
<?xml version="1.0" encoding="utf-8"?><methodCall><methodName>text.add_tx</methodName><params><param><value><string>CQ Q1AAA</string></value></param></params></methodCall>
<?xml version="1.0" encoding="utf-8"?><methodCall><methodName>main.tx</methodName><params></params></methodCall>
```

## E.7 Evidence view, re-encoding and output budget

**`QGI-FLD-0063` — Structured catalog.** If emitted, native.message MUST contain
exactly `root` (literal qualified XML root name), `xml` (the whole original document
decoded in its detected encoding, excluding only BOM, with the matching core wrapper
when invalid), and `method` only when readable from a call or supplied by response
association; no other view members are defined.

**`QGI-FLD-0064` — Whole-document text.** The optional xml view MUST preserve original
entity spellings and line endings, rather than the XML-processed common text.

**`QGI-FLD-0065` — Re-encoding.** Observed re-encoding MUST check recognized common
fields against the body and caller-supplied association, then return exact native
bytes; original-byte retrieval is separate and needs no association/consistency
check. Verification of method-dependent fields without association is unavailable
under `QGI-MAP-0015`, not evidence of disagreement.

**`QGI-FLD-0066` — Output.** Observers MUST use compact UTF-8 JSON escaping only required
characters (or an encoding no larger), E.7's closed view and the context/hop bounds
of `QGI-SER-0010`, enforcing the actual `QGI-SER-0004` byte limit atomically.

**`QGI-FLD-0067` — Schema scope.** The family-only control arguments rx_start and
rx_length MUST be common integers in fldigi control fields only; they add schema
capacity without changing Annex A's shared field meanings.

**`QGI-FLD-0069` — Missing response value.** A reader MUST reject an empty response
param as `malformed`, including S12's bundled-backend serialization of an invalid
result, without synthesizing nil or a successful acknowledgment (E.3 grammar).

**`QGI-FLD-0070` — Modem labels.** A reader MUST preserve the returned modem name,
ADIF mode and submode as their distinct E.4 strings, without translating renamed,
new or empty labels using another release's mode table (S2, S13, S14).

For body size N, exact evidence costs 4 × ceil(N / 3). Mapped scalar text, including native_version (at most
one copy of a mapped return value, or the distinct two fault leaves) costs at most 6N,
including JSON escapes/raw wrappers. Base64 RX decoding is smaller than its source
and fits that same bound. The optional whole-document xml view costs at most 6N,
including UTF-16 invalid-unit arrays. Root and caller method each have a 128-byte
name bound: reserve 2 × 6 × 128 = 1,536. Reserve 32,768 for names, punctuation,
wrappers, numeric scalars and envelope; this profile has fewer than 256 members,
with 128 bytes per member. Context and eight hop contexts use 65,536 more.

The resulting conservative worst-case guarantee is

`4 × ceil(N / 3) + 12N + 1,536 + 32,768 + 65,536 ≤ 1,048,576`.

Its **largest integer N is 71,155**: 94,876 + 853,860 + 1,536 + 32,768 + 65,536 =
**1,048,576**. N = 71,156 raises the bound to 1,048,588. This is the largest body
admitted by this sufficient proof, not an assertion that any particular XML body
attains every conservative reserve simultaneously. The text expansion budget
also prevents UTF-16-to-UTF-8 expansion from exceeding N's allowance.

## E.8 Live-capture questions (informative)

1. Which released builds enable/filter these methods, and which nil/scalar
   spellings occur in actual replies? S1's 4.2.00 manual is not this release pin.
2. How do RX buffers change under editing, clear, multiple clients and non-ASCII
   boundaries? S2's −1 path sets size to full length even for nonzero start; this
   profile's writer excludes it, and captures are needed before offering it.
3. What peer limits and exact rig/audio behavior apply on each platform, including
   frequency lock, retuning, sideband, mode changes and asynchronous GUI updates?
4. How do queued text, macros and empty appends affect on-air content in each mode?
5. What HTTP status/content-type conventions and fault codes occur across builds?

These questions do not weaken the deterministic profile rules above or claim
live interoperability. Rust implementation work and captures are separate work.
