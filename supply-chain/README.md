<!-- SPDX-License-Identifier: GPL-3.0-only -->
# Dependency review store

cargo-vet owns the formatting of config.toml, audits.toml, and imports.lock; even a leading SPDX
comment in those generated files is rejected by `cargo vet --locked`. Keep them in its canonical
format with `cargo vet fmt`. Project policy and notes are licensed GPL-3.0-only. Imported audits
retain their upstream attribution.

The four audit imports, exact-version exemptions, and review status are described in
[docs/DEPENDENCIES.md](../docs/DEPENDENCIES.md). An exemption explicitly means not audited yet.
