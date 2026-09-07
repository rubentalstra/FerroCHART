<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Memory index

- [Product scope](product-scope.md): the owner's product statement is the
  ceiling on what this repository may claim; an openEHR form builder and
  renderer, CDR-agnostic over ITS-REST, layout overlay kept separate from the
  compiled definition; everything else is research on issue #1; the request
  came from the openEHR community on 2026-09-06
- [Domain ferrochart.eu](domain-ferrochart-eu.md): registered 2026-09-06 at
  Vimexx on the family's zxcs nameservers; ferroform.eu was already taken,
  which is why the product is not called FerroFORM; the domain is a Pages
  setting, never a `CNAME` file
- [Sibling projects](sibling-projects.md): FerroEHR at `../ferroehr` is the
  reference CDR, FerroTERM at `../FerroTERM` the reference terminology server,
  FerroBRIDGE at `../FerroBRIDGE` is where this configuration came from, and
  FerroHEALTH at `../FerroHEALTH` carries the brand; all are read-only from
  here
- [Licence: BUSL 1.1](license-busl.md): set at repository creation 2026-09-06
  on the family's terms; non-commercial production free, commercial production
  needs a licence, Apache 2.0 four years after each version; inbound equals
  outbound, no contributor licence agreement
- [Owner work style](owner-work-style.md): research-first and evidence-based,
  from first principles; confirm foundational decisions before scaffolding; no
  code while the design is open
- [PR auto-merge](pr-auto-merge.md): enable auto-merge on every pull request
  the moment it is opened (`gh pr merge <n> --auto --squash --delete-branch`)
- [Community contact: Severin Kohler](community-contact-severin-kohler.md):
  the openEHR community member who asked for this product on 2026-09-06
  (EHRbase, OMOCL, FHIRconnect); offered specification and forms review;
  suggested serving openEHR value sets as FHIR ValueSets
- [Parallel agents need worktrees](parallel-agents-need-worktrees.md): every
  concurrent implementation agent gets its own `git worktree`; two agents plus
  the orchestrator in one checkout corrupted a commit on 2026-09-06
- [Memory lives in the repo](memory-lives-in-repo.md): every learning is a
  tracked file in `.claude/memory/`, never a per-user note
