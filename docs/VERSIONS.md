<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Pinned version matrix

This file is the single source of truth for every version pin in FerroCHART.
When it and a file that repeats a pin disagree, that is drift. Fix the
disagreement; never let either side silently win.
`scripts/checks/versions.sh` enforces the cross-file agreement it can reach and
skips loudly for the files that do not exist yet, so the guard is useful on a
tree with no Cargo workspace and grows teeth as files appear.

No specification governs this file; it is FerroCHART's own design.

## Specifications

FerroCHART is in its design phase and has pinned no specification release yet.
The research on
[issue #1](https://github.com/rubentalstra/FerroCHART/issues/1) chooses each
one and records why in `docs/architecture.md` §2, and the rows below then carry
the value. A pin written here before that research is a guess, and a guess in
this file is worse than an empty row.

| Item | Pin | Repeated in |
|---|---|---|
| openEHR RM | pending #1 | `docs/architecture.md`, later the model crates |
| openEHR AM | pending #1 | `docs/architecture.md`, later the template reader |
| openEHR ITS-REST | pending #1 | `docs/architecture.md`, later the CDR client |
| openEHR AQL | pending #1 | `docs/architecture.md`, later the read-back path |

ADL 1.4 against ADL 2 is part of that decision, not a separate row, because it
follows from the AM release chosen.

## Corpora and machine-readable inputs

A corpus is pinned by commit or immutable tag, never by a moving tag or a
`latest` URL, and vendored by a committed `scripts/vendor/*.sh` with a
`PROVENANCE.md` (`.claude/rules/vendored-inputs.md`). Nothing is vendored yet.

The fixture templates FerroCHART compiles in its tests are a corpus like any
other: they are pinned by commit, they carry provenance, and they are never
hand-edited to make a test pass.

## Model crates

Whether the openEHR model comes from the published `openehr-*` crates, from a
generator in this repository, or from neither is open (`.claude/rules/codegen.md`).
No crate is pinned until that is decided.

## Language and runtime

No Cargo workspace exists yet. The toolchain pin below is live because
`rust-toolchain.toml` is committed; the rest land with the workspace.

| Item | Pin | Repeated in |
|---|---|---|
| Rust toolchain | 1.98.1 | `rust-toolchain.toml` `channel` (stable) |
| Edition | 2024 | root `Cargo.toml` `[workspace.package]` `edition` |
| Cargo resolver | 3 | root `Cargo.toml` `[workspace]` `resolver` |
| MSRV | 1.98 | root `Cargo.toml` `[workspace.package]` `rust-version` |

The deliverable is a server binary, so the MSRV tracks the pinned stable
toolchain.

## Product and citation version

The product version is the workspace `version` in the root `Cargo.toml`, which
every member inherits. The milestone line is 0.0.x, so the first product
version is 0.0.1.

| Item | Pin | Repeated in |
|---|---|---|
| Product version | 0.0.1 | root `Cargo.toml` `[workspace.package]` `version`, `CITATION.cff` `version` |

`CITATION.cff` tracks this row exactly, and the guard compares the two whenever
`CITATION.cff` exists. Once the root `Cargo.toml` lands, the guard also compares
its `[workspace.package]` `version` with both.

## Licence

| Item | Pin | Repeated in |
|---|---|---|
| Project licence | BUSL-1.1 | `LICENSE`, `NOTICE`, the SPDX header of every first-party file, later the `license` field of every own `Cargo.toml`, the container `image.licenses` label, the README badge |

`LICENSE` is the one file that names Apache License 2.0 as a licence of its
own, as the Change License four years after each version. The guard fails when
any other first-party file claims MIT or Apache-2.0 as its own licence.

Third-party and vendored material keeps its upstream terms, recorded beside the
vendored tree (`.claude/rules/vendored-inputs.md`).

## Rust dependency pins

The root `Cargo.toml` `[workspace.dependencies]` table is the authoritative,
fully pinned third-party crate set once it exists. This file does not duplicate
crate versions; on any discrepancy the manifest wins. A crate joins a member
with `dep.workspace = true`.

## CI tool pins

The tier-1 lanes of `.github/workflows/ci.yml` run four analyzers, each pinned
to an exact version so a CI result matches the local one. `zizmor` and
`shellcheck` are fetched by `taiki-e/install-action`, which verifies the
upstream release checksum; `actionlint` and `hadolint` run from their official
container images, pinned by tag and by digest.

| Item | Pin | Repeated in |
|---|---|---|
| `zizmor` | 1.29.0 | `.github/workflows/ci.yml` |
| `actionlint` | 1.7.12 | `.github/workflows/ci.yml` |
| `shellcheck` | 0.11.0 | `.github/workflows/ci.yml` |
| `hadolint` | 2.15.1 | `.github/workflows/ci.yml` |

Keep the locally installed versions on these numbers, so a finding costs a
local run rather than a CI round trip (`.claude/rules/ci-cd.md`).

## GitHub Actions pins

Every `uses:` in `.github/workflows/**` is pinned to a full commit SHA with a
trailing `# vX.Y.Z` comment (`.claude/rules/ci-cd.md`). Dependabot bumps them,
and zizmor checks the form.
