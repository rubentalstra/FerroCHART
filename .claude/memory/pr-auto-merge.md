---
name: pr-auto-merge
description: "Every pull request gets auto-merge enabled the moment it is opened (gh pr merge <n> --auto --squash --delete-branch) so it lands when the conclusion check is green; owner instruction 2026-09-04"
metadata:
  node_type: memory
  type: feedback
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

After opening a pull request, enable auto-merge at once:

```sh
gh pr create ... && gh pr merge <n> --auto --squash --delete-branch
```

Never leave a pull request waiting for a manual merge.

**No exceptions, including a PR the owner will want to read.** Reasoning that
a design document, an architecture decision, or a large change deserves a
manual merge is the wrong call: the owner reads what landed on `main` and
opens a follow-up issue if something is wrong. Withholding auto-merge to
create a review gate the owner did not ask for wastes their time
(FerroCHART, 2026-09-06).

**Why:** the owner asked on 2026-09-04: "for PR's do not forget to trigger
auto merge okay!! so when the CI is green it will be merged". The `main`
ruleset requires the `conclusion` status check, so auto-merge is the correct
hand-off: GitHub merges the moment the check passes. The repository has
`allow_auto_merge` and `delete_branch_on_merge` on.

**How to apply:** the instruction was given on FerroBRIDGE and applies across
the family. Where the required `conclusion` check does not yet report, an armed
auto-merge waits: say so in the hand-off and let the owner merge through the
admin bypass, and never use `--admin` without asking.
