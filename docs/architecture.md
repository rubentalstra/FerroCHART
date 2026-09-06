<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Architecture

The design of record. It answers the research program on
[issue #1](https://github.com/rubentalstra/FerroCHART/issues/1), and every
decision below carries a citation to a primary source or the explicit label
"no specification governs this: our own design".

The research ran on 2026-09-06 in four passes: the openEHR specifications and
the pin choice, the reuse audit of FerroEHR, the reuse and pattern audit of
FerroTERM, FerroCKM and FerroBRIDGE, and the prior-art survey. The findings and
their citations are recorded on issue #1.

Read `.claude/rules/spec-adherence.md` before changing anything here. The
conformance authority is the specification text, and this document is
subordinate to it.

## 1. What FerroCHART is, and where the work actually is

FerroCHART compiles an openEHR operational template into a form definition,
serves that definition to a renderer, turns what a clinician entered into a
COMPOSITION, commits it to any openEHR CDR over ITS-REST, and reads a
COMPOSITION back into the same form.

The product has two halves with different characters.

**The mechanical half is a compiler.** Walk the operational template, and
derive a field from the Reference Model type and the constraint at each node.
It is deterministic, it is fully governed by the specifications, and section 5
is its rule table.

**The other half is everything a specification does not govern**: field order,
grouping, labels, help text, defaults, conditional visibility, widget choice.
A person spends hours on it, and then the template is revised. Section 6 is
the design that keeps that work.

The research changed the size of the first half. `openehr-its` already carries
20,802 lines that compile an operational template to a web template, build a
COMPOSITION, flatten one back, and validate a COMPOSITION against a template
per node. FerroCHART consumes that rather than rewriting it, and spends its
effort on the half nobody has built.

## 2. Pinned versions

A "Release" in openEHR is a per-component version, not a bundle across
components, and a component release carries documents at different maturity
levels inside one number. Every citation therefore names the component
release, the document, and the section.

| Item | Pin | Ground |
|---|---|---|
| openEHR RM | Release-1.1.0 | The current release, 2020-09-29. Adds `DV_SCALE` and `ELEMENT.null_reason`. |
| openEHR AM | Release-2.3.0 | The current release, 2024-03-20. Carries ADL 1.4, AOM 1.4, ADL 2, AOM 2 as STABLE and OPT 2 as DEVELOPMENT. |
| openEHR ITS-REST | Release-1.1.0 | Released 2026-07-19. The first release to publish Simplified Formats as STABLE and to add `return=identifier`, weak ETags and item tags. Release-1.0.3 lacks all four. |
| openEHR AQL | Release-1.1.0 | The current QUERY release, 2021-05-14. |
| openEHR BASE | Release-1.2.0 | Carries the Architecture Overview, whose section 11 defines the path syntax AQL section 3.3 defers to. |
| openEHR ITS-XML | 2.0.0 | The `Template.xsd`, `OpenehrProfile.xsd`, `Archetype.xsd` and `Resource.xsd` family, the only normative artefact for the ADL 1.4 operational template. |
| openEHR TERM | Release-3.0.0 | The published prose. The BMM tree and the openEHR FHIR implementation guide already carry 3.1.0 content, so a TERM claim is checked against both. |
| HL7 FHIR | R4 4.0.1 | The terminology wire target. AQL Release-1.1.0 section 3.9.5.1 names `hl7.org/fhir/4.0` as a `service_api` value and never names R5. R5 is a reading oracle where R4 is silent. |
| openehr-base | 0.0.61 | Apache-2.0. |
| openehr-rm | 0.0.61 | Apache-2.0. BMM-generated, carries a `v1_1` and a `v1_2` generation; FerroCHART selects the `v1_1` generation to match the RM pin. |
| openehr-am | 0.0.61 | Apache-2.0. |
| openehr-adl | 0.0.61 | BUSL-1.1. The ADL 2, cADL and ODIN parser and the AOM 2 validation catalogue. |
| openehr-its | 0.0.61 | BUSL-1.1 and Apache-2.0. Carries `opt14`, `flat` and the ITS-REST types. |
| openehr-query | 0.0.61 | BUSL-1.1. Taken only when FerroCHART composes AQL queries rather than parsing paths. |

Three things this table has to say out loud.

**The model crates are published artefacts, not the CDR.** `CLAUDE.md` bars
FerroEHR as a compile-time dependency. The crates above are published on
crates.io and model the specification; the CDR is `app/ferroehr*` with
`publish = false` and is never a dependency of anything here. Depending on
`openehr-rm` does not couple FerroCHART to a CDR, and FerroCHART stays
CDR-agnostic because these crates model the specification rather than any
server's behaviour. The real risk is operational: a `0.0.x` line where each
patch is its own compatibility set, released in lockstep across the family.
The pin table is the control for that.

**The crate line and the specification line can disagree, and the disagreement
is recorded rather than smoothed.** `openehr-rm` carries a `v1_2` generation
ahead of the published RM release, and the ITS-BMM tree carries AM 2.4.0,
BASE 1.3.0 and TERM 3.1.0 schemas ahead of their published component releases.
FerroCHART pins the published release and selects the matching generation.

**The AM release and the ADL document version are separate numbers.** FerroCKM
records AM 2.4.0 and `openehr-adl` describes itself as ADL 2.4.0, while the
current AM component release is 2.3.0. Documents inside an AM release carry
their own versions, so these statements are compatible. Never conflate them in
a citation.

`docs/VERSIONS.md` repeats these rows and `scripts/checks/versions.sh` fails on
any disagreement.

## 3. Reading a template: two generations, one internal model

FerroCHART reads both operational template generations from the start.

**ADL 1.4.** `openehr_its::opt14::from_xml` parses the `.opt` XML. The oracle
for that layer is the XSD family in ITS-XML `components/AM/Release-1.4/`, not
AOM 1.4 prose. AOM 1.4 Appendix A, the AM 1.4 BMM, and `OpenehrProfile.xsd`
name three incompatible class sets for the same layer, and the XSD is what a
real `.opt` validates against and what a CDR accepts on
`POST /v1/definition/template/adl1.4`. Every divergence between the XSD and
AOM 1.4 prose that FerroCHART depends on carries a `// NOTE:` naming both
sources.

**ADL 2.** `openehr-adl` parses ADL 2 source and flattens it with
`flatten::flat_form` and `opt::create_opt`. FerroCHART does not read a
published OPT 2 file, because there is no published serialisation to read:
OPT 2 (AM Release-2.3.0, `OPT2.html`) has been DEVELOPMENT since 2015-10-28,
was last updated 2022-11-12, and its section 5.2 "Concrete Formats" with its
ADL, JSON/YAML and XML subsections are empty headings. The ADL 2 path
therefore starts from source archetypes and templates and produces the
operational form itself.

**The two generations do not share a constraint model**, and this is the cost
of reading both. ADL 1.4 constrains a quantity with the `C_DV_QUANTITY` domain
type; AOM 2 constrains the same clinical fact with a `C_COMPLEX_OBJECT` over
`magnitude` and `units` tied by a `C_PRIMITIVE_TUPLE` (AOM2 sections 4.3.1 and
4.5.25). Ordinals, coded text, value sets, date and time patterns and node
identity all differ the same way (ADL 1.4 uses `at`-codes for both nodes and
values; ADL 2 splits `id`, `at` and `ac` codes, ADL 2 section 7.13.5.1).

**The decision that makes reading both affordable: both readers normalize into
one internal constraint model, and the field derivation table is written once
against that model.** No specification governs this: our own design. Writing
the derivation twice, once per AOM generation, doubles the surface where a
form can silently admit what a template refuses, which is the failure this
project is least allowed to have. The internal model is the narrow waist: each
reader owes it a node identity, an RM type, an occurrences interval, a
constraint payload, and the terminology bindings in scope. Everything above
that point is generation-blind.

**An operational template with an open slot is refused, not guessed.**
`ARCHETYPE_SLOT` (AOM 1.4 section 4.3.12) in an operational template is either
closed and removed or filled and substituted inline (OPT2 sections 2.2 and
3.3). A slot still open means the content at that point is undetermined, so
the compiler reports it rather than rendering a field. `use_node` internal
references (`ARCHETYPE_INTERNAL_REF`, AOM 1.4 section 4.3.13) are expanded
inline by a conformant flattener, and FerroCHART resolves any it still meets
against the same template before deriving fields, because the 1.4 flattener is
a vendor tool and does not always expand them.

## 4. The form definition

The form definition is what the compiler emits and what the renderer reads. It
is a projection of the operational template, and a form never admits what the
template refuses.

**FerroCHART compiles to the web template shape and owns its own type over
it.** `openehr_its::flat::webtemplate::builder::build_web_template` produces
the web template, carrying `aqlPath`, `inputs`, `min` and `max`, localized
labels and term bindings per node. FerroCHART projects that into its own form
definition type rather than publishing the web template as its public format.

The reason is that the web template is a compatibility target and not a
specification, and this has to be said every time the format is named. There
is no normative web template document. The implementation FerroCHART consumes
records the same thing in its sources, carries eight additive members the
normative schemas do not have, and holds one node shape spec-legal that the
reference implementation rejects, verified empirically. ITS-REST
Release-1.1.0 publishes Simplified Formats as STABLE, registers a media type,
and exposes two endpoints that depend on the format, which makes the absence
of a normative document an upstream defect rather than a detail (section 16).

Owning the type buys three things: a public format this project can specify
and version, freedom to carry what a form needs and a wire format does not,
and a single place where a web template divergence is absorbed. It costs a
mapping layer, which is the price paid deliberately.

**Unknown members are preserved verbatim.** The EHRbase SDK preserves
arbitrary template annotations through a catch-all map, and FerroCHART does
the same: any member of a consumed web template that FerroCHART does not model
is carried through a round trip unchanged. A third party's metadata is not
destroyed by passing through this tool.

**Two web template facts that bite.** `min` and `max` on a node are the
flattened integers rather than the constraint expressions, and `semVer` is
always null for an OPT 1.4 template, so template version detection never
relies on it.

## 5. Field derivation

The rule from an RM type plus its constraint to a field. Sections are in RM
Release-1.1.0 `data_types.html` and `data_structures.html`, AOM 1.4 and AOM 2
in AM Release-2.3.0. The ADL 1.4 constrainer column names the class that
appears in a real `.opt`, which is an XSD class where AOM 1.4 has no prose for
it.

| RM class | RM section | Field collects | ADL 1.4 constrainer | ADL 2 constrainer | What the constraint decides |
|---|---|---|---|---|---|
| `DV_BOOLEAN` | 4.2.2 | `value` | `C_BOOLEAN` under a `C_COMPLEX_OBJECT` | `C_BOOLEAN` (4.5.11) | `true_valid` and `false_valid`. Both true is a two-way control. One true is a fixed value, so the field is hidden. |
| `DV_STATE` | 4.2.3 | `value`, `is_terminal` | `C_DV_STATE` with a `STATE_MACHINE` | none, a `C_COMPLEX_OBJECT` cascade | The permitted states and the transitions out of the current one. Only states reachable from the current state are offered. No AOM prose defines `C_DV_STATE`; it exists only in `OpenehrProfile.xsd`. |
| `DV_IDENTIFIER` | 4.2.4 | `issuer`, `assigner`, `id`, `type` | four `C_STRING` | four `C_STRING` (4.5.12) | Per-field `pattern` as a mask or `list` as an enumeration. |
| `DV_TEXT` | 5.2.1 | `value` | `C_STRING` on `value` | `C_STRING` | `pattern` gives a mask. `list` with `list_open` false gives a closed pick list of plain strings. Free text otherwise. |
| `DV_CODED_TEXT` | 5.2.4 | `defining_code`, and the inherited `value` rubric | `C_CODE_PHRASE`, or `C_CODE_REFERENCE` with `referenceSetUri`, or `CONSTRAINT_REF` to an `ac`-code | `C_TERMINOLOGY_CODE` (4.5.22) | The value set. A non-empty `code_list` is a closed selection with rubrics from the template terminology. A bare `terminology_id` is any code from that terminology. A `referenceSetUri` or `ac`-code is a network expansion. Section 7. |
| `CODE_PHRASE` | 5.2.3 | `terminology_id`, `code_string` | `C_CODE_PHRASE` | `C_TERMINOLOGY_CODE` | The pair every selection emits. `preferred_term` is display only. |
| `DV_ORDINAL` | 6.2.4 | `symbol`, `value` | `C_DV_ORDINAL` with `list` | `C_COMPLEX_OBJECT` with a `C_ATTRIBUTE_TUPLE` over `symbol` and `value` (4.5.26) | The ordered symbol list. Each entry is one option: the integer is the score, the symbol's `defining_code` is stored, the rubric is the label, and list order is display order. |
| `DV_SCALE` | 6.2.5 | `symbol`, `value` | no constrainer class exists | `C_COMPLEX_OBJECT` with a `C_ATTRIBUTE_TUPLE` | As `DV_ORDINAL` with a real score. `OpenehrProfile.xsd` has no `C_DV_SCALE`, so an ADL 1.4 operational template cannot express a `DV_SCALE` domain constraint at all. Section 16. |
| `DV_COUNT` | 6.2.9 | `magnitude` | `C_INTEGER` | `C_INTEGER` (4.5.14) | `range` gives min and max, `list` an enumeration, `assumed_value` the value used when an optional node is omitted. |
| `DV_QUANTITY` | 6.2.8 | `magnitude`, `units`, `precision` | `C_DV_QUANTITY` with `property` and a `C_QUANTITY_ITEM` list | `C_COMPLEX_OBJECT` over `magnitude` and `units` tied by a `C_PRIMITIVE_TUPLE` (4.3.1, 4.5.25) | The permitted units are the `units` of each `C_QUANTITY_ITEM`, and each unit carries its own magnitude interval and decimal precision, so changing the unit changes the permitted range and the decimals. `property` names the UCUM property for a units lookup. |
| `DV_PROPORTION` | 6.2.10 | `numerator`, `denominator`, `type`, `precision` | `C_REAL` on numerator and denominator, `C_INTEGER` on `type` | the same shape (4.2.11) | `type` is a `PROPORTION_KIND` (6.2.11): percent renders one number with denominator 100, unitary one number with denominator 1, ratio and fraction two numbers. |
| `DV_INTERVAL<T>` | 6.2.2 | `lower`, `upper`, the inclusivity and unbounded flags | `C_COMPLEX_OBJECT` with the T constrainer under `lower` and `upper` | the same shape | Two fields of the T widget plus inclusivity. No dedicated constrainer exists in either generation. |
| `DV_DATE` | 7.2.2 | `value` | `C_DATE` with `pattern`, `range`, `timezone_validity`, `assumed_value` | `C_DATE` with `pattern_constraint` (4.5.18) | The pattern says which components the field must, may and must not collect. `range` gives min and max. |
| `DV_TIME` | 7.2.3 | `value` | `C_TIME` | `C_TIME` (4.5.19) | The same, across hour, minute, second, fraction and timezone. |
| `DV_DATE_TIME` | 7.2.4 | `value` | `C_DATE_TIME` | `C_DATE_TIME` (4.5.20) | The same, across all seven components. |
| `DV_DURATION` | 7.2.5 | `value` | `C_DURATION` with `pattern` and `range` | `C_DURATION` with `pattern_constraint` (4.5.21) | The pattern letters say which ISO 8601 slots may be filled, with "or" semantics (ADL 1.4 section 5.4.6.2). Negative durations are legal (7.1.2.1). |
| `DV_MULTIMEDIA` | 9.2.2 | `media_type`, and either `uri` or `data` | `C_CODE_PHRASE` on `media_type` | `C_TERMINOLOGY_CODE` | The `media_type` code list is the accepted set of media types, which becomes the file input's filter. |
| `DV_PARSABLE` | 9.2.3 | `value`, `formalism` | two `C_STRING` | the same | A constrained `formalism` list gives a selector; `value` is a text area. |
| `DV_URI` | 10.3.1 | `value` | `C_STRING` on `value` | `C_STRING` | `pattern` restricts the scheme. |
| `DV_EHR_URI` | 10.3.2 | `value`, `ehr://` scheme only (10.4.1) | as `DV_URI` | as `DV_URI` | Rarely clinician-entered. |
| `DV_ORDERED` | 6.2.1 | `normal_status`, `normal_range`, `other_reference_ranges` | inherited attributes under a `C_COMPLEX_OBJECT` | the same | Reference ranges are display metadata for the clinician rather than entry fields. |
| `TERM_MAPPING` | 5.2.2 | `match`, `purpose`, `target` | `C_COMPLEX_OBJECT` | the same | Never clinician-entered. The composition builder writes it when a coded value carries a mapping. |

### 5.1 The structural rules

**`ELEMENT`** (data_structures 5.2.3) carries the data value, and its
invariants are the whole null mechanism: `is_null()` holds exactly when
`value` is absent, a null flavour is present exactly when the value is null,
the null flavour comes from the openEHR terminology null-flavour group, and
`null_reason` is only legal when the value is null. Read as a form rule, **a
field has either a value or a null flavour, never both and never neither.**
FerroCHART renders a null-flavour affordance beside every `ELEMENT` whose
occurrences permit omission, and `null_reason` as free text enabled only once
a null flavour is set. data_structures 4.1 names the codes `253|unknown|`,
`271|no information|`, `272|masked|` and `273|not applicable|`.

**`CLUSTER`** (5.2.2) is a form group, an ordered list of `ITEM`.

**The `ITEM_STRUCTURE` subtypes** (4.3) get their form shape from the ISO
13606 encoding rules in 4.2: `ITEM_SINGLE` is one field, `ITEM_LIST` is a flat
list of fields, `ITEM_TABLE` is a grid where each row is a `CLUSTER`, the
element names are the column names and the row cluster's name is the row
number, and `ITEM_TREE` is nested groups. In a table, an empty cell is an
`ELEMENT` with a null flavour rather than an absent one.

**Existence, occurrences and cardinality are three different things**, and
AOM 1.4 section 4.2.2 and AOM 2 section 4.2.2 say so in near-identical prose.
Existence (`C_ATTRIBUTE.existence`) says whether the attribute slot is there
at all. Cardinality (`C_MULTIPLE_ATTRIBUTE.cardinality`, carrying
`is_ordered`, `is_unique` and an interval) says how many members a container
holds and whether it behaves as a list, set or bag. Occurrences
(`C_OBJECT.occurrences`, AOM 1.4 section 4.3.6) says how many times a
particular child node may repeat.

**An occurrences upper bound above one is what makes a group repeatable.**
`{0..*}` on a `CLUSTER` is an add-another group, `{0..1}` on an `ELEMENT` is
one optional field, `{1..1}` is one mandatory field, and `{0}` means the
template removed the node so it is never rendered. `is_ordered` true on a
container means display order is significant.

**Defaults and assumed values are different, and only one reaches the data.**
AOM 1.4 section 4.2.3.2: "The notion of assumed values is distinct from that
of 'default values'. The latter is a local requirement, and as such is stated
in templates; default values do appear in data, while assumed values don't."
A form prefills from `T_COMPLEX_OBJECT.default_value` in the template's
`constraints` section. It never writes an `assumed_value` into data.

### 5.2 What the specifications leave open

Each item is something the compiler or the renderer must decide, and no
openEHR specification governs it. These are the reason section 6 exists.

1. **Field order.** Ordering is meaningful only for container attributes with
   `cardinality.is_ordered` true. The order of attributes under a complex
   object and of alternatives under a single-valued attribute carries no
   display semantics anywhere in AM or RM.
2. **Grouping beyond the RM structure.** `SECTION`, `CLUSTER` and the item
   structures give a tree. Nothing says how a tree becomes pages, tabs,
   columns or fieldsets.
3. **Labels, partly.** `ARCHETYPE_TERM.text` and `.description` give a rubric
   and a description per code per language (AOM 1.4 section 7.3.2, AOM 2
   section 7.3.4). Which becomes the label, which becomes help text, and what
   happens when neither fits, is not governed.
4. **Help text.** Nothing defines the distinction from a label.
   `ARCHETYPE_TERM.other_items` is a free hash and defines no keys for form
   use.
5. **Conditional visibility.** ADL 1.4 section 8.5 and the AOM 2 Rules package
   give predicate rules evaluated after entry. Nothing maps a rule to hiding a
   field until another is answered.
6. **Widget choice.** A four-member coded value set could be radio buttons or
   a dropdown. Nothing in RM or AM says.
7. **A node that accepts either a code or free text.** AOM 2 section 4.2.8.2
   says the pattern needs two sibling nodes, one coded and one text-only. How a
   form presents one field that accepts either is unspecified. ITS-REST
   Release-1.1.0 `simplified_formats.html` section 4.7 invents a `|other`
   suffix to make the branch explicit on the wire, which is evidence that the
   models leave it ambiguous.
8. **Field-level validation error presentation.** Section 9.
9. **Which unit in a quantity list is preferred.** The XSD gives an ordered
   list and says nothing about preference, and `assumed_value` is an
   assumption rather than a default.
10. **Repeating group presentation.** The numbers are governed. Whether a
    repeatable group is an inline repeater, a modal or a table is not.
11. **`C_DV_STATE` semantics.** Defined in the XSD, defined in no prose.
12. **The layout overlay itself.** Section 6.

## 6. The layout overlay

No specification governs this: our own design. The prior-art survey settles
the question. The XSDs carry three places layout could live, the `annotations`
section, the `view` and `T_VIEW` structures, and vendor flags such as
`hide_on_form`, and the specification prose defines the semantics of none of
them. AOM 2 section 3.5.1 scopes annotations to documentation, and OPT 2
permits a generator to delete the annotations section outright. openEHR has no
form artefact.

### 6.1 Why the overlay is separate

The alternative designs were read and rejected on evidence. Storing layout in
the template's annotations is what one commercial vendor does; the layout
survives a revision and pollutes the shared clinical model, so a form
decision travels to every consumer of that template. Storing layout in the
rendered artefact is what the open-source web-component libraries do; nothing
replays and nothing is reported when the template changes. Storing layout on
the definition items is what FHIR SDC does; `$assemble` regenerates and
`derivedFrom` forks, with no replay and no missing-path report. None of the
implementations surveyed reports what a recompile did to hand-authored
layout. **That report is the product.**

### 6.2 The key

The overlay is keyed by the identity of the node it decorates, and that
identity was measured rather than assumed. The measurement walked 102
operational templates, 87 single-purpose conformance templates and 16 CKM
clinical templates, node by node, 10,802 nodes in total (issue #8).

**An id-only path is not unique inside an operational template.** 427 sibling
groups share one `node_id` under one attribute, across 14 of the 102
templates. BASE Release-1.2.0 section 11.2.2.2 records that an archetype path
is unique **in an archetype**, and an operational template composes many
archetypes and may repeat a node, so that guarantee does not reach this far.
Section 11.2.4 separately records that a path does not uniquely identify items
in runtime data. The overlay keys the definition rather than the data, which
removes the second problem and leaves the first.

What separates a colliding sibling group in the corpus:

| Discriminator | Groups | Share |
|---|---|---|
| The pinned name | 175 | 41.0% |
| The RM type | 126 | 29.5% |
| The archetype id | 76 | 17.8% |
| Partly separated, some siblings still tie | 20 | 4.7% |
| Nothing | 30 | 7.0% |

**The key is therefore a chain of steps, and each step carries five things**:
the RM attribute name, the `node_id` where the node has one, the archetype id
where the child is an archetype root, the RM type, and the pinned name where
the template states one. No specification governs this: our own design.

Each part earns its place in the corpus:

- **`at`-codes, never meaning-based path segments.** ADL 1.4 section 7.2
  warns that segments such as `items[systolic]` "are only for display
  purposes, and paths used for computing always use the 'at' codes".
- **The archetype id at every chaining point.** BASE section 11.2.3
  distinguishes archetype boundaries with an archetype id predicate. It is the
  only discriminator for 17.8% of collisions, and it makes a swapped archetype
  show as a changed key rather than an invisible rebinding of layout onto a
  different clinical concept. In the CCTA report, `/content[at0000]/items[at0000]`
  holds three `OBSERVATION` children that differ in nothing else.
- **The RM type.** It is the only discriminator for 29.5%, most often the
  `DV_CODED_TEXT` and `DV_TEXT` sibling pair that AOM 2 section 4.2.8.2
  prescribes for a field taking either a code or free text. That pattern
  occurs 42 times in the corpus. Carrying the type also lets a replay report a
  node whose type changed, which is layout that may no longer mean anything.
- **The pinned name, where the template states one.** It is the only
  discriminator for 41.0%, the largest share. In the breast cancer synoptic
  report, `/content[at0000]` holds three `SECTION` children, all `at0000` from
  `openEHR-EHR-SECTION.adhoc.v1`, told apart only by "Patient Identity",
  "Diagnostic Summary" and "2. Current biopsy findings".

**A name in the key is a definition fact, and it is not the same thing as a
name predicate matched against data.** The warning in the flat path machinery,
that a template-derived `[atNNNN,'name']` conjunct can fail to match because
an instance may legitimately redefine `LOCATABLE.name` (RM common), is about
resolving a path against a COMPOSITION. That problem belongs to the
composition builder and the read-back of section 8, where the id-only fallback
applies and is sound only when the id is unique among matched siblings. The
overlay keys the definition, where a pinned name is a fact the template states
and is exactly as stable as the rest of the template. Running the two together
would discard the strongest discriminator the corpus has.

**Where the whole tuple still ties, the step carries a sibling ordinal and the
entry is marked as positionally keyed.** This is 7.0% of collisions, and the
congenital syphilis form is the shape: two `ITEM_TREE` children under one
`events[at0026]/data`, both named "Simple", identical in every other respect.
A positional key is the one a reordering silently breaks, so the replay
reports every positionally keyed entry whenever its container changed, rather
than trusting the position.

### 6.3 Stability across revisions

**No specification says an AQL path is stable across template revisions.**
AQL Release-1.1.0, BASE Release-1.2.0 sections 10.5, 10.6 and 11, AOM 1.4
section 4.2.3.1 and AOM 2 section 4.2.3.2 are all silent. What the
specifications do say bounds the answer, and all of it points one way:
specialisation provably changes codes, since AOM 2 section 7.2.1 makes
`id25.1` a specialisation of `id25`; AOM 1.4 section 4.2.3.1 lets a leaf with
no siblings carry no node id at all, so adding a sibling can make a node id
appear where there was none; and a template revision may re-run the
flattener, swap an archetype version, fill a slot differently, or delete a
node, each of which changes paths.

So recompile-and-report is not a fallback. It is the only correct design. The
key shape of 6.2 is fixed before any overlay is written to disk, because a
stored key cannot be changed later without a migration.

### 6.4 The replay report

A replay against a recompiled definition classifies every overlay entry, and
the classification is the feature:

- **matched**, the key resolves to one node of the same RM type.
- **disappeared**, the key resolves to nothing.
- **moved**, the key resolves to nothing and exactly one node elsewhere
  carries the same terminal node id and RM type. Reported as a suggestion for
  a person to accept, never applied silently.
- **ambiguous**, the key resolves to more than one node, which means the
  five-part step tuple of 6.2 tied and the entry was positionally keyed.
- **reordered**, the entry is positionally keyed and its container changed, so
  the position it relies on is no longer trustworthy. Reported for a person to
  confirm, never followed silently.
- **retyped**, the key resolves to one node whose RM type changed, so the
  layout may no longer be meaningful.
- **new**, a node in the recompiled definition that no overlay entry
  decorates.

Nothing is discarded. An unmatched entry is retained against its old key so a
later revision that restores the node restores its layout.

## 7. Terminology

A coded field needs its permitted codes, and where they come from is decided
by the binding rather than by preference. The split was measured over the same
102 operational templates as section 6.2: 843 coded-value constrainers (issue
#6).

| Binding kind | Constrainer | Count | Share | Resolves |
|---|---|---|---|---|
| A local `at`-code list | `C_CODE_PHRASE`, `terminology_id` of `local` | 595 | 70.6% | Entirely from the template |
| External codes enumerated | `C_CODE_PHRASE` with a foreign `terminology_id` and a full `code_list` | 228 | 27.0% | Membership from the template, display text from elsewhere |
| An open external terminology | `C_CODE_PHRASE`, foreign `terminology_id`, empty `code_list` | 4 | 0.5% | A server expansion |
| A template reference set | `C_CODE_REFERENCE.referenceSetUri` | 3 | 0.4% | A server expansion |
| An `ac`-code constraint group | `CONSTRAINT_REF` plus `constraint_bindings` | 9 | 1.1% | A server expansion |
| Unconstrained coded text | `C_CODE_PHRASE` with neither | 4 | 0.5% | Any code, no expansion |

**97.6% of coded fields get their membership from the template.** ADL 2
section 8.1 says the same thing in prose, that archetype-local sets outnumber
external ones by orders of magnitude, and the corpus bears it out. So the
default path is local and a network call is the exception.

**Membership and display text are separate questions**, and this is the part
the prose does not tell you. An `at`-code carries its rubric in the template's
`ontology` or `component_ontologies` as an `ARCHETYPE_TERM` with mandatory
`text` and `description` (AOM 1.4 section 7.3.2), so kind A needs nothing else.
An enumerated external code carries no rubric there. Checked directly: in the
COVID-19 infection report, all 16 external codes are absent from the template
terminology, `433` and `315642008` among them. Of the 228 enumerated external
codes in the corpus, 207 are openEHR's own terminology, which comes from the
`openehr-term` crate rather than a call. The remainder need a display lookup.

**One code path serves every kind.** The compiler resolves what the template
carries, then asks the terminology client for what is missing: a display text
for an enumerated external code, or a full expansion for a binding that names
a target. The renderer sees one kind of field either way, and a field whose
expansion has not arrived is in a known pending state rather than an empty
picker.

### 7.1 The calls

`ValueSet/$expand` populates a picker for the three server-resolved kinds.
`CodeSystem/$lookup` supplies a display text for an enumerated external code.
`ValueSet/$validate-code` confirms a code the clinician chose. FHIR R4 4.0.1
is the wire target, per the pin table.

**An upstream failure is never flattened into an empty picker.** A refused
expansion, a timeout or an unresolvable binding is a typed error carrying the
upstream status and body, surfaced on the field
(`.claude/rules/reliability.md`).

### 7.2 Serving an archetype's value sets as FHIR resources

No specification governs this: our own design. It is what makes kind A
addressable by a terminology server at all, and it is the request Severin
Kohler put to this project on 2026-09-06.

**The gap is identity, and it is real.** No openEHR specification defines a
canonical URI for an openEHR terminology, for an archetype-local value set, or
for a template-local value set. `CODE_PHRASE.terminology_id` of `local` (RM
`data_types.html` section 5.2.3) is a plain string whose scope is one
archetype. The openEHR Base FHIR implementation guide exists as an unpublished
ci-build and defines `https://specifications.openehr.org/fhir/codesystem-<name>`
for 27 CodeSystem and ValueSet pairs, and it covers the RM support terminology
only, with nothing for any archetype, `at`-code or `ac`-code. A FHIR ValueSet
requires a `url`, and openEHR supplies none.

**So FerroCHART mints one, under a domain its deployer controls**, encoding the
archetype identity, which AOM 2 section 3.2 and ADL 1.4 section 8.8.1 make
globally unique:

```
https://<deployer-domain>/fhir/CodeSystem/openEHR-EHR-OBSERVATION.blood_pressure.v2
https://<deployer-domain>/fhir/ValueSet/openEHR-EHR-OBSERVATION.blood_pressure.v2--ac1
```

`version` is the archetype version, or the `template_id` for a template-scoped
set, and it changes whenever the code list changes, because a terminology
server caches on `url` plus `version`. `identifier` carries the openEHR
identity as a `system` and `value` pair, which CodeSystem section 4.8.3.1
describes as the element for external references. A `urn:openehr:archetype:`
form is legal where the deployer has no domain, and FHIR discourages it because
it does not resolve. **A URL under `specifications.openehr.org` is never
minted**, because that namespace belongs to openEHR International and is
already in use.

**The emitted shapes.** An archetype's `at`-codes become one `CodeSystem` whose
concepts carry `code`, `display` from `ARCHETYPE_TERM.text` and `definition`
from `.description`, per language. Each `ac`-code or inline code list becomes a
`ValueSet` composing those concepts. `term_bindings` become a `ConceptMap`, one
`group` per source and target system pair, which matches openEHR's
per-terminology grouping exactly. Two qualifications the specifications force:
ADL 2 section 8.2 warns that only some local codes have external equivalents
and that a binding implies no coverage, so a `ConceptMap` built from bindings
is partial by construction and never claims completeness; and a path-keyed
binding binds a node to a pre-coordinated term rather than mapping code to
code, so it belongs in the field's metadata rather than in a `ConceptMap`.

**Binding strength maps directly, and the specification says so.** AOM 2
section 4.2.8.2 states that `required`, `extensible`, `preferred` and `example`
follow the FHIR binding-strength model. In FHIR the strength sits on the
binding site rather than on the ValueSet, so it belongs on FerroCHART's field
definition. ADL 1.4 has no equivalent, so a 1.4 binding is `required`.

**One trap that would corrupt the output.** In ADL 1.4 a single `at`-code space
serves both node identifiers and coded values (ADL 1.4 sections 8.2.3 and
8.4.1, which ADL 2 section 7.13.5.1 records as deprecated). Extracting every
`at`-code from a 1.4 template would emit a CodeSystem mixing field names with
field values. **Only an `at`-code appearing inside a `C_CODE_PHRASE.code_list`
or as a `CONSTRAINT_REF` target is a value.**

**No terminology server change is required for the common case.** The FHIR
terminology ecosystem lets a client supply a CodeSystem or ValueSet inline with
an expansion request, and FerroTERM implements that mechanism, so FerroCHART
can hand over an archetype-derived resource per call without anything being
stored. Serving these resources persistently, so any client can expand an
openEHR value set without supplying it, is a FerroTERM feature rather than a
FerroCHART one, and is filed there.

## 8. The wire: FerroCHART as an ITS-REST client

All citations are ITS-REST Release-1.1.0. The base path is `{baseUrl}/v1`.

**Templates.** `POST /v1/definition/template/adl1.4` takes the `.opt` XML as
`application/xml`, and `POST /v1/definition/template/adl2` takes ADL 2 source
as `text/plain`, so a CDR may implement one, the other, or both.
`GET /v1/definition/template/adl1.4/{template_id}` returns the canonical OPT
XML under `Accept: application/xml` and the web template under
`Accept: application/openehr.wt+json`, which means FerroCHART can take a
template from a CDR as well as from FerroCKM.

**A template id has four forms**, and one of them moves: a partial HRID such
as `openEHR-EHR-COMPOSITION.t_vital_signs.v1` "resolves to latest major
version". **An overlay records which form of the id it was keyed against**,
because a partial HRID silently follows a new template and would otherwise
replay layout onto a definition nobody chose to move to.

**Compositions.** `POST /v1/ehr/{ehr_id}/composition` creates,
`PUT /v1/ehr/{ehr_id}/composition/{uid_based_id}` updates, and
`GET /v1/ehr/{ehr_id}/composition/{uid_based_id}` reads, where the id accepts
either a versioned object uid for the latest version or a full version uid.

**`If-Match` is required on update.** The client sends the preceding version
uid in quotes without the `W/` prefix, and the specification's own example is
`If-Match: "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2"`.
A 412 is a concurrent-edit outcome the form surfaces to the clinician, never a
blind retry. Response ETags are weak from Release-1.1.0 and were unprefixed
before it, so the client strips both `W/` and the quotes before comparing.

**`Prefer` is always explicit**, because the specification warns that a server
may be configured to change the default. FerroCHART sends
`return=identifier` on a normal save, which yields the new version uid it
needs for the next `If-Match` and nothing more, and
`return=representation` only when it has to re-render from the server's
canonicalisation.

**The canonical serialisations are the wire.** FLAT, structured and the web
template are compatibility targets rather than specifications, and are named
as such wherever they appear.

**Nothing in the client absorbs an upstream failure.** A refusal, a timeout or
a partial write is a typed error carrying the upstream status and body.

## 9. Validation and the field-level error

**The specification does not give FerroCHART a field-level error, so
FerroCHART owns it.** ITS-REST Release-1.1.0 makes the error body optional
("services MAY return additional error details"), conditional on the client
having sent `Prefer: return=representation`, and shapes its `errors` array as
`DV_CODED_TEXT` entries whose codes are in `terminology_id: "local"`.
`DV_CODED_TEXT` has no path attribute, so **the wire carries no pointer to the
node that failed**, and the codes are vendor-defined and not comparable across
CDRs.

The design this forces:

1. **FerroCHART validates the composition against the operational template
   before it posts.** `openehr_its::rm_instance::validate_composition` returns
   messages carrying a path, a message and a kind, keyed by the same path
   string the compiler emitted, which is a field-level error with no adapter
   in between.
2. **A CDR rejecting a composition FerroCHART built and validated is a
   FerroCHART bug**, per the hard rule in `CLAUDE.md`. The CDR's error body is
   diagnostic material for that bug.
3. **Mapping a vendor error body onto a field is best-effort, vendor-specific,
   and carries a `// NOTE:`** saying so.

## 10. The renderer

**The compiler runs on the server, and the renderer reads the form
definition.** The owner's decision, 2026-09-06. It follows the shape the
README already promises, that validation happens on the server so a clinician
gets a field-level error, and it has a hard technical ground: `openehr-its`
has no dependency set that compiles to `wasm32-unknown-unknown` while
exposing the flat and OPT code, because every dependency rides its default
`full` feature, which pulls a web framework, a cache and a JSON schema
validator. That constraint is filed upstream as FerroEHR issue 3149. The
browser never links `openehr-its`, so the constraint does not bind
FerroCHART's design and the upstream fix is not a blocker.

The renderer is a Leptos client-side binary built with Trunk, following the
family recipe proven in FerroTERM's viewer: `gloo-net` for requests rather
than a server-shaped HTTP client, one module owning every request, typed
errors carrying the upstream status and body, a separate WASM release profile,
and the Tailwind standalone CLI. Two guards from that lane are adopted here: a
boundary check that walks the resolved dependency closure so the UI cannot
link an engine crate, and a gzipped bundle-size gate.

**A third party can write their own renderer** against the published form
definition, which is the reason section 4 owns the type rather than
publishing a web template.

## 11. Workspace layout

The crate names are reserved before code lands, so the layering is fixed by
the manifest rather than by habit.

| Crate | Role |
|---|---|
| `ferrochart-form` | The form definition type, the overlay type, and their serialisations. No I/O. |
| `ferrochart-compile` | Operational template to form definition. Owns the internal constraint model of section 3 and the derivation of section 5. |
| `ferrochart-overlay` | Overlay storage, the key normalization of section 6.2, replay, and the differential report. |
| `ferrochart-cdr` | The ITS-REST client of section 8. |
| `ferrochart-term` | The terminology client of section 7. |
| `ferrochart-server` | The HTTP surface: serves definitions, validates, builds and commits compositions. |
| `ferrochart-renderer` | The Leptos client-side binary of section 10. Depends on `ferrochart-form` and nothing else from this tree. |
| `ferrochart` | The binary. |

The root manifest carries the workspace lints of `.claude/rules/reliability.md`
at their stated tiers, `unsafe_code = "forbid"` among them, and the release
profile pins `panic = "unwind"` and `overflow-checks = true`.

## 12. What each seam carries

- **FerroCKM to FerroCHART:** an operational template, as OPT 1.4 XML or ADL 2
  source. FerroCKM exports OPT 1.4 and retrieves over
  `GET /templates/{cid}/opt` with a `get-latest-published` selector.
- **A CDR to FerroCHART:** a template, and a COMPOSITION on read-back.
- **FerroCHART to a CDR:** a COMPOSITION in a canonical serialisation, with
  `If-Match` on update.
- **A FHIR terminology server to FerroCHART:** an expansion or a code
  validation, for the bindings of section 7 that name a target.
- **FerroCHART to a renderer:** a form definition, and a validation result
  keyed by node path.
- **A person to FerroCHART:** the overlay, which is the only artefact in this
  list no specification governs.

## 13. Verification and the acceptance instrument

**The openEHR conformance component is DEVELOPMENT**, and ITS-REST
Release-1.1.0 has a Conformance section whose entire content is "tbd.", so
there is no published conformance suite to certify against. The acceptance
instrument is therefore assembled from published material:

1. **A per-datatype template grid.** FerroEHR's `corpus/templates/` carries
   about 85 single-purpose `.opt` files, one per datatype and constraint
   shape, which is a ready-made grid for the derivation table of section 5.
   These are re-vendored through a committed `scripts/vendor/*.sh` with a
   `PROVENANCE.md`, per `.claude/rules/vendored-inputs.md`, rather than copied
   between checkouts.
2. **CKM templates**, whose per-file licences are a mix of CC-BY-SA 4.0 and
   3.0 and are recorded with the vendored tree.
3. **A round trip as the acceptance test**: compile a template, fill every
   field with synthetic content, build a COMPOSITION, commit it to a CDR, read
   it back, and assert the form state matches what went in.
4. **An overlay replay test against a real template revision**, asserting the
   overlay survives and the report names what changed. This is the test that
   proves the product's claim.

Fixtures are synthetic content invented for the test. No patient data, ever.

## 14. Build order

**v0.0.2, the workspace and the readers.** The workspace skeleton with the
crate names of section 11 and the lints wired. The pinned model crates. The
vendored template corpus with provenance. Both template readers behind the
internal constraint model of section 3. The Rust CI tier switches on the
moment a `Cargo.toml` exists.

**v0.0.3, the compiler.** The derivation table of section 5 against the
internal model, the form definition type of section 4, and the projection from
the web template with unknown members preserved. Snapshot tests over the
whole vendored grid.

**v0.0.4, the overlay.** The key normalization from issue #8, the overlay
store, replay, and the differential report of section 6.4, with the replay
test against a real revision.

**v0.0.5, the wire.** The ITS-REST client, the composition builder, read-back
into a form, pre-post validation and the field-level error of section 9, and
the terminology client of section 7.

**v0.1.0, the renderer and the loop.** The Leptos renderer, the authoring
surface for the overlay, and the acceptance instrument of section 13 running
green against a CDR.

Each release is green before the next starts.

## 15. Decision register

| Decision | Choice | Ground | Rejected |
|---|---|---|---|
| Model layer | Consume the published `openehr-*` crates | BMM-generated, Apache-2.0 for the model tier, already published; the AM 1.4 BMM cannot generate a usable constraint model | A generator in this repository; hand-writing the RM |
| Template generations | Both, from the start | The owner's decision, 2026-09-06 | ADL 1.4 first, which the research recommended |
| Generation handling | One internal constraint model, both readers normalize into it | Writing the derivation twice doubles the surface where a form admits what a template refuses | A derivation per AOM generation |
| ADL 1.4 oracle | The ITS-XML XSD family | AOM 1.4 prose, the AM 1.4 BMM and `OpenehrProfile.xsd` name three incompatible class sets; the XSD is what a real `.opt` validates against | AOM 1.4 prose |
| ADL 2 input | Source archetypes flattened by `openehr-adl` | OPT 2 publishes no concrete format; its serialisation sections are empty headings | Reading a published OPT 2 file |
| Form definition | A FerroCHART type projected from the web template | The web template has no normative document and carries divergences that would become ours | Publishing the web template as the format |
| Unknown members | Preserved verbatim through a round trip | A third party's metadata must survive this tool | Dropping what is not modelled |
| Layout storage | A separate overlay keyed by computable path | No specification governs a form artefact; every surveyed alternative loses the work or pollutes the shared model | Template annotations; layout on the definition; layout in the rendered artefact |
| Overlay key | A step chain carrying attribute, `node_id`, archetype id, RM type and pinned name, with a sibling ordinal only where those tie | Measured over 102 operational templates: an id-only path collides in 14 of them, and name, RM type and archetype id are the only discriminators for 41.0%, 29.5% and 17.8% of the 427 colliding groups | An id-only key; a key omitting the name, which loses the largest discriminator; keying by position everywhere |
| Revision handling | Recompile, replay, and report per entry | No specification promises path stability across revisions, and specialisation provably changes codes | Migrating layout silently; discarding unmatched entries |
| Coded field resolution | Local first, a server call only for what the template does not carry | Measured over 102 templates: 97.6% of 843 coded fields get their membership from the template, and an enumerated external code carries no rubric there | Expanding everything over the network; treating membership and display as one question |
| Value set identity in FHIR | A minted URL under the deployer's domain, encoding the archetype id, with the archetype version as `version` | No openEHR specification defines a canonical URI for a local value set, and FHIR requires a `url`; the archetype id is globally unique | A `urn:` form, which does not resolve; a URL under `specifications.openehr.org`, which is another publisher's namespace |
| Terminology wire | FHIR R4 4.0.1 | AQL section 3.9.5.1 names `hl7.org/fhir/4.0` and never R5 | R5 |
| Field-level errors | Validated locally before the post, and owned by FerroCHART | The ITS-REST error body is optional, conditional, and carries no path | Rendering the CDR's error body |
| Compile site | The server | The owner's decision, and `openehr-its` has no WASM-capable feature set | Compiling in the browser |
| Renderer | Leptos client-side, Trunk, the FerroTERM recipe | Proven in the family, and the boundary and bundle guards come with it | A JavaScript front end; server-rendered HTML |
| Acceptance | A vendored per-datatype grid plus a round trip and a replay test | The openEHR conformance component is DEVELOPMENT and ITS-REST conformance reads "tbd." | Certifying against a published suite |

## 16. What is outside, and what is reported upstream

Outside: authoring or editing archetypes and templates, which is FerroCKM's
job; being a CDR, which is FerroEHR's; and mapping compositions to FHIR or
OMOP, which is FerroBRIDGE's.

The specification defects and silences found during this research, each an
`upstream-report` candidate:

1. **No per-node validation error on the ITS-REST wire.** The error body is
   optional, conditional on `Prefer: return=representation`, and its `errors`
   array is `DV_CODED_TEXT` with no path field.
2. **The ADL 1.4 operational template has no prose specification**, only a
   vendor-authored XSD from 2007 updated in 2010, while three openEHR
   artefacts name its classes differently.
3. **The web template is unspecified**, yet Simplified Formats is STABLE, a
   media type is registered, and two REST endpoints depend on the format.
4. **AQL says nothing about path stability across template revisions**, while
   specialisation provably changes the codes a path is built from.
5. **No canonical URI exists** for an openEHR terminology, an archetype-local
   value set, or a template-local value set, so every implementation that
   wants to expand a local value set has to mint one and they will not agree.
   The openEHR Base FHIR implementation guide covers the RM support
   terminology only, and is an unpublished ci-build.
6. **`OpenehrProfile.xsd` has no `C_DV_SCALE`**, so an ADL 1.4 operational
   template cannot express a `DV_SCALE` constraint in the domain-type form
   even though RM 1.1.0 defines the type.
7. **`C_DV_STATE` is defined in the XSD and in no prose document.**
8. **`C_CODE_REFERENCE.referenceSetUri` is 1..1**, which blocks a
   multi-terminology binding. Already reported independently on the openEHR
   forum.
9. **ITS-REST section Conformance reads "tbd."** in a released specification.

## Sources

- openEHR RM Release-1.1.0: <https://specifications.openehr.org/releases/RM/Release-1.1.0>
- openEHR AM Release-2.3.0: <https://specifications.openehr.org/releases/AM/Release-2.3.0>
- openEHR BASE Release-1.2.0: <https://specifications.openehr.org/releases/BASE/Release-1.2.0>
- openEHR QUERY Release-1.1.0: <https://specifications.openehr.org/releases/QUERY/Release-1.1.0>
- openEHR ITS-REST Release-1.1.0: <https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0>
- openEHR ITS-XML 2.0.0, the AM Release-1.4 XSD family: <https://specifications.openehr.org/releases/ITS-XML/latest/components/AM/Release-1.4/>
- openEHR release baseline: <https://specifications.openehr.org/release_baseline>
- HL7 FHIR R4 terminology service: <https://hl7.org/fhir/R4/terminology-service.html>
- The openEHR Base FHIR implementation guide, an unpublished ci-build: <https://build.fhir.org/ig/FHIR/openehr-base-ig/>
