---
name: product-scope
description: "The owner's product statement is the ceiling on what this repository may claim: an openEHR form builder and renderer, CDR-agnostic, everything else is research on issue #1"
metadata:
  node_type: memory
  type: project
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

FerroCHART compiles an openEHR operational template into a form definition,
renders that form for a clinician, and turns what they type back into a
COMPOSITION committed to an openEHR CDR. It reads compositions back into the
same form. That statement is the ceiling on what any document here may claim.

Decided:

- **CDR-agnostic over openEHR ITS-REST.** FerroCHART works against EHRbase,
  Better, vitagroup, and FerroEHR alike. FerroEHR is the reference CDR, never a
  requirement. This is not a nice-to-have: a form builder that only works with
  one CDR is useless to the community that asked for it.
- **The layout overlay is separate from the compiled definition.** A template
  is revised; hand-authored layout must survive the revision. Recompile from
  the new template, replay the overlay, report which paths disappeared.
- **Terminology comes from a FHIR terminology server** for external value sets,
  and from the template itself for local at-codes.
- **Pure Rust, one binary.** The reference renderer ships embedded in it.

Open, and answered by research on issue #1: the crate layout, the form
definition format and how far it tracks the de facto web template, ADL 1.4
against ADL 2 support, the renderer's technology, and the acceptance
instrument.

**Why this exists:** the request came from Sev Kohler in the openEHR community
on 2026-09-06, who said an open source form builder is the thing the community
is missing and offered spec input. That is the audience. Anything that makes
the product less useful to a hospital already running someone else's CDR is
the wrong call.
