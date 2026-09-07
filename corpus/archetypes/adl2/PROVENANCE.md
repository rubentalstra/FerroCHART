<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# The ADL 2 archetype pack: provenance

Fetched from <https://github.com/openEHR/adl-archetypes>, directory `Reference/CKM_2013_12_09`, at commit
`093c77ea003742b9540e3dd377d615e2b26f2996`, by `scripts/vendor/adl2-archetypes.sh`.

The pin is the commit. This file states no fetch time on purpose: a
timestamp would make a committed record go dirty every time anyone runs
the script, which trains a reader to ignore the diff.

Upstream describes the tree as archetypes exported from the openEHR
Clinical Knowledge Manager on 2013-12-09.

**Every ADL 2 half declares `generated`** (openEHR AM Release-2.3.0
`ADL2.html` section 7.5, the generated indicator), so the pack is one
authoring plus a conversion rather than two independent authorings. That
bounds what a comparison across the pair can prove: agreement shows the
conversion preserved something, not that two people modelled the same
concept the same way. Issue #59 measured the consequences.

- ADL 2 archetypes (`*.adls`): 322
- ADL 1.4 twins (`*.adl`): 330
- Archetypes present in both dialects: 321

## Why this source and not CKM

The live openEHR CKM publishes ADL 1.4 only. Its archetype export answers
`adl` and `xml`, and `adl2`, `adls` and `opt` all return 404, verified on
the wire on 2026-09-07. The ADL 2 side therefore comes from this pinned
library.

It is never produced by running an ADL 1.4 to ADL 2 converter of our own.
That would validate the ADL 2 reader against output this project
generated, which proves nothing about either.

## Nothing here is committed

The upstream repository carries no licence file. Of the
652 archetypes in this tree, 1 states a
licence of its own; every other file carries a copyright line and nothing
that grants redistribution.

Silence is not permission (`.claude/rules/vendored-inputs.md`), so this
repository ships none of it. `.gitignore` refuses the archetype files, and
this record is committed in their place so the omission is visible. Run
the fetch script to get the tree locally.

This is a stricter reading than a sibling project applied to the same
upstream, and it is deliberate: a provenance record is a legal record, and
an unstated licence is not a permissive one.

## The one file that states a licence

| file | licence as stated |
|---|---|
| `composition/openEHR-EHR-COMPOSITION.t_encounter_opt_test.v1.0.0.adls` | Creative Commons CC-BY 4.0 unported <http://creativecommons.org/> |

## No patient data

These are clinical models rather than clinical data. Nothing here is a
record about a person, and nothing in this repository's tests may be
(`.claude/rules/testing.md`).
