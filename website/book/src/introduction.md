# FerroCHART

FerroCHART is a pure-Rust openEHR form builder and renderer. It compiles an
operational template into a form definition, renders that form for a
clinician, and turns the entered values back into a COMPOSITION committed to
an openEHR Clinical Data Repository over the ITS-REST API. It reads
compositions back into the same form, so one definition serves data entry,
review, and editing.

It runs as its own server beside any openEHR CDR, and uses any FHIR
terminology server to expand the value sets behind coded fields.

## What a template becomes

The compiler works. It reads both ADL generations, and 121 of the 123
committed CKM templates derive a form definition, 2658 fields across 1789
groups. The server publishes that definition and a browser draws it, with a
control for every field kind the derivation produces, each admitting what its
template admits and refusing the rest.

![A form the compiler derived from an operational template](operate/img/renderer/form-family-history-summary-item-r2.png)

That picture was taken by the end-to-end battery rather than by hand, and
[the renderer](operate/renderer.md) has the rest of the screens.

Two things do not work yet. The layout overlay's authoring surface is not
built, so a form is drawn in template order; the overlay itself is, including
its replay across a template revision and its report of what matched, moved or
disappeared. And the round trip has never run against a real CDR: the whole
build, validate, post and accept path is exercised against a mock, and the
test that would prove the claim is [issue #126][live]. What each release adds
is the [build order](evaluate/build-order.md).

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
[live]: https://github.com/rubentalstra/FerroCHART/issues/126
