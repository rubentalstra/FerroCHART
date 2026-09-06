---
name: license-busl
description: "FerroCHART's own code and text are under the Business Source License 1.1 on the family's terms, set when the repository was created on 2026-09-06; Apache 2.0 is the Change License four years after each version"
metadata:
  node_type: memory
  type: project
---

<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

FerroCHART's own code and text are under the **Business Source License 1.1**,
on the terms FerroEHR, FerroTERM, and FerroBRIDGE use. The owner set this when
the repository was created on 2026-09-06, so there is no earlier licence in
this repository's history.

The terms, as `LICENSE` and `NOTICE` state them:

- Free to read, build, modify, and redistribute.
- Free for every non-production use and for non-commercial production use.
- A commercial licence from the Licensor for any other production use, always
  for a hosted, managed, or embedded service and for for-fee distribution.
- The Change License is Apache License 2.0, four years after each version.
- Contribution is inbound equals outbound under the same licence. There is no
  contributor licence agreement and no copyright assignment.

The Additional Use Grant names the service this repository's work is: a service
through which anyone other than you and your affiliates builds, renders, or
captures health data through the Licensed Work.

**How to apply:**

- Every first-party file carries `SPDX-FileCopyrightText: Ruben Talstra` and
  `SPDX-License-Identifier: BUSL-1.1` in its header. The pin-matrix guard
  (`scripts/checks/versions.sh`) fails on a stale licence claim anywhere in the
  tree.
- `LICENSE` is the one file that names Apache 2.0 as a licence of its own,
  where it is the Change License. Nowhere else describes the project as
  Apache-licensed.
- `deny.toml` lists BUSL-1.1 for the workspace's own crates only; the
  dependency allowlist is a separate decision.
- Vendored specifications and third-party material keep their upstream terms,
  recorded in a `PROVENANCE.md` beside each vendored tree
  (`.claude/rules/vendored-inputs.md`).
- The licence is decided per repository by the owner and never assumed from a
  sibling (see `sibling-projects.md`).
- The family site states the terms beside every licence link, because the
  BUSL boilerplate fills none of its parameters in. Keep the README's licensing
  paragraph and the site in step.
