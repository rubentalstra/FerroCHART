---
name: spec-lookup
description: Look up the authoritative openEHR Reference Model, Archetype Object Model, operational template, ITS-REST, AQL, or FHIR terminology requirement for any spec-facing behaviour, in the correct oracle order. Use before implementing or reviewing form-compiler or composition-building behaviour, or to settle a "what does the spec say" question.
allowed-tools: Read, Grep, Glob, WebFetch
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Spec lookup

Answer a "what does the specification say" question about a template, a form
field, or a COMPOSITION, and cite the source (document plus section). Never
resolve a spec-facing question from memory or from a reference implementation's
behaviour alone (`.claude/rules/spec-adherence.md`).

## Route by surface, then cite

| The question is about | The authority |
|---|---|
| what a value is, and how it serialises into a COMPOSITION | the openEHR Reference Model |
| what a constraint means: occurrences, cardinality, existence, ranges, term bindings | the Archetype Object Model and ADL |
| what an operational template carries, and how flattening works | the AM template specifications |
| a call into an openEHR CDR, its status codes, headers, or error bodies | the openEHR ITS-REST specification |
| a read-back query that fills a form from stored data | the AQL specification |
| expanding or validating an external value set | the HL7 FHIR terminology service API |
| the web template JSON or the flat composition format | no specification: the published implementations, cited as compatibility evidence |

Within a surface, the normative class definition outranks a remembered summary,
and the surrounding prose gives it meaning. Read both where both exist. Where
ADL 1.4 and ADL 2 differ, say which one the answer is for.

The existing implementations (Better Form Builder and its web template format,
Medblocks UI, the form tooling around EHRbase) and the reference deployments
(FerroEHR, FerroTERM) are behavioural evidence for spec-silent edge cases only.
Cite them explicitly as such and record the decision; never treat one as
authority.

## Layout is spec-silent, and it is most of the product

No openEHR specification governs field order, grouping, labels, help text,
conditional visibility, or anything else a form author edits by hand. Do not
go looking for an authority that does not exist. Answer that the question is
spec-silent, name the behaviour you would match and why, and flag it as a
decision to record on the tracker.

## Where to look

- **Vendored specifications (once they exist):** the research on issue #1 pins
  the versions, and the pinned texts and fixture templates are then vendored
  under `docs/specs/` or a codegen vendor directory, each with a
  `PROVENANCE.md` (`.claude/rules/vendored-inputs.md`). Grep there first,
  because it is the exact pinned text this project implements. **Nothing is
  vendored yet**, so today every lookup goes to the published sources below.
- **Published sources (fetch to confirm):**
  - openEHR Reference Model:
    <https://specifications.openehr.org/releases/RM/latest/>
  - openEHR Archetype Object Model and ADL:
    <https://specifications.openehr.org/releases/AM/latest/>
  - openEHR ITS-REST:
    <https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0/>
  - AQL: <https://specifications.openehr.org/releases/QUERY/latest/>
  - openEHR specifications index:
    <https://specifications.openehr.org/releases>
  - HL7 FHIR terminology service API:
    <https://hl7.org/fhir/terminology-service.html>

## How to answer

State the requirement, quote the decisive sentence, and cite the document plus
section (and the class and attribute where the model carries it). If the
sources are silent, say so explicitly and name the behaviour you would match;
flag it as a spec-silent decision to record on the tracker, never as a spec
fact. If the question needs a version pin the research has not made yet, say
that too: an answer that assumes a version is worse than no answer.
