---
paths: ["**/*.rs", "scripts/**", "docs/**"]
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Spec adherence (the published specifications are the oracle)

The conformance authority for this project is the published specifications, not
another implementation, not memory, not intuition. FerroCHART compiles an
openEHR operational template into a form, and turns what a person types back
into a COMPOSITION, so it answers to seven sources:

1. **The openEHR Reference Model**
   (<https://specifications.openehr.org/releases/RM/latest/>): every data type
   and every structure a COMPOSITION is built from. `DV_QUANTITY`,
   `DV_CODED_TEXT`, `DV_DATE_TIME` and the rest decide what a field is.
2. **The openEHR Archetype Object Model and ADL**
   (<https://specifications.openehr.org/releases/AM/latest/>): what a constraint
   means. Occurrences, cardinality, existence, ranges, and term bindings all
   come from here, and they are what makes a field required, repeatable, or
   bounded.
3. **The openEHR operational template**, the flattened artefact FerroCHART
   compiles, defined in the AM specifications above.
4. **openEHR ITS-REST**
   (<https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0/>):
   every call FerroCHART makes into a CDR to commit or retrieve a COMPOSITION,
   with its status codes, headers, and error bodies.
5. **AQL** (<https://specifications.openehr.org/releases/QUERY/latest/>): the
   read-back path that fills a form from data already in the CDR.
6. **The HL7 FHIR terminology service API**
   (<https://hl7.org/fhir/terminology-service.html>): the expand and
   validate-code operations, used for external value sets. FerroTERM is the
   reference server for this, never the authority for it.
7. **Simplified Formats for openEHR Data**
   (<https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0/simplified_formats.html>),
   a component document of the ITS-REST release above: the FLAT and structured
   JSON formats. It covers the two media types, the field identifiers, level
   removal, the `|other` suffix, and the mapping of each Reference Model class
   into the formats. It does not cover the web template.

The precise release of each, and which parts are vendored in-tree, are output
of the research on issue #1. Once versions are pinned, the machine-readable
artifacts are vendored under `docs/specs/` or a codegen vendor directory with a
`PROVENANCE.md` per tree (`vendored-inputs.md`), and this file gains the exact
paths. Until then, read the published sources at the URLs above and cite them.

## The web template and the Simplified Formats have different status

FerroCHART reads and writes two format families beside the canonical
serialisations, and openEHR treats them differently. Get the status right
before citing either, because it decides what wins in a disagreement.

**The Simplified Formats are specified.** openEHR ITS-REST Release-1.1.0
publishes `simplified_formats.html`, "Simplified Formats for openEHR Data", and
its §1.2 puts the document in the STABLE state. §1.4 says that with the
openEHR REST API "the available calls to be conformance tested are the same as
for other openEHR serialisation formats (canonical JSON etc), with a different
representation format indicated by setting the appropriate Content-Type HTTP
header". §2.3 defines the two media types, `application/openehr.wt.flat+json`
and `application/openehr.wt.structured+json`. The document then specifies the
field identifiers (§4.2: node-id generation, path construction, instance
indexing, attribute suffixes, and the underscore prefix for RM attributes not
in the template), level removal (§4.6), the `|other` suffix for an open value
set (§4.7), and the Reference Model mapping class by class (§5, COMPOSITION
through ELEMENT and CLUSTER). **It is the authority for the FLAT and structured
mappings.** Answer a question about either format from it, cite the section,
and never settle one by observing what EHRbase or Better does.

**The web template is a compatibility target.** No openEHR specification
defines the document. `simplified_formats.html` §2.2 lists "Web Template itself
as a resource" under what the specification does not cover, and §4.1 adds that
"Specification of Web Template metadata is separate from the data serialization
format described in this specification". ITS-REST registers the media type
`application/openehr.wt+json` and an endpoint that delivers one
(`definition.html`), and stops there. So the format is established by
observation of the published implementations, **the Reference Model is the
authority** where the two disagree, and a behaviour that exists only to match
an implementation carries a `// NOTE:` saying that, so a later reader does not
mistake it for a conformance requirement.

**Where `simplified_formats.html` and the Reference Model disagree, the
specification does not settle it.** The document positions itself as a
serialisation of Reference Model data (§2.4: serialized instances "represent
valid RM structures (e.g. COMPOSITION)"; §3.4.3 requires that "all clinical
semantics from the original archetype and template constraints are
preserved"), and it states no precedence rule for a contradiction. So the two
divide by subject: the Reference Model decides what a value is, and
`simplified_formats.html` decides how that value is spelled in FLAT or
structured. A genuine contradiction between them is an upstream defect, filed
as an `upstream-report` issue with both citations, never resolved silently in
favour of whichever reading is convenient.

## Hard rules

- **Before implementing or changing any spec-facing behaviour, read the
  governing section first.** Route by surface:
  - what a field is, and what values it admits, goes to the Reference Model
    plus the constraint in the operational template;
  - what a constraint means goes to the Archetype Object Model;
  - the shape of a committed COMPOSITION goes to the Reference Model;
  - a call into a CDR goes to openEHR ITS-REST;
  - a read-back query goes to AQL;
  - an external value set expansion goes to the FHIR terminology service API.
- **NEVER LAX. Strictness is a hard rule.** FerroCHART accepts EXACTLY what the
  governing specification admits, nothing more and nothing less.
  - Everything a specification REFUSES, we refuse, and every refusal is an
    ASSERTED NEGATIVE TEST pinning its error outcome (a value outside a
    constrained range, an occurrence violation, a code outside its value set, a
    template FerroCHART cannot compile), so a silently loosened reader is a
    failing build rather than quiet drift.
  - A spec-SILENT form is accepted only with a first-hand citation, recorded on
    a tracker issue. Stalled or contradictory upstream material is never
    carried silently.
  - Weakening any existing refusal needs a spec-grounded adjudication recorded
    on an issue, with the flipped test updated to assert the new expected
    outcome. Inventing a prohibition the specification does not contain is the
    same defect class as leniency: strict means exact, in both directions.
- **A form never admits what the template refuses.** The form definition is a
  projection of the operational template, so a field that accepts a value the
  template constrains away is a defect in the compiler, not a convenience. The
  CDR rejecting a COMPOSITION that FerroCHART built and validated is the
  loudest possible failure, and it is always our bug.
- **Cite the source.** A conformance-relevant decision names the specification
  and section in the commit or PR description. A deliberate deviation or gap
  gets a `// NOTE:` with the reference and the reason.
- **Cite ONLY durable references.** In code, doc comments, and findings,
  justify behaviour by citing one of the seven sources above, official external
  documentation (the Rust book and reference, a pinned crate's docs.rs page),
  or `docs/architecture.md`. The architecture is citable because it is the
  design of record and permanent, and because the decisions it holds are the
  ones no specification governs, so nothing else could carry them. A citation
  to it is paired with the "no specification governs this" label rather than
  standing in for a specification. **A plan document is never citable**: it is
  deleted in the pull request that implements it, so the citation rots on the
  day the work lands. The rest of the durable record is the closed issues, the
  pull request descriptions, `CHANGELOG.md` and git history. Where the
  specifications are SILENT
  (layout, the process model, storage mechanics, transport details,
  infrastructure), flag it explicitly: "no specification governs this: our own
  design". Layout is the largest such area, and it is most of the product.
- **The existing implementations are prior art, never a substitute for the
  specification.** Better Form Builder and its web template format, Medblocks
  UI, and the form tooling around EHRbase all solved parts of this. Read them
  for how a problem was solved. Where one disagrees with the specification text,
  the specification wins and the divergence is worth a note. Never resolve a
  spec question from an implementation's observed behaviour alone.
- **FerroEHR and FerroTERM are reference deployments, not oracles.** A response
  from either is evidence in a comparison, never the reference. A divergence
  found against a reference server is attributed against the specification
  before anything is changed, and if the defect is theirs it is reported to that
  project rather than worked around here.
- **A defect in a published specification is reported outbound.** File it as an
  `upstream-report` issue (`issue-workflow.md`) with what the specification
  says, what this implementation does, and the resolution sought. Do not encode
  a workaround with no record.
- Subagents doing spec-facing work must be handed the relevant sections or URLs
  in their prompt, and reviewers verify claims against them.

## Two directions, one discipline

Compiling a template into a form and building a COMPOSITION out of form data
are separate surfaces, and neither one's answer settles the other. A field that
renders correctly proves nothing about the COMPOSITION it produces. The pair is
only correct when a value survives the round trip: template to form, form to
COMPOSITION, COMPOSITION back to a filled form, with the value unchanged and
the CDR accepting it. Test the round trip, never one leg of it.

## Make no claim beyond the specification

The strongest temptation is to state a technical fact about openEHR from
memory. Do not. Every claim about the Reference Model, the Archetype Object
Model, ITS-REST, AQL, the Simplified Formats, or the web template that appears
in this repository is one the product statement in `CLAUDE.md` already makes,
or one `docs/architecture.md` has established with a citation. Anything else is a
question to answer with the specification in front of you, not a sentence in a
file.
