<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Release QG Interop

KI7MT runs `.github/workflows/release.yml` manually from `main`. Merging publishes
nothing. The workflow builds and signs before publishing the library and CLI to
crates.io, tags only after both packages are available with matching checksums,
and creates the GitHub Release last. The FFI crate is never published to crates.io.

## One-time setup

1. Protect `main` with pull requests and the required `ci-all-green` check.
   Create the `windows-signing`, `macos-signing`, and `crates-io` Environments in
   `qso-graph/qso-graph-interop`. Restrict each Environment to `main`.
2. Put `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, and `AZURE_SUBSCRIPTION_ID` in
   `windows-signing`. Configure the signing app's Azure federated credential for
   issuer `https://token.actions.githubusercontent.com`, audience
   `api://AzureADTokenExchange`, and this exact subject:

   ```text
   repo:qso-graph@265651030/qso-graph-interop@1450652267:environment:windows-signing
   ```

   Reuse the signing account `qsographsigning`, profile `qso-graph`, and endpoint
   `https://wus2.codesigning.azure.net`. These are names, not credentials.
3. Give this repository visibility to the six Apple organization secrets:
   `CSC_LINK`, `CSC_KEY_PASSWORD`, `APPLE_API_KEY_P8`, `APPLE_API_KEY_ID`,
   `APPLE_API_ISSUER`, and `APPLE_TEAM_ID`. The release forwards those six by
   name to the shared `sign-macos.yml`, never through `secrets: inherit`. The
   Azure IDs are not forwarded: the shared `sign-windows.yml` job runs in this
   repository's `windows-signing` Environment and reads them there.
4. For the first publish, put `CRATES_IO_FIRST_PUBLISH` in `crates-io`: a
   short-lived crates.io token permitted to create `qso-graph-interop` and
   `qso-graph-interop-cli`. Only a crate not yet on crates.io uses it. After that
   run creates the crates, configure Trusted Publishing on **each** crate for
   organization `qso-graph`, repository `qso-graph-interop`, workflow
   `release.yml`, Environment `crates-io`. Delete the first-publish secret.
   Later releases reject a leftover first-publish secret once both crates exist.
   An interrupted first publish can resume: existing crates require Trusted
   Publishing, and only the still-new crate uses the first-publish token.
5. Add this line to `qso-graph-packages/products.tsv` in its own reviewed change:

   ```text
   qso-graph/qso-graph-interop  qso-graph-interop  no
   ```

   Stable releases attach `qso-graph-interop-<version>-linux-{amd64,arm64}.deb`
   and `qso-graph-interop-<version>-linux-{x86_64,aarch64}.rpm`; every filename
   appears in `SHA256SUMS.txt`. The packages repository signs and serves them.

## Run a release

1. Prepare a release pull request: update the workspace version and CLI/FFI
   library dependency versions, update both lockfiles, and date the changelog.
   Versions are `X.Y.Z` or `X.Y.Z-beta.N`. Keep package copies current with
   `uv run --no-project python scripts/package-files.py`. Run:

   ```sh
   source /home/ki7mt/workspace/qso-graph/qso-graph-sdk/prefix/qgsdk-env.sh
   cargo xtask verify --fuzz-seconds 10
   ```

2. Merge after CI passes. The release checks require a successful same-repository
   pull request CI run whose tree exactly matches `main`; the operator and
   rerun operator must both be KI7MT.
3. Open **Actions > Release > Run workflow**, select `main`, enter the version
   already in the manifests, and leave `dry_run` checked. This packages the
   crates and builds, signs, notarizes, assembles, and attests the downloads.
   It publishes no crates, tag, or GitHub Release. Inspect all four final
   platform artifacts: four CLI archives, four C bundles, and four Linux packages.
4. Run the same workflow and version with `dry_run` unchecked. Check both
   crates.io versions, tag `v<version>`, and the GitHub Release. Betas are
   prereleases. The release verifies all 12 download attestations and adds an
   attested `SHA256SUMS.txt`. A stopped run can be rerun; an existing package
   must match exactly and an existing tag must point to the same commit.

Linux CLI downloads are static musl executables. Linux C bundles use GNU targets
on Ubuntu 22.04 runners and require glibc 2.35 or later. Confirm that baseline for
C consumers before releasing; static C consumers also need their platform's system
link libraries. macOS bundles target Apple silicon; Windows bundles target x86_64
MSVC and include separate DLL import and static libraries.
