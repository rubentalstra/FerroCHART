# Build order

The tracker is the scope. Milestones are delivery promises, and a release is
cut when its milestone reaches zero open issues.

Six releases are published, `v0.0.1` through `v0.1.0`. A template compiles to a
form, a browser renders it against the layout a person authored, and the round
trip against a real CDR runs on every pull request: measured against the 123
openEHR CKM templates this repository vendors, 121 read and all 121 derive,
2621 fields across 1775 groups.

This is a record of what each release added, with the current line at the end.
The full order is section 14 of the [architecture document][arch], and the
tracker is the live version.

| Release | What it adds |
|---|---|
| `v0.0.1` | The design of record, the workspace, the pin matrix, the gates. |
| `v0.0.2` | The vendored template corpus, both template readers behind one internal constraint model, and the release supply chain. |
| `v0.0.3` | The field derivation table, the form definition type, and snapshots over the whole corpus. |
| `v0.0.4` | The overlay store, its geometry, replay, and the differential report. |
| `v0.0.5` | The ITS-REST client, the composition builder, read-back, validation, and the terminology client. |
| `v0.1.0` | The renderer: a control for every field kind, the screens it draws them on, and the published format a renderer reads. |
| `v0.1.1` | In progress. The renderer and the release: a form a person can actually fill, the layout overlay reaching the browser, the round trip against a real CDR in CI, and every published artefact carrying what it claims. |

After it, `v0.1.2` closes the gaps between the entered-value document and the
Reference Model and turns the round trip into an instrument, and `v0.2.0` is
the surface a person lays a form out on. The
[milestones](https://github.com/rubentalstra/FerroCHART/milestones) carry what
is in each, and a release is cut when its milestone reaches zero open issues.

[arch]: https://github.com/rubentalstra/FerroCHART/blob/main/docs/architecture.md
