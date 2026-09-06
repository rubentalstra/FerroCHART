# FerroCHART

FerroCHART is a pure-Rust openEHR form builder and renderer. It compiles an
operational template into a form definition, renders that form for a
clinician, and turns the entered values back into a COMPOSITION committed to
an openEHR Clinical Data Repository over the ITS-REST API. It reads
compositions back into the same form, so one definition serves data entry,
review, and editing.

It runs as its own server beside any openEHR CDR, and uses any FHIR
terminology server to expand the value sets behind coded fields.

## There is no binary yet

This documents a design and the code being built against it. The design of
record is [`docs/architecture.md`][arch] in the repository, where every
decision carries a citation to a primary source or an explicit note that no
specification governs it.

What exists today: the architecture, the Cargo workspace, the vendored
template corpus, and the gates. What each release adds is the
[build order](evaluate/build-order.md).

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
