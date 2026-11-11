<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# qg-interop

Watch WSJT-X UDP messages and send a simulated radio exchange to a chosen destination.

```sh
qg-interop listen --wsjtx
qg-interop sim --wsjtx --to 127.0.0.1:2237
```

Use `--json` on the listener for JSON lines.
Run `qg-interop --help`, `qg-interop listen --help`, or `qg-interop sim --help` for examples.

Licensed under GPL-3.0-or-later.
