# FerroCHART

FerroCHART is a pure-Rust openEHR form builder and renderer. It compiles an
operational template into a form definition, renders that form for a
clinician, and turns the entered values back into a COMPOSITION committed to
an openEHR Clinical Data Repository over the ITS-REST API. It reads
compositions back into the same form, so one definition serves data entry,
review, and editing.

It runs as its own server beside any openEHR CDR, and uses any FHIR
terminology server to expand the value sets behind coded fields.

## Nothing renders a form yet

The compiler works. It reads both ADL generations, and 121 of the 123
committed CKM templates derive a form definition, 2658 fields across 1789
groups. The overlay replays hand-authored layout onto a recompiled form and
reports what changed. Releases publish binaries for four Linux targets and a
container image.

What the binary does today is read its configuration, bind, and answer a
health probe. There is no renderer, so no clinician sees a form; nothing
builds a COMPOSITION or commits one to a CDR; there is no authoring surface
for the layout; and no round trip has been proven against a running CDR. What
each release adds is the [build order](evaluate/build-order.md).

The design of record is [`docs/architecture.md`][arch] in the repository,
where every decision carries a citation to a primary source or an explicit
note that no specification governs it.

## Why it exists

openEHR separates the clinical model from the software, which is what makes
the data outlive the vendor. The cost is that a template is not a screen.
Something has to turn an operational template into a form a nurse can fill in
during a ward round, and turn what they typed back into a valid COMPOSITION.

The tooling that does this well is commercial. The open source options are
thin enough that people running openEHR in a hospital build their own, one
form at a time, or go without. That gap is the reason for this project, and it
was named by the openEHR community rather than invented here.

[arch]: https://github.com/rubentalstra/FerroCHART/blob/main/docs/architecture.md
