<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# <img src="/assets/brand/ferrochart-lockup-auto.svg" alt="FerroCHART" width="244" height="56">

<!-- badges:begin -->
[![CI](https://github.com/rubentalstra/FerroCHART/actions/workflows/ci.yml/badge.svg)](https://github.com/rubentalstra/FerroCHART/actions/workflows/ci.yml)
[![CodeQL](https://github.com/rubentalstra/FerroCHART/actions/workflows/codeql.yml/badge.svg)](https://github.com/rubentalstra/FerroCHART/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/rubentalstra/FerroCHART/badge)](https://scorecard.dev/viewer/?uri=github.com/rubentalstra/FerroCHART)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue.svg)](LICENSE)
<!-- badges:end -->

A pure-Rust openEHR form builder and renderer. It compiles an operational
template into a form definition, renders that form for a clinician, and turns
the entered values back into a COMPOSITION committed to an openEHR Clinical
Data Repository. It reads compositions back into the same form, so one
definition serves data entry, review, and editing.

It runs as its own server beside any openEHR CDR reached over the openEHR
ITS-REST API, and uses any FHIR terminology server to expand the value sets
behind coded fields.

## Status: the design is settled, the engine is being built

The design of record is [`docs/architecture.md`](docs/architecture.md), the
output of the research program on
[issue #1](https://github.com/rubentalstra/FerroCHART/issues/1), where every
decision carries a citation or an explicit note that no specification governs
it.

A release publishes signed binaries and a container image, and the build order
in `docs/architecture.md` section 14 says what each one adds. **Nothing here
compiles a template into a form yet**, so a release today is the scaffolding
rather than the product. Watch the milestones for when that changes.

## Why this exists

openEHR separates the clinical model from the software, which is what makes the
data outlive the vendor. The cost is that a template is not a screen: something
has to turn an operational template into a form a nurse can fill in during a
ward round, and turn what they typed back into a valid COMPOSITION.

The tooling that does this well is commercial. The open source options are thin
enough that people running openEHR in a hospital build their own, one form at a
time, or go without. That gap is the reason for this project, and it was named
by the openEHR community rather than invented here.

## What is decided

- **A form is compiled from its operational template.** Field kinds come from
  the Reference Model type at each node: `DV_QUANTITY` becomes a number with a
  unit, `DV_CODED_TEXT` becomes a selection bound to a value set, a `CLUSTER`
  that may repeat becomes a repeatable group. No form is hand-written per
  template, and no field is hand-coded per archetype.
- **Hand-authored layout lives in a separate overlay**, keyed by AQL path.
  Templates get revised, and the layout, labels, help text, and visibility
  rules a person spent hours on must survive the revision. Recompile from the
  new template, replay the overlay, and report which paths disappeared. An
  editor that loses that work on a template update is the failure mode this
  design exists to avoid.
- **Any CDR, over ITS-REST.** FerroCHART is a client of the openEHR REST API,
  never a compile-time dependency of a CDR. It works against the CDR a hospital
  already runs. A form builder that works with only one CDR is no use to the
  people who asked for this.
- **Validation belongs on the server.** Form data is validated against the
  operational template before a COMPOSITION is built, so a clinician gets an
  error on the field they got wrong rather than one rejection for the whole
  document.
- **Pure Rust.** No JVM, and a single binary, like the rest of the family.
- **Business Source License 1.1.** Free for non-commercial use, a commercial
  licence for production use in a business. See below.

## What is decided since the research closed

Both template generations are read, and they normalize into one internal
constraint model so the field derivation is written once. The openEHR model
comes from the published `openehr-*` crates rather than a generator here. The
form definition is FerroCHART's own type projected from the web template,
which is a compatibility target and not a specification. The compiler runs on
the server and the renderer reads the definition it serves. The acceptance
instrument is a per-datatype template grid, a round trip against a CDR, and an
overlay replay against a real template revision.

The reasoning and the citations are in
[`docs/architecture.md`](docs/architecture.md) §15, the decision register.

## Licensing

FerroCHART is source-available under the Business Source License 1.1
([`LICENSE`](LICENSE), [`NOTICE`](NOTICE)), with no open-core tier: the
compiler, the renderer, the server, and the tools are in this repository under
the one licence, and nothing is held back to be sold back to you.

The licence lets you read, build, modify, and redistribute the source without a
fee and without asking anyone, and it covers every non-production use:
development, testing, evaluation, and prototyping. Production use is free for
Non-Commercial Purposes, which the licence defines as personal use, academic or
scientific research, teaching, and use by a non-profit organisation or public
body that is not in the course of a business, does not deliver a service for
payment, and is not for commercial advantage. Any other production use needs a
commercial licence from the Licensor: a hospital, clinic, or care provider
running FerroCHART for its patients needs one, and so does a vendor,
integrator, or any company running it in production. Offering FerroCHART, or a
work derived from it, to third parties as a hosted, managed, or embedded
service that builds, renders, or captures health data, and selling,
sublicensing, or otherwise distributing it for a fee on its own or inside
another product, need a commercial licence in every case. Each version becomes
Apache License 2.0 four years after that version is published. The commercial
licence starts with a short conversation with the maintainer named in
[MAINTAINERS.md](MAINTAINERS.md).

Contributions are licensed inbound equals outbound under the same licence, and
there is no contributor licence agreement and no copyright assignment. Vendored
specifications and third-party material keep their upstream terms, recorded
beside them.

## The family

FerroCHART is one of the [FerroHEALTH](https://ferrohealth.eu) servers:
[FerroEHR](https://github.com/rubentalstra/FerroEHR) stores the data,
[FerroTERM](https://github.com/rubentalstra/FerroTERM) answers the terminology
questions, [FerroBRIDGE](https://github.com/rubentalstra/FerroBRIDGE) moves it
to FHIR and OMOP, and FerroCHART is how it gets entered in the first place.
Each runs on its own and against other people's servers.

## Contributing

[`CONTRIBUTING.md`](CONTRIBUTING.md) has the rules, and the open issues are the
worklist. While the engine is being built the most useful contribution
is evidence: a specification citation, a measurement, or first-hand experience
building and running clinical forms over openEHR. If you have watched a
clinician use a form and seen where it failed them, that is worth more here
than a pull request.

- [`CLAUDE.md`](CLAUDE.md): the working discipline, in full.
- [`AI_STATEMENT.md`](AI_STATEMENT.md): how AI tools are used to build this,
  and what they are not allowed to do.
- [`GOVERNANCE.md`](GOVERNANCE.md), [`MAINTAINERS.md`](MAINTAINERS.md): who
  decides, and the honest answer about the bus factor.
- [`SECURITY.md`](SECURITY.md): report a vulnerability privately.
