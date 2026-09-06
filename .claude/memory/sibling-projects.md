---
name: sibling-projects
description: "The four sibling Ferro repositories checked out beside FerroCHART, what each one is to this project, and the rule that none of them is ever edited from here"
metadata:
  node_type: memory
  type: project
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

FerroCHART is the newest project in the Ferro family, created 2026-09-06. The
others are checked out beside it on the owner's machine:

- **FerroEHR** at `../ferroehr`: the openEHR CDR. It is the **reference CDR**
  for FerroCHART, reached over the openEHR ITS-REST API, and never a
  compile-time dependency. FerroCHART commits COMPOSITIONs to any conformant
  CDR; FerroEHR is the one it is tested against first.
- **FerroTERM** at `../FerroTERM`: the FHIR terminology server. It is the
  **reference terminology server** for expanding the value sets behind coded
  fields, and the project this repository's working configuration descends
  from.
- **FerroCKM** at `../FerroCKM`: the Clinical Knowledge Manager. It governs the
  archetypes and operational templates that FerroCHART compiles, so it is
  **upstream of this project in the modelling chain**, not a runtime
  dependency. A template arrives here as a file.
- **FerroBRIDGE** at `../FerroBRIDGE`: the openEHR to FHIR and OMOP bridge.
  This repository's `.claude/` configuration was copied from it on 2026-09-06
  and adapted, so it is the closest reference for how a rule here is meant to
  read.
- **FerroHEALTH** at `../FerroHEALTH`: the family site and brand. FerroCHART's
  mark, hue, and product card live there once they exist.

**How to apply:**

- Read any of them freely: their code, their rules, their git history, their
  closed issues. That is the fastest source of prior art for a decision here.
- **Never edit any of them from this repository.** No file changes, no commits,
  no branches in a sibling from a FerroCHART session. A code change a sibling
  needs is made there, in its own session.
- **A tracker issue in a sibling MAY be filed from here when the owner asks.**
  The owner's ruling on FerroBRIDGE, 2026-09-05: "you just create an issue
  directly there because it's my repo". File it in that repository's tracker
  with its labels and milestone, and record it on the FerroCHART issue that
  depends on it. Issues only; code stays theirs.
- None is an oracle. A response from a running FerroEHR or FerroTERM is
  evidence in a comparison; the specification is the authority
  (`.claude/rules/spec-adherence.md`). A defect found in one of them is
  reported to that project rather than worked around here.
- All are under the Business Source License 1.1 on the same terms. Never copy
  code between the repositories anyway: each is its own Licensed Work with its
  own Licensor copy, and a licence decision is made per repository by the
  owner, never assumed from a sibling.
