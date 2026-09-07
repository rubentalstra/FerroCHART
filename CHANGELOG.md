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

- A field carries the reference bands its template states beside the value
  (#52). openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 gives every
  `DV_ORDERED` a `normal_status`, a `normal_range` and
  `other_reference_ranges`, none of them entered by a clinician, and the
  derivation used to drop all three. They now sit in
  `field::ReferenceRanges`, reachable only from a field or a choice
  alternative and never from a field's kind or a group's items, so a renderer
  shows them without reading a Reference Model class name to tell metadata
  from an entry field. A band reuses the value shape of the class it is stated
  over: a status carries the value set of a coded field, and a range carries
  the interval shape of the class the value collects. The three attributes are
  no longer refused as unentered, and a band the derivation cannot represent
  is refused by name rather than dropped.
- What "equivalent" means for the two ADL readers is defined in one place,
  cited, with the six differences accepted as legitimate and a count for each
  allowance it spends, and the matched pair is compared as a whole value so a
  field the internal constraint model grows cannot be left out of the
  comparison (#59). The published dual-dialect archetype library cannot supply
  the ADL 1.4 half of that comparison: the twins are ADL 1.4 text, the ADL 1.4
  reader takes an ITS-XML operational template, nothing in the pinned crates
  turns one into the other, and every one of the 321 ADL 2 halves declares
  itself `generated` from its 1.4 twin. What the library does prove is
  asserted over all 321 pairs: for the 300 whose ADL 2 half the reader reads,
  the internal model names the same concept, in the same language, over the
  same Reference Model class as the published ADL 1.4 twin.
- `docs/architecture.md` section 6.5 records the five things the replay's
  outcome classes left underdetermined, decided while implementing #19 and
  #20. The load-bearing one is that a `retyped` outcome is unreachable from
  the key alone, because a field's key step says `ELEMENT` while the field
  collects a `DV_*` class. Section 11 states where the line between
  `ferrochart-form` and `ferrochart-overlay` falls and why (#72).
- `docs/architecture.md` section 6.5 records why one document state is a
  refusal at authoring time and an advisory afterwards, and section 11 stops
  reading as though `ferrochart-form` were dependency-free. Two doc comments
  that had gone stale with the type move are corrected, and the hint map's
  ordering is asserted beside the type that has to keep it rather than in
  another crate's test suite.
- The book covers what the overlay carries, why its geometry is a column grid,
  and what a replay reports, and says which releases are published and what
  the compiler measurably does.
- The citation rule says that `docs/architecture.md` is citable, because it is
  the design of record and permanent, while a plan document is not. The rule
  banned "an internal markdown file" while justifying the ban with plan
  documents, which left every architecture citation in the overlay crate
  arguably against the rules. Also fixes a `codegen.md` reference to a
  `CLAUDE.md` heading that does not exist (#77) and a prose deferral where a
  `TODO` belongs (#78).
- The replay runs against the largest real difference the corpus can produce
  (#21): an overlay authored on every node of one CKM entry, replayed against
  a second entry for the same clinical concept. Nothing survives, and that is
  the point: the two differ in the pinned name of nearly every node, so 0 of
  171 entries match and the replay reports 7 moved, 164 disappeared and 181
  appeared rather than rebinding layout onto nodes that merely look similar.
  Every entry is still accounted for and every key still held.
- The layout overlay and its store (#19), in `ferrochart-overlay`. An overlay
  carries what no specification governs: field order, authored sections,
  labels, help text, defaults, conditional visibility and widget choice. It is
  stored apart from the compiled definition and keyed by the node key of
  `docs/architecture.md` section 6.2, whose every step carries the Reference
  Model attribute, the node id, the archetype id, the Reference Model type and
  the pinned name, with a sibling ordinal only where all five tie. The store
  normalizes a key against the definition rather than trusting it: an entry
  that decorates no node of the form is refused, and an entry whose key names
  several nodes is reported as positionally keyed instead of being resolved
  silently. It records which form the template identifier takes, because
  openEHR ITS-REST Release-1.1.0 `definition.html` resolves a partial
  identifier to the latest major version, and none of the 123 committed CKM
  templates pins a release in its own identifier. The serialisation is
  byte-deterministic and round-trips.
- The replay and the differential report (#20). A replay against a recompiled
  definition classifies every entry as matched, disappeared, moved, ambiguous,
  reordered or retyped, and names every node of the form that carries no
  layout yet. Nothing is discarded and nothing is rebound: an unmatched entry
  is retained against its old key so a later revision that restores the node
  restores its layout, and a move is a suggestion a person accepts. The report
  prints as text a form author can read, saying which entries survived, which
  need a decision and what changed about each. Authoring a layout on every one
  of the 4,447 nodes of the committed pack and replaying it comes back fully
  matched, and the 102 nodes in 4 templates that no key can tell apart are
  reported rather than guessed at.
- The overlay's geometry (#69), a column grid that stores no coordinate. A
  section declares a column count of 1 to 12, and an item carries a span, a
  break-before flag and a width hint in character units. Nothing stores a row
  or a column index: an item's place is the sibling order the overlay already
  carries plus its span and break, so the visual order of a form equals its
  source order by construction, which is what W3C WCAG 2.2 success criterion
  1.3.2 asks for. A span wider than its section is refused naming both
  numbers, a section wider than four columns is stored and advised against,
  and a renderer clamps a span to a narrower grid rather than the overlay
  dropping it, so one authored form serves a ward-round tablet and a desk. A
  per-item hint map carries what the grid cannot express, uninterpreted and
  outside the clean-replay guarantee. A replay reports what became of the
  geometry beside what became of the key: a span that no longer fits the
  section it lands in, and an entry whose section was dropped. The overlay
  format version is 2, and version 1 is refused rather than read as a form
  whose author chose one column.

### Changed

- The overlay's layout types moved from `ferrochart-overlay` to
  `ferrochart-form`, where `docs/architecture.md` section 11 puts them (#72).
  `Layout`, `Section`, `Visibility`, `Condition`, `WidgetName` and the column
  grid now live beside the definition they decorate, because a renderer applies
  a layout and has no business with the store, the authoring session or the
  replay, which read and write files and compare two definitions. The stored
  document is unchanged, byte for byte. `ferrochart-overlay` keeps the store,
  the key normalization, the replay and the report, and re-exports nothing it
  does not own, so a caller that needs a layout type names `ferrochart-form`.
  `scripts/checks/crate-closure.sh` is the new CI lane that holds the boundary:
  it walks the resolved dependency closure and fails when `ferrochart-form`
  links any other crate of this tree, or when `ferrochart-renderer` links
  anything but `ferrochart-form`.

## [0.0.3] - 2026-09-07

### Added

- Snapshots of the derived form over the whole vendored corpus (#18). An
  inventory carries one line per template with its group and field counts and
  the kinds it derives, so a change shows as one changed line naming the
  template rather than as a moved total; four whole documents are snapshotted
  beside it, from a 4-field form to a 303-field one. A test also derives twenty
  templates twice and compares the bytes, so the format's determinism claim is
  checked rather than trusted.
- The ADL 2 reader runs over the published archetype library (#49): 322
  archetypes parse, 306 flatten into an operational template, and 301 read.
  The 5 the reader refuses are mandatory unfilled slots, which is the
  adjudication on #13; the 16 it never sees are refused by the upstream
  flattener because a differential path in a specialised child does not
  resolve in its parent, in a 2013 export that predates the AOM 2 validity
  rules. The two families are counted apart, because they belong to different
  owners. CI fetches the library and sets `FERROCHART_REQUIRE_ADL2`, so a
  fetch that stops working is a red build rather than a skipped suite.
- A fetch script for the ADL 2 archetype library (#49), pinned to a commit of
  `openEHR/adl-archetypes`. It brings 322 ADL 2 archetypes and 330 ADL 1.4
  twins, 321 of them the same archetype in both dialects, which is the first
  real input for the property that both readers fill one internal constraint
  model. Nothing from it is committed: one of the 652 files states a licence
  and the rest state none, so the tree is fetched into a directory
  `.gitignore` refuses and `PROVENANCE.md` records the omission.
- The published form definition type (#17), in `ferrochart-form`. A definition
  is a tree of groups rooted at the template root; a group holds items, an item
  is a nested group or a field, and a field carries one of eighteen field
  kinds, one per thing a clinician can enter. Labels and help text are resolved
  per language from the archetype terminology, occurrences carry repeatability
  and optionality, an `ELEMENT` the template permits to be omitted carries the
  four null flavours openEHR RM Release-1.1.0 `data_structures.html`
  section 4.1 names, and a `default_value` prefills the field. The crate
  performs no I/O and depends on nothing else in the tree, so a third party can
  write a renderer against it. Every map in the format is ordered and every
  list keeps template order, so recompiling an unchanged template produces an
  identical document.
- The field derivation (#16), in `ferrochart-compile`: `docs/architecture.md`
  section 5 implemented against the internal constraint model, so it is written
  once for both ADL generations. Each permitted unit of a quantity carries its
  own magnitude bounds and decimal precision, so changing the unit changes
  both; a coded field's value set is enumerated with its rubrics where the
  archetype lists them and marked for expansion at render time where it only
  names them; an ordinal keeps list order, scores by its integer and stores its
  symbol's code; a date, time, date-and-time or duration pattern is honoured
  component by component; an occurrences upper bound above one makes an item
  repeatable; an `assumed_value` never reaches the data; and an open slot is
  recorded as undetermined content rather than rendered. A constraint the
  derivation does not understand is a typed error naming the node, never a
  permissive field. Each of the twelve cases `docs/architecture.md`
  section 5.2 lists as ungoverned carries a decision in code labelled as
  FerroCHART's own design. All 121 committed CKM templates the reader reads
  derive a form.

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

[Unreleased]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/rubentalstra/FerroCHART/releases/tag/v0.0.1
