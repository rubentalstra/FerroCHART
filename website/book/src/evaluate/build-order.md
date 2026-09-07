# Build order

The tracker is the scope. Milestones are delivery promises, and a release is
cut when its milestone reaches zero open issues.

`v0.0.1` through `v0.0.3` are published. As of `v0.0.3` a template compiles to
a form: measured against the 123 openEHR CKM templates this repository
vendors, 121 read and all 121 derive, 2658 fields across 1789 groups.

The full order, with what each stage owes, is section 14 of the
[architecture document][arch]. The tracker is the live version.

| Release | What it adds |
|---|---|
| `v0.0.1` | The design of record, the workspace, the pin matrix, the gates. |
| `v0.0.2` | The vendored template corpus, both template readers behind one internal constraint model, and the release supply chain. |
| `v0.0.3` | The field derivation table, the form definition type, and snapshots over the whole corpus. |
| `v0.0.4` | The overlay store, its geometry, replay, and the differential report. |
| `v0.0.5` | The ITS-REST client, the composition builder, read-back, validation, and the terminology client. |
| `v0.1.0` | The renderer, the overlay authoring surface, and a round trip proven against a CDR. |

The tracker is the scope. Milestones are delivery promises, and a release is
cut when its milestone reaches zero open issues.

[arch]: https://github.com/rubentalstra/FerroCHART/blob/main/docs/architecture.md
