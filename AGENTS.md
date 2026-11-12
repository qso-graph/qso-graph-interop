<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Working on QG Interop

<!-- qso-graph:shared:start -->
<!-- Maintained in qso-graph/.github; change this block there. -->
- Keep credentials and keys in the OS credential store at rest; never put them in code, config, logs, tool results, issues, or pull requests.
- Use HTTPS for every connection to an outside service, never disable TLS verification, and validate all input.
- Report security vulnerabilities through private vulnerability reporting, never in a public issue.
- Keep one open pull request per repository, targeting `main`; the next change waits for it or joins it.
- Use the pull request template headings: Issues, What changed, Verification, and Credentials.
- Add each change under `## [Unreleased]` in `CHANGELOG.md`.
- Bump versions only in a release pull request; merging publishes nothing, and release is a separate step.
- Install, run, and develop Python with uv (`uvx`, `uv tool install`, `uv add`, `uv run`); documentation never tells users to use pip.
- Make `README.md` the product page: what it is, who it is for, what it does, installation, and first use.
- Keep caveats and work in progress out of `README.md`.
- Put user guides and images in `docs/user/`; use the rest of `docs/` for developers.
- Expect qso-graph.io to show each product's README and `docs/user/` at its latest Release, so documentation goes live by releasing.
- Follow the [Google Developer Documentation Style Guide](https://developers.google.com/style) and use American English.
- Check user-facing documentation with `vale README.md docs/user` using the organization's Vale configuration.
- See [TEMPLATES.md](https://github.com/qso-graph/.github/blob/main/TEMPLATES.md) for what every public repository carries and the release process.
<!-- qso-graph:shared:end -->

## This repository

The library implements the protocol; its caller chooses destinations and control policy.
Public files describe this project only. Use American English and synthetic test QSOs.

- Invented callsigns use the Q prefix.
- Rust 1.99.1, edition 2024; compatible dependency ranges; keep both lockfiles current.
- Every crate root, including tests and fuzz targets, says `#![forbid(unsafe_code)]`.
  Every workspace member inherits the workspace lints. Our own code has no unsafe Rust
  and defines no macros. Dependencies may use unsafe Rust and define macros; they must
  come from crates.io, contain no native code, and pass cargo-vet, cargo-deny, and cargo-audit.
- Fuzzing is separate developer tooling on nightly-2026-10-30, with libFuzzer's runtime only
  in its isolated workspace. Do not make shipped crates depend on it.
- The library opens no sockets or processes. The CLI uses only UDP at the numeric addresses
  the user supplies; the listener defaults to 127.0.0.1:2237. Do not add discovery or DNS.
- Preserve null, empty, absent, raw bytes, all family fields, and trailing extensions.
  Never guess a new schema's field encoding or infer a variant from its sender ID.
- Pin protocol changes to released source; keep evidence in docs/PROTOCOL.md and independent
  fixtures in crates/qso-graph-interop/tests/fixtures. New checks have planted failure cases.
- Keep the README a product page. Put validation details and limitations in docs/ or SECURITY.md.
- SPDX GPL-3.0-or-later headers on new files, using that format's comment syntax.

Run every check with one command before you report work as done:

```sh
cargo xtask verify                    # all checks plus 60 s of fuzzing; one pass/fail table
cargo xtask verify --fuzz-seconds 0   # the same without fuzzing
```

It runs formatting, Clippy, tests (full and core only), policy, fixtures, the specification
vectors, cargo-vet, cargo-deny, cargo-audit and the fuzz smoke, and fails if a lockfile changed.
Logs are in `target/verify/`. Report its table, not individual commands.

For the dated-nightly fuzz commands, see CONTRIBUTING.md. CI checks Linux, macOS, and Windows,
policy, dependency checks, fixtures, and fuzz smoke; ci-all-green requires every job to pass.
