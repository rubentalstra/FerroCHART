---
name: community-contact-severin-kohler
description: "Severin Kohler (openEHR community, EHRbase, author of OMOCL and FHIRconnect) asked for an open-source openEHR form builder on 2026-09-06, offered specification help, and suggested serving openEHR value sets as FHIR ValueSets"
metadata:
  node_type: memory
  type: reference
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

FerroCHART exists because Severin Kohler asked for it in the openEHR community
chat on 2026-09-06. He works on EHRbase, moved to vitasystems, and wrote OMOCL
and FHIRconnect, the two mapping languages FerroBRIDGE consumes. He offered
input on the specifications, on terminology servers, and on forms, and he
reads a paper review most weeks, so the offer is real rather than polite.

What he asked for, in his words: "a tool to build and render forms on openEHR
templates, so clinicians and nurses can display and input data into openEHR",
because "there is currently not a good one opensource out there", and it is
the piece missing from the open-source hospital research stack he published.

Three things he said that change the work:

- **Serve openEHR value sets as FHIR ValueSets.** An archetype's internal
  value sets have no FHIR representation today, so a terminology server that
  expands them would be the first. This joins FerroCHART's coded-field path to
  FerroTERM and is a research question on issue #1.
- **Mappings need clinicians for the hard cases.** He said the last five per
  cent of an openEHR-to-FHIR or openEHR-to-OMOP mapping takes clinical
  judgement and cannot be generated, from his own work with an AI lab. The
  same holds for form layout, which is why the overlay exists.
- **The form builder has to be editable: a person moves and sizes the fields
  themselves.** On 2026-09-07, called very important: "important is that this
  form builder is editable so the forms itself, so you can move and size the
  form fields etc". So the authoring surface is direct manipulation over a
  live form, not a property sheet beside a tree, and the overlay has to carry
  geometry it did not carry when it was first built (#69, #27).
- **The licence text has to answer the hospital question at a glance.** He
  asked whether hospitals and research institutes may run it in production,
  and found the FerroHEALTH draft site linking the boilerplate BUSL text with
  no answer. The repository READMEs answer it; the site did not.

**How to apply:** send him the architecture document from issue #1 when it
lands, and ask him to review the field derivation table and the prior-art
section, since he has read the tooling from the inside. His address is the
openEHR community chat; the owner also gave him an email address. The next
meeting point is openEHR ehrcon, where the owner speaks about DICOM and
radiotherapy DVH curves. See [[product-scope]] and [[sibling-projects]].
