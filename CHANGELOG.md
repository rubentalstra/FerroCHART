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

FerroCHART is in its design phase, and the architecture is the output of the
research program on
[issue #1](https://github.com/rubentalstra/FerroCHART/issues/1). Releases on
the 0.0.x line carry the repository, its gates, and its documentation; there is
no binary to download yet.

## [Unreleased]

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

[Unreleased]: https://github.com/rubentalstra/FerroCHART/commits/main/
