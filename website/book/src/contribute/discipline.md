# The working discipline

<!-- toc -->

## The specification is the oracle

The conformance authority is the openEHR Reference Model, the Archetype Object
Model and ADL, the operational template specifications, openEHR ITS-REST, AQL,
and the HL7 FHIR terminology service API. Never memory, and never another
implementation's behaviour. A conformance-relevant decision cites its
specification and section.

The Better web template is a compatibility target. No openEHR specification
defines it, every mention of it says so, and the Reference Model wins where the
two disagree. The FLAT and structured formats are specified: openEHR ITS-REST
Release-1.1.0 publishes `simplified_formats.html` in the STABLE state, and it
is the authority for their media types, their field identifiers, and their
Reference Model mapping.

## A form never admits what the template refuses

The form definition is a projection of the operational template. A CDR
rejecting a COMPOSITION that FerroCHART built and validated is always a defect
here.

## The gates

Every pull request runs shell, workflow and container linting, the comment-style
guard, the version-matrix guard, and the Rust set: `rustfmt`, `clippy` with
warnings denied, tests under `--locked`, doctests, `rustdoc` with warnings
denied, an MSRV check, and `cargo deny`. One required check, `conclusion`,
stands for all of them.

No gate is ever weakened to go green, and no test is skipped, weakened or
edited to route around a defect it exposes.

## Safety posture

`unsafe` is forbidden, not discouraged. Application code carries no `unwrap`,
`expect`, `panic!` or panicking index. Recoverable failures are typed errors
that carry their cause. An upstream failure is never flattened into a success
or a default, because a silently wrong clinical record is worse than a loud
failure.

## No patient data, ever

Fixtures are synthetic content invented for the test. This project sits at the
point a clinician types, so a real reproduction would be easy to create by
accident.

## Tracker and releases

The open issue list is the worklist. One type label and one priority label per
issue, milestones are releases, and a release is cut when its milestone reaches
zero open issues. Every change with a user-visible effect adds a changelog
entry in the same pull request.
