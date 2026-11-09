<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# qg-udp

Watch WSJT-X UDP messages and send a simulated radio exchange to a chosen destination.

```sh
qg-udp listen --wsjtx
qg-udp sim --wsjtx --to 127.0.0.1:2237
```

Use `--json` on the listener for JSON lines.
Run `qg-udp --help`, `qg-udp listen --help`, or `qg-udp sim --help` for examples.

Licensed under GPL-3.0-or-later.
