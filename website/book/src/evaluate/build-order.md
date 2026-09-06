# Build order

Each release is green before the next starts. The full order, with what each
stage owes, is section 14 of the [architecture document][arch]; the tracker is
the live version.

| Release | What it adds |
|---|---|
| `v0.0.1` | The design of record, the workspace, the pin matrix, the gates. |
| `v0.0.2` | The vendored template corpus, both template readers behind one internal constraint model, and the release supply chain. |
| `v0.0.3` | The field derivation table, the form definition type, and snapshots over the whole corpus. |
| `v0.0.4` | The overlay store, replay, and the differential report. |
| `v0.0.5` | The ITS-REST client, the composition builder, read-back, validation, and the terminology client. |
| `v0.1.0` | The renderer, the overlay authoring surface, and a round trip proven against a CDR. |

The tracker is the scope. Milestones are delivery promises, and a release is
cut when its milestone reaches zero open issues.

[arch]: https://github.com/rubentalstra/FerroCHART/blob/main/docs/architecture.md
