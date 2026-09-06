<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# CLAUDE.md

**FerroCHART** is a pure-Rust openEHR form builder and renderer. It compiles an
operational template into a form definition, renders that form for a clinician,
and turns the entered values back into a COMPOSITION committed to an openEHR
CDR over the ITS-REST API (FerroEHR is the reference CDR, never a compile-time
dependency). It reads compositions back into the same form, and uses a FHIR
terminology server (FerroTERM is the reference) to expand the value sets behind
coded fields.

The name follows the Ferro family (FerroEHR, FerroTERM, FerroBRIDGE, FerroCKM).
FerroCHART in prose, `ferrochart` in identifiers. The product is not called
FerroFORM because ferroform.eu was already registered
(`.claude/memory/domain-ferrochart-eu.md`).

## Status: the design is open, and nothing is built

There is no Cargo workspace, no crate, and no `docs/architecture.md` yet. The
architecture is the output of the research program on issue #1, and until that
closes the deliverable of most issues is cited evidence and a recommendation
rather than code. **Do not scaffold a workspace, a crate layout, or a form
definition format on your own initiative** (`.claude/memory/owner-work-style.md`).

## What the product is, and where the hard part is

The mechanical half is a compiler. Walk the operational template, and derive a
field from the Reference Model type and the constraint at each node:
`DV_QUANTITY` to a number with its permitted units, `DV_CODED_TEXT` to a
selection over its value set, `DV_DATE_TIME` to a date field at the right
precision, a `CLUSTER` with an upper occurrence above one to a repeatable
group. That part is deterministic and needs no human.

The hard half is everything a specification does not govern: field order,
grouping, labels, help text, defaults, conditional visibility. A person spends
hours on it, and then the template is revised. **The layout overlay is stored
separately, keyed by AQL path**, so a recompile from the new template replays
the overlay and reports which paths disappeared, rather than discarding the
work. Losing hand layout on a template update is the failure mode this design
exists to avoid, and it is the reason to prefer the overlay over an editor that
owns the whole document.

The other load-bearing decision is that FerroCHART is **CDR-agnostic**. It is a
client of the openEHR REST API and works against EHRbase, Better, vitagroup, or
FerroEHR. A form builder that works with only one CDR is no use to the
community that asked for this (`.claude/memory/product-scope.md`).

## Repo map

The tree is the working discipline, the documents, and the gates. There is no
code.

- `.claude/`: `rules/` (the path-scoped and standing rules), `hooks/`,
  `skills/`, `agents/`, `memory/`.
- `scripts/gh/`: the tracker helpers (`rel.sh`, `project.sh`, `labels.sh`).
- `scripts/checks/`: the committed guards (`comment-style.sh`, and
  `versions.sh`, which fails when a file disagrees with the `docs/VERSIONS.md`
  pin matrix or claims a licence other than `BUSL-1.1`).
- `.github/`: issue and pull-request templates, CODEOWNERS, Dependabot, and
  five workflows that work on a repository with no code (CI, CodeQL, Scorecard,
  SonarQube Cloud, Release). `ci.yml` runs its workflow, shell, container and
  guard tier now and keeps the Rust tier gated behind a `Cargo.toml` detection
  job; its `conclusion` job is the single required check on `main`
  (`docs/ci-cd.md`). The documentation lane lands when there is something to
  publish.
- `docs/`: `VERSIONS.md` (the pin matrix), `ci-cd.md`, `release.md`.
- Root markdown: this file, `README.md`, and the community and governance set.

This configuration was copied from FerroBRIDGE on 2026-09-06 and adapted from
the FHIR and OMOP oracles to the openEHR ones. When a rule here reads oddly,
the sibling is the reference for how it was meant to read.

## Issue workflow (the loop)

The tracker is GitHub Issues; the open issue list is the worklist
(`.claude/rules/issue-workflow.md`). One type label per issue
(bug/enhancement/documentation/chore/refactor/perf/ci), one priority label
(P0 to P3), and domain labels as needed (`spec:RM`, `spec:AM`,
`spec:ITS-REST`, `spec:AQL`, `spec:terminology`, `compat`, `ux`). Milestones
are releases. Record progress on the issue (tick criteria, comment); a PR
declares `Closes #N`. New work found while working an issue is filed and fixed
before the next unit starts (the fix-first cadence). Native sub-issue and
dependency edges are set only with `scripts/gh/rel.sh`
(`.claude/rules/issue-relationships.md`). The SessionStart hook prints the open
issue list.

## Model orchestration (workflows and subagents)

**When the session runs on Fable 5 (effort `high`), Fable is the orchestrator,
not the implementer.** Fable plans, coordinates, reviews, and does the taste-
and intelligence-heavy work itself; it fans implementation out to subagents via
the `Agent` tool (`model: 'opus'`, which resolves to the newest Opus, currently
Claude Opus 5: 1M-token context, 128K output; the `.claude/agents/*` defs with
`model: opus` pick it up automatically) or a `Workflow` (per-agent `model`). The
main loop does not auto-delegate, so this section is the standing instruction
that it should.

Why this split: the win is context isolation, parallelism, and sparing the
orchestrator's capacity. Keep Fable's context clean and let Opus workers grind
through file-heavy implementation in parallel. It is an intelligence downgrade
per worker (see the table: Fable outranks Opus on both intelligence and cost),
so delegate by the nature of the work, never reflexively.

Rankings, higher = better. `cost` is relative spend, `intelligence` is how hard
a problem the model can be handed unsupervised, `taste` covers code quality,
API design, and clarity. Models are the ones the `Agent`/`Workflow` `model`
parameter accepts.

| model     | cost | intelligence | taste |
|-----------|------|--------------|-------|
| fable-5   | 2    | 9            | 9     |
| opus-5    | 4    | 8            | 8     |

Only these two models are used: Fable 5 orchestrates (and takes the rare
top-intelligence delegation), Opus 5 is the worker tier for everything else.
Sonnet and Haiku are not used in this project; never pass them to
`Agent`/`Workflow`, even for a mechanical pass.

How to apply (defaults, not limits; override when the output misses the bar.
Intelligence beats taste beats cost when the axes conflict for anything that
ships):

- **Orchestrator (Fable 5, high):** owns the issue loop, architecture and
  design decisions, spec-conformance judgement, and the hard bespoke logic (the
  field derivation rules, the overlay replay semantics, the composition
  builder, the terminology binding). Keeps these in context rather than
  delegating; they need top intelligence and taste and are the project's
  critical path.
- **Delegate to Opus 5 subagents:** bulk or parallelizable implementation on a
  clear spec (wiring handlers, DTO impls, client plumbing, test scaffolding),
  file-heavy investigation, and codebase analysis, which would otherwise burn
  the orchestrator's context. Fan out several concurrently in one message (max
  2 implementation workers, an owner cap). Opus 5's 1M-token window means one
  worker can hold a whole subsystem, so prefer one worker with the full spec up
  front over splitting a coherent task. Prompt-tune for Opus 5: it self-verifies,
  so drop "double-check your work" scaffolding from worker prompts; it also
  delegates onward readily, so tell workers NOT to spawn their own subagents.
- **Fable 5 subagents:** use when a delegated task still needs top intelligence
  or taste (a tricky algorithm, an API-shape decision) but you want it off the
  main context.
- **Reviews:** an independent read before committing a subsystem, especially
  spec and wire conformance. Spec questions and requirements extraction go to
  `spec-researcher`; bounded implementation to `implementer`. Both are defined
  in `.claude/agents/` and both are handed the governing spec sections in the
  prompt.
- Effort: keep Fable on `high`. Use `effort: 'low'` for cheap mechanical worker
  stages, higher tiers only for the hardest verify or judge stages.

Discipline is unchanged for subagents: they obey the hard rules below (never
hand-edit a generated file, no test-weakening, conventional-type branches, no
AI attribution). Delegate with a tight spec and verify the result.

## IMPORTANT hard rules

- **The specification text is the oracle.** The conformance authority is the
  openEHR Reference Model, the Archetype Object Model and ADL, the operational
  template specifications, openEHR ITS-REST, AQL, and the HL7 FHIR terminology
  service API for external value sets. Never memory, and never another
  implementation's behaviour. Read the governing section before implementing or
  reviewing any spec-facing behaviour, and cite it (spec, page or section) for
  conformance-relevant decisions. Full policy:
  `.claude/rules/spec-adherence.md`.
- **The web template and flat formats are compatibility targets, not
  specifications.** Say so every time one is named. Where one disagrees with
  the Reference Model, the Reference Model wins, and a behaviour that exists
  only to match one carries a `// NOTE:` saying that.
- **A form never admits what the template refuses.** The form definition is a
  projection of the operational template. A CDR rejecting a COMPOSITION that
  FerroCHART built and validated is always our bug.
- **Cite only durable references:** the openEHR specifications, or official
  external documentation (the Rust book and reference, the docs.rs page of a
  pinned crate). Never cite an internal markdown file as a design authority;
  internal plan documents are deleted in the PR that implements them. Where no
  specification governs a decision (layout, storage mechanics, the process
  model, infrastructure), flag it: "no specification governs this: our own
  design". Layout is the largest such area, and it is most of the product.
- **Never hand-edit a `// @generated` file.** Change the generator and
  regenerate (`.claude/rules/codegen.md`).
- **Comments follow RFC 505 and RFC 1574 with hard budgets:** line comments
  only, pending work is `// TODO(#NNNN):` naming its issue, a settled decision
  is `// NOTE:` as a citation and one sentence. No essays in code; the record
  lives on the issue or PR (`.claude/rules/comments.md`, enforced by
  `scripts/checks/comment-style.sh`).
- **Prose follows `.claude/rules/writing-style.md`:** no em dashes, no
  "not X but Y", no decorative triads, no filler buzzwords.
- **Branches use conventional types** (`feat/`, `fix/`, `chore/`, `docs/`,
  `refactor/`, `perf/`, `test/`, `ci/`, `build/`, `release/`) as
  `<type>/<kebab-case-slug>`. Never force-push `main`.
- **NEVER add AI or Claude attribution** to a commit, PR, issue, comment, or
  code comment: no `Co-Authored-By`, no "Generated with", no bot trailer or
  footer, no emoji marker, ever. This is an absolute rule with no exceptions.
  A `PreToolUse` hook blocks a commit or PR command carrying one.
- **Keep the changelog.** `CHANGELOG.md` follows Keep a Changelog 1.1.0: every
  change with user-visible effect adds an entry under `[Unreleased]` in the
  same PR. Releases are cut from the changelog.
- **Never weaken, skip, or delete a test** to make a build pass, and never edit
  a test to route around a bug it exposes (`.claude/rules/testing.md`).
- **No patient data, ever.** Fixtures are synthetic content invented for the
  test. This project sits at the point a clinician types, so a real
  reproduction is easy to build by accident.
- **Record progress on the tracker and commit before ending a session.** Issues
  and git survive `/clear` and `/compact`; the session todo list does not.
- **Build compiling, tested increments** once code exists. Do not defer
  compilation; keep every crate you touch green.

## Licence

The project's own code and text are under the **Business Source License 1.1**
(`LICENSE`, `NOTICE`): free to read, build, modify, and redistribute, free for
every non-production use and for non-commercial production use, a commercial
licence from the Licensor for any other production use (always for a hosted,
managed, or embedded service and for for-fee distribution), and Apache License
2.0 four years after each version. Every first-party file carries
`SPDX-FileCopyrightText: Ruben Talstra` and `SPDX-License-Identifier: BUSL-1.1`
in its header. Contribution is inbound equals outbound under the same licence,
and there is no contributor licence agreement and no copyright assignment.
Vendored specifications and third-party material keep their upstream terms,
recorded in a `PROVENANCE.md` beside each vendored tree
(`.claude/rules/vendored-inputs.md`).

The decision and its history are recorded in
`.claude/memory/license-busl.md`. The one file that names Apache 2.0 as a
licence of its own is `LICENSE`, where it is the Change License.

## Working discipline (`.claude/`)

Path-scoped rules load on demand when files in their scope are read; the rest
apply always. Read the relevant one before working in that area.

- `.claude/rules/writing-style.md`: no AI tells in any prose. The top priority
  for docs and comments.
- `.claude/rules/rust-style.md`, `reliability.md`, `comments.md`, `testing.md`:
  the Rust engineering discipline (idiomatic style, safety posture, comment
  budgets, test discipline).
- `.claude/rules/spec-adherence.md`: the openEHR specifications as the oracles;
  strictness; cite the spec.
- `.claude/rules/codegen.md`: the generated-versus-hand-written rule, pending
  the research that fixes the boundary.
- `.claude/rules/vendored-inputs.md`: every external corpus is fetched by a
  committed `scripts/vendor/*.sh`, vendored verbatim, provenance-stamped.
- `.claude/rules/ci-cd.md`, `ai-code-review.md`: the workflow-security
  discipline and the advisory-analyzer policy (SonarQube Cloud, CodeQL).
- `.claude/rules/issue-workflow.md`, `issue-relationships.md`,
  `project-board.md`: the tracker work style.
- Skills: `/spec-lookup` (find the authoritative answer in oracle order),
  `/next-task`, `/phase-done`, `/phase-status` (the issue loop).
- Agents: `spec-researcher`, `implementer` (both on Opus 5).

## Sibling projects

Five sibling repositories are checked out beside this one, and none of them is
ever edited from here (`.claude/memory/sibling-projects.md`). `FerroEHR`
(`../ferroehr`) is the reference CDR, `FerroTERM` (`../FerroTERM`) the
reference terminology server, `FerroCKM` (`../FerroCKM`) governs the templates
this project compiles, `FerroBRIDGE` (`../FerroBRIDGE`) is where this
configuration came from, and `FerroHEALTH` (`../FerroHEALTH`) carries the
family site and brand.

## References

- The openEHR Reference Model:
  <https://specifications.openehr.org/releases/RM/latest/>
- The openEHR Archetype Object Model and ADL:
  <https://specifications.openehr.org/releases/AM/latest/>
- The openEHR ITS-REST specification:
  <https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0/>
- AQL: <https://specifications.openehr.org/releases/QUERY/latest/>
- The HL7 FHIR terminology service API:
  <https://hl7.org/fhir/terminology-service.html>
- The tracker: `gh issue list --state open`. Issue #1 carries the research
  program that produces `docs/architecture.md`.
