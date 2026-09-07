---
name: parallel-agents-need-worktrees
description: "Every concurrent implementation agent gets its own git worktree; two agents plus the orchestrator in one checkout corrupted a commit on 2026-09-06"
metadata:
  node_type: memory
  type: feedback
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

Run every concurrent implementation agent in its own `git worktree`, and tell
it so in its prompt. Two agents and the orchestrator sharing one checkout is
not a small risk, it is a corrupted commit within the hour.

**Why:** on 2026-09-06 two implementation agents ran against
`/Users/rubentalstra/RustroverProjects/FerroCHART` at once while the
orchestrator also worked there. One agent had `openehr-*` dependencies
half-added to `Cargo.toml` and `Cargo.lock`. The orchestrator's `git add -A`
on an unrelated documentation branch swept those in, and CI failed on the
documentation pull request for a licence rejection in a dependency that
branch never intended to have. The agent that used a worktree
(`git worktree add`) finished cleanly and never touched the shared tree; the
one that did not caused the collision.

**How to apply:** in the prompt for a background implementation agent, say
that it must create its own worktree from `main` and work there. In the
orchestrator's own work, stage by explicit path and never `git add -A` while
an agent is running, and run `git status --short` before every commit to
confirm every path is yours. Also state the branch explicitly with
`git switch -c`: a `git switch` that was part of a command a hook blocked
leaves you on the previous branch, which is how one commit landed on the wrong
one that day. See [[pr-auto-merge]].

**Remove a worktree when its pull request merges.** Each one carries its own
`target/`, which reaches 5 to 8 GB after a full test run, so eleven merged
worktrees filled the disk on 2026-09-07 and a `Bash` call failed with
`ENOSPC` mid-task. `git worktree remove --force <path>` then
`git worktree prune`; that run recovered 184 GB. The isolation is still
right, and the cleanup is part of the merge, not an afterthought.
