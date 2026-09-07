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

The research on
[issue #1](https://github.com/rubentalstra/FerroCHART/issues/1) chose each
release and records the ground in `docs/architecture.md` §2. A component
release carries documents at different maturity levels inside one number, so a
citation names the component release, the document, and the section.

| Item | Pin | Repeated in |
|---|---|---|
| openEHR RM | Release-1.1.0 | `docs/architecture.md` §2, the model crates |
| openEHR AM | Release-2.3.0 | `docs/architecture.md` §2, the template readers |
| openEHR ITS-REST | Release-1.1.0 | `docs/architecture.md` §2, the CDR client |
| openEHR AQL | Release-1.1.0 | `docs/architecture.md` §2, the read-back path |
| openEHR BASE | Release-1.2.0 | `docs/architecture.md` §2, the path syntax |
| openEHR ITS-XML | 2.0.0 | `docs/architecture.md` §2, the OPT 1.4 XSD family |
| openEHR TERM | Release-3.0.0 | `docs/architecture.md` §2, the terminology groups |
| HL7 FHIR | R4 4.0.1 | `docs/architecture.md` §2, the terminology client |

Both template generations are read (`docs/architecture.md` §3), so the AM
release covers ADL 1.4, AOM 1.4, ADL 2 and AOM 2 together. The AM release
number and the version of a document inside it are separate numbers and are
never conflated in a citation.

## Corpora and machine-readable inputs

A corpus is pinned by commit or immutable tag, never by a moving tag or a
`latest` URL, and vendored by a committed `scripts/vendor/*.sh` with a
`PROVENANCE.md` (`.claude/rules/vendored-inputs.md`). Nothing is vendored yet.

The fixture templates FerroCHART compiles in its tests are a corpus like any
other: they are pinned by commit, they carry provenance, and they are never
hand-edited to make a test pass.

## Corpora

Each corpus is pinned by commit or by a per-artefact immutable identity, never
by a moving reference, and fetched by a committed `scripts/vendor/*.sh` with a
`PROVENANCE.md` beside it (`.claude/rules/vendored-inputs.md`).

| Item | Pin | Repeated in |
|---|---|---|
| openEHR CKM templates | per template `cid` | `corpus/templates/ckm/PROVENANCE.md` |
| openEHR ADL 2 archetypes | `093c77ea003742b9540e3dd377d615e2b26f2996` | `scripts/vendor/adl2-archetypes.sh`, `corpus/archetypes/adl2/PROVENANCE.md` |

The CKM template pack is committed for the exports that state a licence. The
ADL 2 pack is fetched and never committed, because one of its 652 archetypes
states a licence and the rest state none.

## Model crates

The openEHR model comes from the published `openehr-*` crates
(`docs/architecture.md` §2). They are published on crates.io and model the
specification; the CDR they are published from is never a dependency here. The
line releases in lockstep, and each `0.0.x` patch is its own compatibility
set, so the rows below move together.

| Item | Pin | Repeated in |
|---|---|---|
| openehr-base | 0.0.61 | `docs/architecture.md` §2, root `Cargo.toml` |
| openehr-rm | 0.0.61 | `docs/architecture.md` §2, root `Cargo.toml` |
| openehr-am | 0.0.61 | `docs/architecture.md` §2, root `Cargo.toml` |
| openehr-adl | 0.0.61 | `docs/architecture.md` §2, root `Cargo.toml` |
| openehr-its | 0.0.61 | `docs/architecture.md` §2, root `Cargo.toml` |
| openehr-query | 0.0.61 | `docs/architecture.md` §2, root `Cargo.toml` |

The set mixes licences: `openehr-base`, `openehr-rm` and `openehr-am` are
Apache-2.0, `openehr-adl` and `openehr-query` are BUSL-1.1, and `openehr-its`
is BUSL-1.1 and Apache-2.0. `openehr-term` arrives with them, under Apache-2.0
and CC-BY-SA-3.0 for the openEHR support terminology it embeds. `deny.toml`
allows exactly this set and no more.

## The FHIR model crate

The FHIR model comes from the published `fhir-types` crate
(`docs/architecture.md` sections 2 and 7.3). It is generated from the HL7 FHIR
packages and released from FerroTERM, which is never a dependency here.

| Item | Pin | Repeated in |
|---|---|---|
| fhir-types | 0.1.85 | `docs/architecture.md` §2, root `Cargo.toml` |

Two lockstep lines now feed this project. `fhir-types` releases together with
`fhir-terminology` and `sct-ecl` on one FerroTERM version, the way the
`openehr-*` crates release together on one FerroEHR version, so bumping any
crate of either family means bumping every crate this project takes from it.
FerroCHART takes only `fhir-types` from the FerroTERM line, and the reason the
other two are left is in `docs/architecture.md` section 7.3.

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

## Documentation toolchain

The book under `website/book` is rendered by the pinned mdBook toolchain that
`.github/actions/docs-toolchain` installs, and `docs.yml` publishes it to
GitHub Pages.

| Item | Pin | Repeated in |
|---|---|---|
| mdBook | 0.5.4 | `.github/actions/docs-toolchain/action.yml` `mdbook-version` |
| mdbook-toc | 0.15.4 | `.github/actions/docs-toolchain/action.yml` `mdbook-toc-version` |
| mdbook-mermaid | 0.17.1 | `.github/actions/docs-toolchain/action.yml` `mdbook-mermaid-version` |

## Product and citation version

The product version is the workspace `version` in the root `Cargo.toml`, which
every member inherits. The milestone line is 0.0.x, so the first product
version is 0.0.1.

| Item | Pin | Repeated in |
|---|---|---|
| Product version | 0.0.4 | root `Cargo.toml` `[workspace.package]` `version`, `CITATION.cff` `version` |

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

## Release lane tool pins

The release lanes install their tools by exact version too, for a stronger
reason than reproducibility: a tool that runs inside the isolated build lane
writes a document that lane then signs, so an unpinned tool is an unsigned
input to a signed artifact.

| Item | Pin | Repeated in |
|---|---|---|
| `cargo-auditable` | 0.7.5 | `.github/workflows/release-build.yml` |
| `cargo-cyclonedx` | 0.5.9 | `.github/workflows/release-build.yml` |
| `syft` | v1.51.1 | `.github/workflows/release-image.yml` |

## GitHub Actions pins

Every `uses:` in `.github/workflows/**` is pinned to a full commit SHA with a
trailing `# vX.Y.Z` comment (`.claude/rules/ci-cd.md`). Dependabot bumps them,
and zizmor checks the form.
