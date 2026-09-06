<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Maintenance rule: every pull request that changes user-visible behaviour adds
an entry under **[Unreleased]** in the same PR. Cutting a release renames
[Unreleased] to the version and date, and adds a fresh link reference.

The 0.0.x line carries the design of record, the repository and its gates, and
the engine as it is built. `docs/architecture.md` is the design, and
`docs/architecture.md` section 14 is the build order each milestone follows.
A release publishes binaries for four Linux targets; what they do grows with
the build order.

## [Unreleased]

### Added

- A fetch script for the ADL 2 archetype library (#49), pinned to a commit of
  `openEHR/adl-archetypes`. It brings 322 ADL 2 archetypes and 330 ADL 1.4
  twins, 321 of them the same archetype in both dialects, which is the first
  real input for the property that both readers fill one internal constraint
  model. Nothing from it is committed: one of the 652 files states a licence
  and the rest state none, so the tree is fetched into a directory
  `.gitignore` refuses and `PROVENANCE.md` records the omission.

## [0.0.2] - 2026-09-07

### Added

- A `./.github/actions/setup-rust` composite action (#33). The six gating jobs
  in `ci.yml` and the coverage job in `sonar.yml` call it instead of each
  carrying its own pinned toolchain step. The release lane keeps its own,
  because a publishing lane restores no cache, and now says so at the step.
- The internal constraint model and the two template readers that fill it
  (#13, #14), in `ferrochart-compile`. Both ADL generations normalize into one
  model, so the field derivation is written once and nothing above that point
  can tell which reader produced a node: each node carries a node identity, a
  Reference Model type, an occurrences interval, a constraint payload, the
  terminology in scope, and the five parts an overlay key step needs
  (`docs/architecture.md` sections 3 and 6.2). The ADL 1.4 reader walks a
  parsed `.opt`, resolving `use_node` internal references the vendor flattener
  left in place and reading the constrainers only the ITS-XML family declares
  (`C_DV_STATE`, `C_CODE_REFERENCE`, `T_COMPLEX_OBJECT`). The ADL 2 reader
  starts from source archetypes, flattens them and generates the operational
  form itself, folding the `C_PRIMITIVE_TUPLE` shapes for quantity and ordinal
  into the same payloads the ADL 1.4 domain types produce, and carrying
  `constraint_status`, which ADL 1.4 cannot state. A node with
  `occurrences matches {0}` is absent from the model, and a template that
  requires content at a slot it never filled is refused with a typed error
  naming the slot rather than rendered as a guess. 121 of the 123 committed CKM
  templates read; the two refusals are required unfilled slots.
- A crate-scoped `deny.toml` licence exception for `openehr-term`, which
  embeds the openEHR support terminology under CC-BY-SA-3.0 and reaches this
  tree through `openehr-its` and `openehr-adl`.
- The release supply chain (#39, #34): a `v*` tag now publishes a container
  image, SBOMs, checksums and Sigstore attestations beside the binaries, and a
  quickstart `compose.yaml` a downloader can run without a clone. The binaries
  and the image are built in reusable workflows called by jobs that carry no
  steps of their own, which is what the SLSA Build Level 3 claim rests on;
  `cargo auditable` writes the dependency list into every shipped binary,
  CycloneDX describes it beside the archive, and syft describes each image
  platform from that embedded list. A consumer verifies with
  `gh attestation verify --signer-workflow`, and `finalize-release` refuses to
  publish a draft missing any of the 27 assets a four-target release promises.
  The compose file's `demo` profile starts FerroEHR and FerroTERM alongside, and
  its header says plainly that those are separately licensed products and that
  the profile is for evaluation.
- The server's runtime surface (#41): one `FERROCHART_` environment namespace
  read in one place, a loopback default bind that only the container image
  widens, a refusal to start when either upstream endpoint is missing that
  names the variable, an unauthenticated `GET /health`, and orderly shutdown
  on SIGTERM. The book gains an Operate section describing all of it.
- The documentation lane (#4): a book under `website/book` rendered by the
  pinned mdBook toolchain that `.github/actions/docs-toolchain` installs, and
  `docs.yml`, which verifies it on every pull request and publishes it to
  GitHub Pages from `main`. `docs/VERSIONS.md` carries the three tool pins and
  `scripts/checks/versions.sh` stops skipping that check.
- The vendored openEHR CKM template corpus, licence by licence (#12).

### Fixed

- Status text that had gone stale as the repository gained code: the README no
  longer says there is no code and no binary, `docs/ci-cd.md` no longer calls
  the Rust tier gated off, `scripts/checks/versions.sh` and four `.claude`
  rules no longer describe a design phase that closed, and a reference to
  issue #20 that meant a different repository's issue now names #33.

### Changed

- `.claude/rules/ci-cd.md` and `docs/ci-cd.md` describe the pipeline that
  exists rather than the one that was waiting for a workspace, and drop a
  stale issue reference carried over from another repository.

## [0.0.1] - 2026-09-06

### Added

- The Rust CI tier, live now that the workspace exists (#15): `rustfmt`,
  `clippy` with `-D warnings`, `cargo nextest`, doctests, `rustdoc`, an MSRV
  check through `cargo-hack`, and `cargo deny`, all under the one required
  `conclusion` check, with SonarQube importing Rust coverage from an
  instrumented run.
- The Cargo workspace (#11): the eight crates of `docs/architecture.md` §11,
  the workspace lint table from `.claude/rules/reliability.md` with
  `unsafe_code` forbidden, a release profile that keeps `panic = "unwind"` and
  `overflow-checks`, and a separate `wasm-release` profile for the renderer.
  `deny.toml`'s licence exceptions now name the real crates, including the
  three pinned openEHR crates that are BUSL-1.1.
- How a coded field gets its codes, decided against measured evidence (#6):
  97.6% of the 843 coded fields in 102 operational templates take their
  membership from the template, an enumerated external code carries no rubric
  there, and one code path serves every binding kind by resolving locally and
  asking a terminology server only for what is missing. Archetype value sets
  are emitted as FHIR `CodeSystem`, `ValueSet` and `ConceptMap` under a minted
  URL, because no openEHR specification defines one.
- The overlay key, decided against measured evidence (#8): a step chain
  carrying the RM attribute, the `node_id`, the archetype id, the RM type and
  the pinned name, with a sibling ordinal only where those tie. Walking 102
  operational templates showed an id-only path collides in 14 of them, and
  that the pinned name is the only discriminator for 41% of the 427 colliding
  sibling groups, which reversed the earlier decision to keep the name out of
  the key.
- The design of record, `docs/architecture.md` (#1): the version pins, both
  template generations normalized into one internal constraint model, the
  field derivation table from the Reference Model and its constraints, the
  layout overlay with its key normalization and its replay report, the
  terminology split between local and network resolution, the ITS-REST client
  rules, the workspace layout, the build order, and a decision register.
  Every decision carries a citation or an explicit note that no specification
  governs it.
- The specification and model-crate pins in `docs/VERSIONS.md`, which now
  carry values instead of `pending #1`, and `scripts/checks/versions.sh` also
  checks `openehr-am` and `openehr-adl` (#9).
- The brand: the mark, the "Rose & Iron" palette, and the lockup, favicon and
  social-card set (#3). Rose was chosen by measuring worst-case CIEDE2000
  distance from the other three product hues under normal, deuteranope and
  protanope vision, and `assets/brand/README.md` records the ratio for every
  token against both grounds.
- The repository: the working discipline in `.claude/` (rules, hooks, skills,
  agents, memory), the community and governance documents, the pinned version
  matrix in `docs/VERSIONS.md`, the committed guards under `scripts/checks/`,
  the tracker helpers under `scripts/gh/`, and five workflows that work on a
  repository with no code (CI, CodeQL, Scorecard, SonarQube Cloud, Release).
  The configuration is FerroBRIDGE's, adapted from the FHIR and OMOP oracles to
  the openEHR Reference Model, the Archetype Object Model, ITS-REST, and AQL.

[Unreleased]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/rubentalstra/FerroCHART/releases/tag/v0.0.1
