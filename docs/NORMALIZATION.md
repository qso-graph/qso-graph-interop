<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Identity, direction, and protocol adapters

`Message<F = wsjtx::Packet>` separates normalized values, caller context, and the complete native
family representation. The default preserves the existing binary protocol; another adapter can
use a parsed XML/JSON envelope plus original bytes as `F`. Schema belongs to the
family, not to the transport-independent envelope. `client_id` preserves the neutral wire ID,
including null, empty, and invalid UTF-8. It is not a peer identity or an authenticated source.
Protocols without an application ID use a null `Utf8`; their family retains the exact distinction
between absent and explicitly null fields.

`Context` has direction relative to the application (`Unknown`, `FromApplication`, `ToApplication`),
optional caller-supplied source/target identities, request/response/notification role, and an
optional correlation ID. The caller obtains endpoint knowledge from its transport/session; the
codec opens no connection and does not infer that knowledge from an ID. Direction and exchange
role are independent: an application can originate notifications, requests, and responses.

`Message::from(packet)` uses unknown context. WSJT-X Clear and Close become `Kind::Ambiguous`, even
when Clear has no window: the legacy inbound command has no window too. `with_context` maps Clear
from the application to a notification and Clear to the application to a control. Close from the
application is lifecycle notification; Close to it is a shutdown control. Reply and Configure
have protocol-defined target IDs in `Control::target_id`, copied from `client_id`; neither becomes
a source ID. Context identities remain exactly what the caller supplied.
Normalization does not validate or execute a command.
Heartbeat is lifecycle
in either direction. Every normalization path retains the original packet, and `into_family`
returns it without applying edits to normalized views. See WSJT-X v3.0.2
[Clear/Reply](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L235) and
[Close](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L321).

The following are adapter design mappings, not additional protocol implementations:

| Adapter | Direction and identity | Normalized content and native preservation |
| --- | --- | --- |
| [N1MM XML over UDP](https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/) | A configured receive endpoint supplies `FromApplication` and notification context. An XML app/station label is metadata, not evidence of origin. A contact record ID remains a record ID in the family. | Map supported contact fields to LoggedQso and radio reports to RadioState. Keep XML attributes, units, missing/empty fields, and original bytes in the family; use Native for unsupported reports. |
| [JS8Call JSON over TCP/UDP](https://github.com/js8call/js8call) | The session tells the adapter whether a message is from or to JS8Call. Request correlation is separate from client/source identity; receiving JSON does not by itself mean a notification. | Map explicit radio/decode fields to the common views; commands use Control, unmatched payloads use Native. Keep message type, correlation fields, and the full JSON envelope in the family. |
| [fldigi XML-RPC](https://www.w1hkj.org/FldigiHelp/xmlrpc_control_page.html) | ToApplication requests and FromApplication responses share a caller-created correlation ID for the RPC exchange. XML-RPC has no general client ID: leave it null. | Map getter results only using the associated method and its defined units. Setter requests are controls; faults and unsupported methods use Native. Preserve the complete method call/response, including fault data and original XML, in the family. |

No adapter needs to introduce sockets into this model or treat every incoming value as an
observation. Optional time fields are populated only when the adapter has enough information. If a common
view requires a field the native record lacks, use Native rather than inventing a value; raw
fields remain in the family.

## JSON float preservation

Finite binary64 fields use JSON numbers; nonfinite values use `ieee754:` plus 16 hexadecimal bit
digits. All project serde_json dependencies enable `float_roundtrip`. Downstream consumers that
use serde_json must enable that feature too when deserializing packets or normalized views:

```toml
serde_json = { version = "=1.0.145", features = ["float_roundtrip"] }
```

Other JSON readers must correctly round binary64 numbers to preserve finite bits. This is a
reader requirement, not a serializer-independent exactness guarantee. Preserve the binary packet
when a downstream JSON implementation cannot provide it. Tests cover boundary values, both
signed zeros, 10,000 deterministic random bit patterns, and the review's `3f99867e711c7600` case;
the fuzz target covers both packet and common-model JSON paths.
