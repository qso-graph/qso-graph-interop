<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex G: PSK Reporter MQTT placeholder

Reserved family: `pskreporter-mqtt`. This is a scope placeholder, not a complete mapping or
conformance claim. It profiles PSK Reporter's live spot feed, distinct from reception-report export.

| Item | Scope to be completed |
|---|---|
| Native message | One MQTT PUBLISH: topic and payload, plus the delivered retain flag. |
| Framing | The caller supplies a complete PUBLISH; payloads are not delimited by JSON newlines. The publish/subscribe class uses `native.format: "mqtt_publish"` and `QGI-CLS-0010`–`0013`. |
| Sending direction | Feed publications are decoded from the service. No application control or publication-to-service direction is established here. SUBSCRIBE is transport, not a command event. |
| Source to pin | Dated, hashed copies of the [developer page](https://pskreporter.info/pskdev.html), the [feed documentation](https://www.mqtt.pskreporter.info/) and its linked [timing and receiver-info document](https://www.mqtt.pskreporter.info/timing.html), consulted 2026-11-12. These live pages are not yet a mapping pin. |

The feed document describes JSON spot payloads and the `pskr/filter/v2/` topic hierarchy. It lists
`rc`, `rl`, `sc`, `sl`, `f`, `md`, `rp` and `t`, suggesting Annex A's receiver/sender, frequency,
mode, SNR and instant fields. These are candidate mappings only; topic/payload disagreement and
absent/empty handling remain to be specified. [Feed payload documentation](https://www.mqtt.pskreporter.info/)

The linked document distinguishes receiver-reported `t` from derived `t_tx`. It also documents
retained `pskr/rxinfo/` receiver metadata with decoder software and zero-length deletion payloads.
A spot PUBLISH does not establish that software by itself. This codec cannot fetch metadata or
silently join a different PUBLISH to a spot. Whether to expose metadata separately or add a
bounded, evidence-preserving join is an open annex question. No universal transmission-start
meaning is assigned to `t`. [Timing and receiver information](https://www.mqtt.pskreporter.info/timing.html)

Still to establish: a stable dated/hash pin, complete topic variants and payload schema, feed
version handling, exact optional fields, numeric bounds, retained spot behavior, empty metadata
behavior and the software association. No broker connection, subscription or test publication was
made during this review. HTTPS document reads do not validate broker behavior.

Radio parts have no established mapping in this placeholder (`QGI-MOD-0014`); their native
source and per-kind availability remain annex work. The family output limit and expansion proof
required by `QGI-MAP-0003` are also unestablished; no conformance is claimed for an assumed limit.
