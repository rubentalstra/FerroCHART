# Pinned versions

Every pin in FerroCHART has one source of truth, `docs/VERSIONS.md`, and
`scripts/checks/versions.sh` fails the build when a file disagrees with it.

## The specifications

| Component | Pin |
|---|---|
| openEHR RM | Release-1.1.0 |
| openEHR AM | Release-2.3.0 |
| openEHR ITS-REST | Release-1.1.0 |
| openEHR AQL (QUERY) | Release-1.1.0 |
| openEHR BASE | Release-1.2.0 |
| openEHR ITS-XML | 2.0.0 |
| openEHR TERM | Release-3.0.0 |
| HL7 FHIR | R4 4.0.1 |

A component release carries documents at different maturity levels inside one
number, so a citation names the component release, the document, and the
section. The AM release number and the version of a document inside it are
different numbers and are never conflated.

## The model crates

`openehr-base`, `openehr-rm`, `openehr-am`, `openehr-adl`, `openehr-its` and
`openehr-query`, pinned together because the line releases in lockstep and each
patch is its own compatibility set.

The FHIR model is `fhir-types`, generated from the published HL7 FHIR packages
and released on a lockstep line of its own. FerroCHART reads its R4 module, to
match the FHIR pin above.

## The template corpus

The openEHR CKM library, fetched by `scripts/vendor/ckm-templates.sh` and
pinned per template by the `cid` that carries its asset version. CKM publishes
no repository-level licence, so only the exports that state one are
redistributed here.
