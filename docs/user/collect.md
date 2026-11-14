<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Collect reception submissions

Use QG Interop as a local collector to inspect reception submissions from an app
or the built-in simulator. Each receiver or reception record becomes an NDJSON event.

Start the collector in one terminal:

```sh
qg-interop collect --pskreporter-ipfix
```

The default address is `127.0.0.1:4739`. To use another local port, supply a numeric
unicast address. Use brackets around an IPv6 address:

```sh
qg-interop collect --pskreporter-ipfix 127.0.0.1:14739 --count 1
qg-interop collect --pskreporter-ipfix '[::1]:14739' --count 1
```

Send one synthetic submission to the first collector from another terminal:

```sh
qg-interop sim --pskreporter-ipfix --to 127.0.0.1:4739 --profile wsjtx
```

Choose `--profile fldigi` for fldigi's template layout. Both profiles use invented
Q-prefixed callsigns. The destination is required and must be numeric unicast.
Add `--loop` to send successive submissions at intervals longer than five minutes.

Events go to standard output. Diagnostics go to standard error and name the
[Annex H.1 rule](../../spec/annex-h-pskreporter-ipfix.md#h1-published-submitter-rules),
Set, and record when known. For example, repeated submissions within five minutes
can produce `H.1-INTERVAL` and `H.1-DUPLICATE` diagnostics. fldigi's published source
layout produces an `H.1-SCOPE` diagnostic for its legacy zero scope count.

To save events separately from diagnostics:

```sh
qg-interop collect --pskreporter-ipfix > receptions.ndjson 2> collector.log
```

A `receiver_information` event describes a receiver. A `spot` describes a reception;
its receiver fields come from one receiver record in that same message. Native evidence
keeps each record, its template, and the complete datagram for inspection.
