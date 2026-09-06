<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# The openEHR CKM template pack: provenance

Vendored verbatim from the openEHR Clinical Knowledge Manager REST API,
<https://ckm.openehr.org/ckm/rest/v1>, by
`scripts/vendor/ckm-templates.sh`. Each file is CKM's own Operational
Template export for the cited template.

- Fetched: 2026-09-06T21:37:02Z
- Templates listed by CKM: 306
- Committed here, licence stated: 123
- Withheld, licence not stated: 182
- Unreachable at fetch time: 1

## Pinning

CKM is not a git repository and publishes no tags, so a commit pin is not
available. Each template is pinned by its `cid`, which carries the asset
version, together with the resource id and the asset version number. Those
are immutable for a given version of a template, so a re-run that returns a
different `cid` for the same file is a real change rather than a moving
reference.

## Licensing

CKM publishes no repository-level licence, and each export either states
one in `other_details id="licence"` or states nothing.

**A file stating a Creative Commons licence is committed here.** Those are
CC-BY-SA-4.0 and CC-BY-SA-3.0, whose terms are at
<https://creativecommons.org/licenses/by-sa/4.0/> and
<https://creativecommons.org/licenses/by-sa/3.0/>. Attribution rides along
in each file, which is vendored verbatim and never edited.

**A file stating nothing is not redistributed.** Silence is not
permission (`.claude/rules/vendored-inputs.md`), so the fetch puts those
under `unstated/`, which `.gitignore` refuses. Run the script to get them
locally when you want the full breadth; they are never committed and never
published.

Nothing here is relicensed. The project's own code and text are BUSL-1.1
(`CLAUDE.md`); this tree is not.

## No patient data

These are clinical models rather than clinical data. Nothing here is a
record about a person, and nothing in this repository's tests may be
(`.claude/rules/testing.md`).

## The pack

| file | cid | resource id | asset version | status | licence as stated |
|---|---|---|---|---|---|
| `aboriginal-and-torres-strait-islander-health-check-master.opt` | `1013.26.254` | `60e6d963-73ac-47b7-b3ab-9d47909ab13a` | 8 | INITIAL | CC-BY-SA-4.0 |
| `aedes-indices-jm.opt` | `1013.26.896` | `4f833aa8-967e-45d3-9791-3ed4c779ad59` | 2 | INITIAL | CC-BY-SA-4.0 |
| `alcohol-consumption-summary-item-r1.opt` | `1013.26.317` | `e05c4491-cd2f-459c-b6a7-d3bbb70cb486` | 2 | INITIAL | CC-BY-SA-4.0 |
| `alcohol-consumption-summary-item-r2.opt` | `1013.26.320` | `c3f27ab9-8ec3-4423-9585-88c69cbd2228` | 1 | INITIAL | CC-BY-SA-4.0 |
| `antibiotic-prophylaxis.opt` | `1013.26.1063` | `0a59f499-60c1-4d95-912a-14c8bf9b2be5` | 1 | INITIAL | CC-BY-SA-4.0 |
| `au-covid-19-likelihood-assessment.opt` | `1013.26.273` | `2864a632-4716-475e-9bd5-a95ba69f74df` | 2 | DRAFT | CC-BY-SA-4.0 |
| `birth-detail-jm.opt` | `1013.26.1056` | `2cf8a52d-0cff-48d5-86bc-8022fe025bce` | 2 | INITIAL | CC-BY-SA-4.0 |
| `birth-summary-jm.opt` | `1013.26.938` | `b90b0ad6-7111-4441-aa84-d8adaad56e65` | 4 | INITIAL | CC-BY-SA-4.0 |
| `blood-donation-summary-jm.opt` | `1013.26.1339` | `b8deed70-ffed-435b-891d-8bb413f87d88` | 1 | INITIAL | CC-BY-SA-4.0 |
| `body-temperature-jm.opt` | `1013.26.934` | `8e6b4d8e-0f79-4a52-82d3-db05c527c70a` | 2 | INITIAL | CC-BY-SA-4.0 |
| `bpg-administration-jm.opt` | `1013.26.1010` | `98a93170-f4c8-475f-8932-684eaa483c09` | 2 | INITIAL | CC-BY-SA-4.0 |
| `bpg-medication-screening-jm.opt` | `1013.26.1014` | `421a0d29-e3ea-4e40-9ba4-ad000f22ba5a` | 1 | INITIAL | CC-BY-SA-4.0 |
| `case-demographic-information-jm.opt` | `1013.26.916` | `11b7d75b-0dc0-4182-98d0-25d03a99b42f` | 7 | INITIAL | CC-BY-SA-4.0 |
| `cause-of-death-jm.opt` | `1013.26.929` | `cf915e1e-3d80-41b9-9a0f-806cf19047b6` | 2 | INITIAL | CC-BY-SA-4.0 |
| `clinical-context-jm.opt` | `1013.26.946` | `133e08f6-467e-4368-a10e-8ec11b46e6bf` | 5 | INITIAL | CC-BY-SA-4.0 |
| `clinical-management-jm.opt` | `1013.26.974` | `25201a07-33ec-4a1e-9f55-0ab501dede8e` | 8 | INITIAL | CC-BY-SA-4.0 |
| `clinical-notes-jm.opt` | `1013.26.959` | `c2a9729f-3349-40f8-a9e2-f1b9c6ad9c5d` | 2 | INITIAL | CC-BY-SA-4.0 |
| `clinical-screening-jm.opt` | `1013.26.936` | `fe4c154e-7415-4c6c-8f5b-e9e193e1212b` | 2 | INITIAL | CC-BY-SA-4.0 |
| `cohort.opt` | `1013.26.236` | `073e8e71-4ead-4d76-bc26-3ef3e3f2fe02` | 2 | INITIAL | CC-BY-SA-3.0 |
| `csf-blood-cell-count-and-differential.opt` | `1013.26.1033` | `6b4a3182-41af-4976-8e8e-a4f25101fc9a` | 1 | INITIAL | CC-BY-SA-4.0 |
| `date-of-birth-jm.opt` | `1013.26.913` | `34569352-0a74-4ff9-aebf-8ec9929eb948` | 2 | INITIAL | CC-BY-SA-4.0 |
| `death-summary-jm.opt` | `1013.26.931` | `c8f9988d-d7f5-491e-b30f-26215426354f` | 1 | INITIAL | CC-BY-SA-4.0 |
| `education-summary-item-r2.opt` | `1013.26.261` | `9117a86c-a261-4685-beb4-5d0d1e8f68cc` | 2 | INITIAL | CC-BY-SA-4.0 |
| `education-summary-jm.opt` | `1013.26.958` | `c6416355-d047-4028-b3b9-077db1b79a81` | 1 | INITIAL | CC-BY-SA-4.0 |
| `environmental-survey-jm.opt` | `1013.26.953` | `fe9b2c45-ce6a-4a1b-aa2e-0dc12cd3dd8e` | 1 | INITIAL | CC-BY-SA-4.0 |
| `exposure-profile-jm.opt` | `1013.26.955` | `4bfb75f9-9c5d-42a8-9fd8-d5a7e965fdaa` | 10 | INITIAL | CC-BY-SA-4.0 |
| `exposure-screening-jm.opt` | `1013.26.954` | `bc9e9651-0382-4b64-91a5-8161a44cadfa` | 2 | INITIAL | CC-BY-SA-4.0 |
| `family-history-summary-item-r1.opt` | `1013.26.253` | `f24ddee6-5c01-4154-ace8-67e626e12145` | 2 | INITIAL | CC-BY-SA-4.0 |
| `family-history-summary-item-r2.opt` | `1013.26.252` | `ea2fe922-4ca4-475c-a524-aa5dc14a03f6` | 2 | INITIAL | CC-BY-SA-4.0 |
| `family.opt` | `1013.26.235` | `abc930db-7740-4591-ad69-950c592b61cb` | 2 | INITIAL | CC-BY-SA-3.0 |
| `father-demographic-information-jm.opt` | `1013.26.1094` | `0abfaf70-a760-4f9a-a85a-ff02937113fe` | 1 | INITIAL | CC-BY-SA-4.0 |
| `field-investigator-classification-jm.opt` | `1013.26.930` | `28b2e01e-ab46-43c3-a436-e7710962c1c5` | 7 | INITIAL | CC-BY-SA-4.0 |
| `food-and-nutrition-summary-jm.opt` | `1013.26.940` | `84eba3a9-4517-4536-88ba-d9ef71b6a0aa` | 1 | INITIAL | CC-BY-SA-4.0 |
| `gender-item-r1.opt` | `1013.26.337` | `56b82098-69c6-49ee-94ed-f24d99f81624` | 1 | INITIAL | CC-BY-SA-4.0 |
| `genomics-example-template.opt` | `1013.26.223` | `71194c4f-28ad-44fd-834a-484a3eed2507` | 2 | DRAFT | CC-BY-SA-3.0 |
| `health-issue-item-r2.opt` | `1013.26.313` | `921aceba-1f4b-450f-a691-b726f8c5a315` | 2 | INITIAL | CC-BY-SA-4.0 |
| `home-environment-jm.opt` | `1013.26.960` | `c8e2cdb5-7047-4f5d-b9a7-ddee7344d100` | 4 | INITIAL | CC-BY-SA-4.0 |
| `imaging-examination-result-jm.opt` | `1013.26.972` | `006b271b-93e0-488f-81ee-1eb6aeebd64d` | 2 | INITIAL | CC-BY-SA-4.0 |
| `imaging-examination-result.opt` | `1013.26.266` | `9bd29942-0390-4953-970d-34f385e44b44` | 1 | DRAFT | CC-BY-SA-4.0 |
| `immunoglobulin-test-finding-jm.opt` | `1013.26.1129` | `13a4172a-1d36-4572-9b19-449744ca5d4d` | 2 | INITIAL | CC-BY-SA-4.0 |
| `india-ink-stain-findings.opt` | `1013.26.1034` | `82b86aa4-b27a-4d33-81b1-c15babd4c3dc` | 2 | INITIAL | CC-BY-SA-4.0 |
| `infant-clinical-context-jm.opt` | `1013.26.939` | `4d3a8ff5-0e39-4168-9fa4-46e8e542f321` | 8 | INITIAL | CC-BY-SA-4.0 |
| `infant-measurements-jm.opt` | `1013.26.937` | `ab1e6483-98b9-4f0e-99b5-813dba72524c` | 4 | INITIAL | CC-BY-SA-4.0 |
| `infectious-disease-investigation-metadata-jm.opt` | `1013.26.923` | `f4c7ce5a-4f84-47b0-bc66-309c61bb52be` | 6 | INITIAL | CC-BY-SA-4.0 |
| `inpatient-episode-details-jm.opt` | `1013.26.1236` | `5958fc8c-443f-4fc6-bb0a-c35020fa74f0` | 1 | INITIAL | CC-BY-SA-4.0 |
| `interpretation.opt` | `1013.26.238` | `b631c87e-16fb-4064-8d75-7c5af314b9ca` | 5 | INITIAL | CC-BY-SA-3.0 |
| `investigator-classification-initial-jm.opt` | `1013.26.925` | `ebac4d27-80be-4587-8b89-5718d0b97a31` | 4 | INITIAL | CC-BY-SA-4.0 |
| `investigator-diagnosis-jm.opt` | `1013.26.927` | `4df2507a-3982-48d2-b7bf-02d635f37cf7` | 1 | INITIAL | CC-BY-SA-4.0 |
| `ips-allergies-and-intolerances.opt` | `1013.26.1365` | `628500fc-6cb4-4471-a3e7-7931b7d4fc4a` | 2 | INITIAL | CC-BY-SA-4.0 |
| `ips-problem-list.opt` | `1013.26.1363` | `77750679-e20d-4e8f-aa77-1ad957cb6874` | 3 | INITIAL | CC-BY-SA-4.0 |
| `laboratory-analyte-result-interpretative-jm.opt` | `1013.26.966` | `4e2386aa-94d6-4cd1-aa03-ac18d922655d` | 2 | INITIAL | CC-BY-SA-4.0 |
| `laboratory-analyte-result-quantitative-jm.opt` | `1013.26.965` | `f86cd61b-0e0e-4a18-8c07-9c4e537bb4e4` | 3 | INITIAL | CC-BY-SA-4.0 |
| `laboratory-test-result-jm.opt` | `1013.26.973` | `90225e36-20a8-403a-8172-77e53b9990b2` | 4 | INITIAL | CC-BY-SA-4.0 |
| `local-final-infectious-disease-investigation-classification-jm.opt` | `1013.26.928` | `f86e5166-bbb2-4acd-b409-1e4d7f6ad733` | 1 | INITIAL | CC-BY-SA-4.0 |
| `management-screening-jm.opt` | `1013.26.969` | `d0a3b39f-2dcb-4d15-acdf-d30aadb7843f` | 4 | INITIAL | CC-BY-SA-4.0 |
| `medication-order-item-r1.opt` | `1013.26.341` | `d0f9f971-bc42-473c-aa09-af60eadb4f09` | 1 | INITIAL | CC-BY-SA-4.0 |
| `microbiology-culture-findings-jm.opt` | `1013.26.1029` | `b3cb1b10-b0d1-422e-a8b7-ffe1dc36d9dc` | 2 | INITIAL | CC-BY-SA-4.0 |
| `microbiology-parasitology-findings.opt` | `1013.26.1035` | `c54b42dd-fcb2-4493-8720-8f238375a1d9` | 2 | INITIAL | CC-BY-SA-4.0 |
| `molecular-microbial-test-findings-jm.opt` | `1013.26.1030` | `97961601-c3f0-4731-8dae-2baa51ea84c6` | 1 | INITIAL | CC-BY-SA-4.0 |
| `molecular-pathology-report.opt` | `1013.26.249` | `7737a066-aa00-499d-8d21-f21b7d0068d5` | 2 | DRAFT | CC-BY-SA-4.0 |
| `mother-demographic-information-jm.opt` | `1013.26.976` | `382ea4d0-88f3-454d-b138-354207819822` | 3 | INITIAL | CC-BY-SA-4.0 |
| `neuro-examination-findings-jm.opt` | `1013.26.935` | `162b4b8f-5a47-4751-a8cc-343310c6b78c` | 1 | INITIAL | CC-BY-SA-4.0 |
| `non-treponemal-screening-jm.opt` | `1013.26.1011` | `0bce7bdf-8326-438b-bd48-7fe8af446a12` | 1 | INITIAL | CC-BY-SA-4.0 |
| `obstetric-summary-item-r1.opt` | `1013.26.305` | `4a396e8b-dcb7-4f13-85e6-4605197a7829` | 3 | INITIAL | CC-BY-SA-4.0 |
| `obstetric-summary-jm.opt` | `1013.26.1013` | `942c1e3f-aa49-42a8-bd25-53788e28790e` | 2 | INITIAL | CC-BY-SA-4.0 |
| `occupation-summary-item-r2.opt` | `1013.26.260` | `8a781f30-9f64-4e67-8dbe-d592ebc1e5d9` | 3 | INITIAL | CC-BY-SA-4.0 |
| `occupation-summary-jm.opt` | `1013.26.961` | `82275b1b-765c-42fe-8a3f-2a2b19d54ac1` | 5 | INITIAL | CC-BY-SA-4.0 |
| `openehr-confirmed-covid-19-infection-report-v0-2.opt` | `1013.26.282` | `7426b67d-3011-4b5c-bff4-a8e5e728b1b9` | 2 | DRAFT | CC-BY-SA-4.0 |
| `openehr-confirmed-covid-19-infection-report-v0.opt` | `1013.26.271` | `4ad80730-73e1-4ef6-a00b-716f14dd467f` | 2 | INITIAL | CC-BY-SA-4.0 |
| `openehr-suspected-covid-19-assessment-v0.opt` | `1013.26.267` | `e3354674-7613-4646-bb1c-c6d8c2d86ae1` | 4 | INITIAL | CC-BY-SA-4.0 |
| `openehr-suspected-covid-19-risk-assessment-nephrology-v0.opt` | `1013.26.298` | `beb5066b-b024-4255-b31b-5dd6ce6f19a8` | 4 | INITIAL | CC-BY-SA-4.0 |
| `openehr-suspected-covid-19-risk-assessment-v0.opt` | `1013.26.286` | `aa5aa917-d440-4ce5-92ef-80611c81bfd3` | 3 | DRAFT | CC-BY-SA-4.0 |
| `organisation-healthcare-facility-jm.opt` | `1013.26.919` | `db555049-c847-47b5-9b9f-4f21c8922e5d` | 4 | INITIAL | CC-BY-SA-4.0 |
| `other-diagnosis-jm.opt` | `1013.26.926` | `d879b24b-c88f-4391-adcf-332d99111043` | 1 | INITIAL | CC-BY-SA-4.0 |
| `outcome-jm.opt` | `1013.26.932` | `4e947d29-0982-498a-9817-e50baeb757db` | 5 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-adult-patient.opt` | `1013.26.909` | `5b4ea413-6a41-4800-a370-9449803541a0` | 5 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-child-patient.opt` | `1013.26.910` | `7e424539-73c6-44f4-81ed-767ff4a7c644` | 5 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-clinician.opt` | `1013.26.921` | `76a4f42e-7aa1-4e6c-b0ec-515c240c97c9` | 3 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-contact-for-tracing.opt` | `1013.26.951` | `5bf65641-0817-444d-b5eb-6de18f530d3d` | 6 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-father.opt` | `1013.26.903` | `346bf500-c650-4a88-bdaf-ed86bb6e4c93` | 6 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-guardian.opt` | `1013.26.981` | `be854a78-f98e-40a6-a438-d3df1ae05f21` | 3 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-investigating-officer.opt` | `1013.26.920` | `bf9fe81d-36e2-47d2-a5b2-51f640dcc101` | 4 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-nameless-infant-patient.opt` | `1013.26.912` | `e7101374-ece4-4826-a6c5-8c397e97c757` | 4 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-next-of-kin.opt` | `1013.26.956` | `3f7fbe34-4e64-4c02-8d1a-402f7b0f57ce` | 6 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-notifying-individual.opt` | `1013.26.918` | `deb450cd-c10d-4bb0-a3ac-4c77a0f62b06` | 2 | INITIAL | CC-BY-SA-4.0 |
| `person-jm-public-health-official.opt` | `1013.26.922` | `11f5a523-402b-44b3-97d8-dedde63dd301` | 4 | INITIAL | CC-BY-SA-4.0 |
| `pharmacogenetic-test-result.opt` | `1013.26.891` | `b44cfdbd-ba1c-4eae-b45b-f8b022676b53` | 2 | DRAFT | CC-BY-SA-4.0 |
| `phenopacket.opt` | `1013.26.234` | `98ea8bda-2351-446e-9153-90f338080dd8` | 3 | INITIAL | CC-BY-SA-3.0 |
| `phone-jm-home.opt` | `1013.26.905` | `245dc58b-8cd7-4c6c-aeca-3351eaf37a7d` | 2 | INITIAL | CC-BY-SA-4.0 |
| `phone-jm-mobile.opt` | `1013.26.904` | `4d2e5456-20f5-4785-8693-3a5851f78aa0` | 2 | INITIAL | CC-BY-SA-4.0 |
| `phone-jm-unspecified.opt` | `1013.26.902` | `40263253-bdde-429b-8b35-bf1de9a4bdd2` | 2 | INITIAL | CC-BY-SA-4.0 |
| `phone-jm-work.opt` | `1013.26.907` | `1e659eee-7b55-4b21-acc7-27edea409cd9` | 2 | INITIAL | CC-BY-SA-4.0 |
| `pregnancy-screening-jm.opt` | `1013.26.944` | `db7f44eb-ea7b-418f-8e3b-5656b0d98917` | 1 | INITIAL | CC-BY-SA-4.0 |
| `problem-diagnosis-item-r1.opt` | `1013.26.343` | `c1598778-79a1-48f9-9cd0-13d87ad821d0` | 1 | INITIAL | CC-BY-SA-4.0 |
| `problem-diagnosis-screening-jm.opt` | `1013.26.945` | `981e7993-9bdb-4175-bc9b-f9eac7c41dec` | 4 | INITIAL | CC-BY-SA-4.0 |
| `receiving-laboratory-private-jm.opt` | `1013.26.963` | `c2b9a370-361e-4fd1-a631-4acc6130ff42` | 3 | INITIAL | CC-BY-SA-4.0 |
| `receiving-laboratory-public-jm.opt` | `1013.26.964` | `8f9323f8-b005-4d2f-aa02-cd36862579cc` | 2 | INITIAL | CC-BY-SA-4.0 |
| `sex-and-gender-jm-newborn.opt` | `1013.26.914` | `d7c0ddc1-ec81-4334-9588-3704877d5984` | 4 | INITIAL | CC-BY-SA-4.0 |
| `sex-and-gender-jm.opt` | `1013.26.915` | `a50c8112-3ae3-4169-9b6c-8044eb72d41e` | 3 | INITIAL | CC-BY-SA-4.0 |
| `social-network-jm.opt` | `1013.26.957` | `ac7fd6bc-9d60-4401-8f75-ed2d6819b54d` | 6 | INITIAL | CC-BY-SA-4.0 |
| `social-profile-jm.opt` | `1013.26.962` | `a82233b9-7f2d-4dd5-8db4-37f6963cfd8c` | 6 | INITIAL | CC-BY-SA-4.0 |
| `structured-name-jm-core.opt` | `1013.26.917` | `f8ec2c0f-d49f-4604-9257-f89b5c77a0cf` | 2 | INITIAL | CC-BY-SA-4.0 |
| `structured-name-jm-full-maiden.opt` | `1013.26.911` | `2374d734-2197-4a50-8621-290caf6a2f24` | 1 | INITIAL | CC-BY-SA-4.0 |
| `structured-name-jm-full.opt` | `1013.26.899` | `a8371fc1-06a5-4506-8174-2eead36f7746` | 3 | INITIAL | CC-BY-SA-4.0 |
| `suspected-covid-19-assessment-v0-1.opt` | `1013.26.280` | `cafb06d0-f1f6-4eaa-abc7-f64985332c52` | 2 | INITIAL | CC-BY-SA-4.0 |
| `symptom-sign-screening-jm.opt` | `1013.26.933` | `2c968e1c-bfbe-4e38-b173-574e20c70af4` | 2 | INITIAL | CC-BY-SA-4.0 |
| `symptom-sign-screening.opt` | `1013.26.288` | `d7287e5e-1d11-4154-af53-3cdc3ca6e447` | 1 | DRAFT | CC-BY-SA-4.0 |
| `tobacco-smoking-summary-item-r1.opt` | `1013.26.258` | `f790b0ea-0678-46dd-ae90-8d6d2e75cc63` | 2 | INITIAL | CC-BY-SA-4.0 |
| `tobacco-smoking-summary-item-r2.opt` | `1013.26.259` | `8a9aea88-20e5-42fb-8e49-81cbef510617` | 2 | INITIAL | CC-BY-SA-4.0 |
| `travel-event-international-jm.opt` | `1013.26.948` | `fb60795d-3e57-41a3-928a-3b442879c324` | 1 | INITIAL | CC-BY-SA-4.0 |
| `travel-event-jm.opt` | `1013.26.1351` | `6218bbce-3375-4198-b94d-73173b304512` | 2 | INITIAL | CC-BY-SA-4.0 |
| `travel-event-visitor-jm.opt` | `1013.26.947` | `82d15878-43f0-4e47-90f8-80e4f74d8d8a` | 1 | INITIAL | CC-BY-SA-4.0 |
| `travel-profile-jm.opt` | `1013.26.950` | `f64a3b75-d7f5-4fb7-86d2-651cf2446a4b` | 6 | INITIAL | CC-BY-SA-4.0 |
| `travel-screening-jm.opt` | `1013.26.949` | `6ba068a7-4b42-4727-a75d-ac0e5125048d` | 9 | INITIAL | CC-BY-SA-4.0 |
| `treatment-summary-jm.opt` | `1013.26.970` | `b41dfb70-62e2-4382-a7fb-7e894850f8cf` | 5 | INITIAL | CC-BY-SA-4.0 |
| `treponemal-confirmation-test-jm.opt` | `1013.26.1012` | `f94cad4c-e3c1-4524-9f59-b9482a6465b7` | 1 | INITIAL | CC-BY-SA-4.0 |
| `vaccinations-jm.opt` | `1013.26.941` | `2d58a117-a8bd-4cec-a564-0b158f52634f` | 2 | INITIAL | CC-BY-SA-4.0 |
| `white-cell-count-and-differential.opt` | `1013.26.1032` | `2059ae2b-9cdb-40ac-96a3-41bdf591c6fe` | 1 | INITIAL | CC-BY-SA-4.0 |
| `wound-assert-report.opt` | `1013.26.218` | `130d168f-b10d-4608-865d-d419c0039050` | 3 | INITIAL | CC-BY-SA-3.0 |
| `wound-assert.opt` | `1013.26.216` | `11ca34ac-ab9c-44d4-997d-062c0fe1f14e` | 2 | INITIAL | CC-BY-SA-3.0 |
| `wound-assessment-panel.opt` | `1013.26.215` | `3f274bf5-26d2-4c6f-9fc3-7b74ddf0d6b7` | 1 | INITIAL | CC-BY-SA-3.0 |
| `wound-presence-assertion.opt` | `1013.26.214` | `1a6c4792-295f-4c33-9b48-563718f3af66` | 1 | INITIAL | CC-BY-SA-3.0 |
| `wound-related-observations-panel.opt` | `1013.26.217` | `3515bdab-271d-4caa-bd0b-f5bde1952f73` | 2 | INITIAL | CC-BY-SA-3.0 |

## Withheld: the export states no licence

CKM lists these and returns them, and they carry no licence statement,
so this repository ships none of them. The fetch script writes them to
`unstated/` for local use.

| cid | file | status |
|---|---|---|
| `1013.26.975` | `2024-master-infectious-disease-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.1219` | `2025-master-infectious-disease-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.316` | `absolute-cardiovascular-risk-assessment-item-r2.opt` | INITIAL |
| `1013.26.988` | `accidental-poisoning-case-investigation-form.opt` | INITIAL |
| `1013.26.142` | `acquisition-and-validation-of-vf-test.opt` | INITIAL |
| `1013.26.1005` | `additional-notes-jm.opt` | INITIAL |
| `1013.26.901` | `address-jm-international.opt` | INITIAL |
| `1013.26.908` | `address-jm-local-gis.opt` | INITIAL |
| `1013.26.900` | `address-jm-local.opt` | INITIAL |
| `1013.26.147` | `administration-of-antiglaucoma-drugs.opt` | INITIAL |
| `1013.26.359` | `adverse-reaction-risk-item-r1.opt` | INITIAL |
| `1013.26.898` | `age-assertion-jm-infant.opt` | INITIAL |
| `1013.26.897` | `age-assertion-jm.opt` | INITIAL |
| `1013.26.319` | `alcohol-use-disorders-identification-test-audit-item-r2.opt` | INITIAL |
| `1013.26.125` | `amd-assessment.opt` | INITIAL |
| `1013.26.113` | `amd-treatment.opt` | INITIAL |
| `1013.26.321` | `blood-pressure-item-r2.opt` | INITIAL |
| `1013.26.322` | `body-mass-index-item-r2.opt` | INITIAL |
| `1013.26.323` | `body-temperature-item-r2.opt` | INITIAL |
| `1013.26.324` | `body-weight-item-r2.opt` | INITIAL |
| `1013.26.199` | `british-columbia-cancer-agency-breast-cancer-synoptic-report.opt` | INITIAL |
| `1013.26.389` | `cause-of-death.opt` | DRAFT |
| `1013.26.386` | `ccta-report.opt` | INITIAL |
| `1013.26.146` | `cg-assessment.opt` | INITIAL |
| `1013.26.1336` | `cholera-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.1240` | `class-1-notification-form-jm.opt` | INITIAL |
| `1013.26.101` | `classification-and-treatment-of-amd.opt` | INITIAL |
| `1013.26.171` | `classification-and-treatment-of-dr-and-dme.opt` | INITIAL |
| `1013.26.132` | `classification-and-treatment-of-glaucoma.opt` | INITIAL |
| `1013.26.116` | `clinical-image-acquisition-and-validation-nmr.opt` | INITIAL |
| `1013.26.117` | `clinical-image-acquisition-and-validation-oct.opt` | INITIAL |
| `1013.26.356` | `clinical-synopsis-item-r1.opt` | INITIAL |
| `1013.26.977` | `congenital-syphilis-case-investigation-form.opt` | INITIAL |
| `1013.26.71` | `consultation-for-amd-assessment.opt` | INITIAL |
| `1013.26.76` | `consultation-for-patient-admission-in-dr-screening.opt` | INITIAL |
| `1013.26.952` | `contact-for-tracing-jm.opt` | INITIAL |
| `1013.26.291` | `covid-19-pneumonia-diagnosis-and-treatment-7th-edition.opt` | DRAFT |
| `1013.26.1` | `demo-with-hide-on-form.opt` | DRAFT |
| `1013.26.22` | `diagnostic-test-planning.opt` | INITIAL |
| `1013.26.119` | `diagnostic-tests-for-monitoring-the-treatment-of-wet-amd.opt` | INITIAL |
| `1013.26.165` | `diagnostic-tests-in-the-dr-screening-service.opt` | INITIAL |
| `1013.26.144` | `diagnostic-tests-in-the-follow-up-of-cg.opt` | INITIAL |
| `1013.26.980` | `diphtheria-case-investigation-form.opt` | INITIAL |
| `1013.26.172` | `dr-assessment.opt` | INITIAL |
| `1013.26.170` | `dr-screening.opt` | INITIAL |
| `1013.26.176` | `dr-treatment.opt` | INITIAL |
| `1013.26.16` | `ear-primary-hip-arthroplasty-report.opt` | DRAFT |
| `1013.26.17` | `ear-revision-hip-arthroplasty-report.opt` | DRAFT |
| `1013.26.1166` | `echocardiography-findings-jm.opt` | INITIAL |
| `1013.26.399` | `ehr-organisation-with-contact-person.opt` | DRAFT |
| `1013.26.398` | `ehr-person-with-structured-name-and-address.opt` | DRAFT |
| `1013.26.402` | `ehr-person-with-structured-name-and-address2.opt` | INITIAL |
| `1013.26.396` | `ehr-person-with-unstructured-name-and-address.opt` | DRAFT |
| `1013.26.906` | `email-jm.opt` | INITIAL |
| `1013.26.420` | `embryo-storage.opt` | DRAFT |
| `1013.26.94` | `eprescription-epsos-contsys.opt` | DRAFT |
| `1013.26.80` | `eprescription-fhir.opt` | DRAFT |
| `1013.26.12` | `epsos-active-problems-section-1-3-6-1-4-1-19376-1-5-3-1-3-6.opt` | DRAFT |
| `1013.26.8` | `epsos-allergy-and-other-adverse-reactions-section-1-3-6-1-4-1-19376-1-5-3-1-3-13.opt` | DRAFT |
| `1013.26.9` | `epsos-history-of-past-illness-section-1-3-6-1-4-1-19376-1-5-3-1-3-8.opt` | DRAFT |
| `1013.26.6` | `epsos-medication-summary-section-1-3-6-1-4-1-12559-11-10-1-3-1-2-3.opt` | DRAFT |
| `1013.26.13` | `epsos-vital-signs-observations-1-3-6-1-4-1-19376-1-5-3-1-1-5-3-2.opt` | DRAFT |
| `1013.26.2` | `ereferral.opt` | DRAFT |
| `1013.26.390` | `ett.opt` | INITIAL |
| `1013.26.124` | `evaluation-of-the-remote-assessment-model-for-wet-amd.opt` | INITIAL |
| `1013.26.62` | `examination-archetypes.opt` | DRAFT |
| `1013.26.326` | `examination-of-an-ear-item-r2.opt` | INITIAL |
| `1013.26.327` | `examination-of-an-eye-item-r2.opt` | INITIAL |
| `1013.26.328` | `examination-of-both-eyes-item-r2.opt` | INITIAL |
| `1013.26.330` | `examination-of-the-heart-item-r2.opt` | INITIAL |
| `1013.26.329` | `examination-of-the-mouth-item-r2.opt` | INITIAL |
| `1013.26.1047` | `final-classification-jm.opt` | INITIAL |
| `1013.26.426` | `folliculometry.opt` | DRAFT |
| `1013.26.369` | `follow-up-activity-item-r1.opt` | INITIAL |
| `1013.26.134` | `follow-up-schedule-for-cg.opt` | INITIAL |
| `1013.26.79` | `follow-up-schedule-for-patients-with-suspected-dr.opt` | INITIAL |
| `1013.26.114` | `follow-up-schedule-for-the-treatment-of-wet-amd.opt` | INITIAL |
| `1013.26.391` | `follow-up.opt` | INITIAL |
| `1013.26.314` | `food-and-nutrition-summary-item-r2.opt` | INITIAL |
| `1013.26.334` | `gambling-summary-item-r2.opt` | INITIAL |
| `1013.26.372` | `gecco-core.opt` | DRAFT |
| `1013.26.336` | `gender-item-r2.opt` | INITIAL |
| `1013.26.408` | `generic-lab-test-result-example-simple.opt` | DRAFT |
| `1013.26.943` | `gestation-assertion-jm.opt` | INITIAL |
| `1013.26.942` | `gestation-assertion.opt` | INITIAL |
| `1013.26.383` | `glucose-test-result-example.opt` | DRAFT |
| `1013.26.335` | `goal-item-r2.opt` | INITIAL |
| `1013.26.191` | `gp-data-set.opt` | INITIAL |
| `1013.26.1101` | `gram-stain-findings.opt` | INITIAL |
| `1013.26.1235` | `hansen-s-disease-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.56` | `health-risk-assessment.opt` | DRAFT |
| `1013.26.353` | `height-length-item-r2.opt` | INITIAL |
| `1013.26.1238` | `hepatitis-b-c-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.169` | `identification-of-patients-with-dr.opt` | INITIAL |
| `1013.26.123` | `image-test-analysis-for-amd.opt` | INITIAL |
| `1013.26.145` | `image-test-analysis-for-cg.opt` | INITIAL |
| `1013.26.167` | `image-test-analysis-for-dr.opt` | INITIAL |
| `1013.26.1118` | `imaging-examination-screening-jm.opt` | INITIAL |
| `1013.26.421` | `imaging.opt` | DRAFT |
| `1013.26.149` | `incisional-glaucoma-surgery.opt` | INITIAL |
| `1013.26.376` | `international-patient-summary.opt` | DRAFT |
| `1013.26.1218` | `intervention-summary-jm.opt` | INITIAL |
| `1013.26.112` | `intraocular-injection.opt` | INITIAL |
| `1013.26.23` | `intraocular-pressure-study-2.opt` | INITIAL |
| `1013.26.141` | `intraocular-pressure-study.opt` | INITIAL |
| `1013.26.924` | `investigator-initial-infectious-disease-investigation-classification-jm.opt` | INITIAL |
| `1013.26.1114` | `laboratory-test-screening-jm.opt` | INITIAL |
| `1013.26.27` | `laser-procedure.opt` | INITIAL |
| `1013.26.148` | `laser-procedures-to-treat-cg.opt` | INITIAL |
| `1013.26.174` | `laser-procedures-to-treat-dr.opt` | INITIAL |
| `1013.26.340` | `living-arrangement-summary-item-r2.opt` | INITIAL |
| `1013.26.1040` | `management-notes-jm.opt` | INITIAL |
| `1013.26.358` | `medicine-item-r1.opt` | INITIAL |
| `1013.26.357` | `medicines-list-item-r1.opt` | INITIAL |
| `1013.26.979` | `meningitis-encephalitis-case-investigation-form.opt` | INITIAL |
| `1013.26.429` | `middle-name-and-nickname.opt` | DRAFT |
| `1013.26.150` | `modify-cg-treatment.opt` | INITIAL |
| `1013.26.180` | `next-step-in-amd-monitoring-service.opt` | INITIAL |
| `1013.26.143` | `next-step-in-cg-follow-up-service.opt` | INITIAL |
| `1013.26.164` | `next-step-in-dr-screening-service.opt` | INITIAL |
| `1013.26.25` | `next-step-planning.opt` | INITIAL |
| `1013.26.118` | `next-step-remote-assessment-or-schedule-additional-diagnostic-tests.opt` | INITIAL |
| `1013.26.65` | `odl-report-empower.opt` | INITIAL |
| `1013.26.67` | `odl-report-issues.opt` | INITIAL |
| `1013.26.61` | `odl-report-vital-signs.opt` | INITIAL |
| `1013.26.419` | `oocyte-and-embryo-assessment.opt` | DRAFT |
| `1013.26.1037` | `ophthalmia-neonatorum-case-investigation-form.opt` | INITIAL |
| `1013.26.1117` | `other-investigation-screening-jm.opt` | INITIAL |
| `1013.26.379` | `pathological-breast-tumour-size.opt` | INITIAL |
| `1013.26.158` | `patient-admission-into-the-dr-screening-service.opt` | INITIAL |
| `1013.26.133` | `patient-admission-into-the-follow-up-service-for-cg.opt` | INITIAL |
| `1013.26.111` | `patient-admission-into-the-long-term-treatment-for-wet-amd.opt` | INITIAL |
| `1013.26.20` | `patient-s-admittance-in-dr-screening-service.opt` | INITIAL |
| `1013.26.131` | `patient-s-background-and-diagnosis-of-glaucoma.opt` | INITIAL |
| `1013.26.21` | `patient-s-background-for-dr-screening.opt` | INITIAL |
| `1013.26.157` | `patient-s-background-leading-to-suspicion-of-dr.opt` | INITIAL |
| `1013.26.99` | `patient-s-background-leading-to-the-diagnosis-of-amd.opt` | INITIAL |
| `1013.26.98` | `patient-s-enrolment-in-anti-vegf-therapy.opt` | INITIAL |
| `1013.26.128` | `patient-s-enrolment-in-cg-follow-up.opt` | INITIAL |
| `1013.26.156` | `patient-s-enrolment-in-dr-screening.opt` | INITIAL |
| `1013.26.424` | `pelvic-ultrasound-simple.opt` | DRAFT |
| `1013.26.1092` | `person-jm-mother.opt` | INITIAL |
| `1013.26.345` | `personal-safety-summary-item-r2.opt` | INITIAL |
| `1013.26.1038` | `pertussis-case-investigation-form.opt` | INITIAL |
| `1013.26.892` | `pgx-cumulative-pharmacogenetic-profile.opt` | DRAFT |
| `1013.26.344` | `physical-activity-summary-item-r2.opt` | INITIAL |
| `1013.26.160` | `planning-the-diagnostic-tests-to-detect-and-classify-dr.opt` | INITIAL |
| `1013.26.138` | `planning-the-diagnostic-tests-to-follow-up-the-progression-of-cg.opt` | INITIAL |
| `1013.26.364` | `pregnancy-detail-a-item-r1.opt` | INITIAL |
| `1013.26.366` | `pregnancy-detail-b-item-r1.opt` | INITIAL |
| `1013.26.371` | `pregnancy-detail-b2-item-r1.opt` | INITIAL |
| `1013.26.367` | `pregnancy-detail-c-item-r1.opt` | INITIAL |
| `1013.26.430` | `pregnancy-ecosystem-testing.opt` | INITIAL |
| `1013.26.360` | `problem-diagnosis-list-item-r1.opt` | INITIAL |
| `1013.26.361` | `procedure-item-r1.opt` | INITIAL |
| `1013.26.302` | `promis-29.opt` | DRAFT |
| `1013.26.346` | `pulse-item-r2.opt` | INITIAL |
| `1013.26.197` | `rcpa-breast-cancer-histopathology-1st-edition-2010.opt` | INITIAL |
| `1013.26.198` | `rcpa-breast-cancer-structured-report-1st-edition-2010.opt` | INITIAL |
| `1013.26.370` | `reason-for-encounter-item-r1.opt` | INITIAL |
| `1013.26.347` | `respiration-item-r2.opt` | INITIAL |
| `1013.26.1036` | `rheumatic-fever-heart-disease-case-investigation-form.opt` | INITIAL |
| `1013.26.377` | `sars-event-notification.opt` | DRAFT |
| `1013.26.189` | `season-s-greetings.opt` | INITIAL |
| `1013.26.33` | `slovenia-res-primary-hip-arthroplasty-report.opt` | REVIEWSUSPENDED |
| `1013.26.34` | `slovenia-res-revision-hip-arthroplasty-report.opt` | REVIEWSUSPENDED |
| `1013.26.363` | `social-summary-item-r1.opt` | INITIAL |
| `1013.26.1097` | `specimen-collection-details-jm.opt` | INITIAL |
| `1013.26.315` | `substance-use-summary-item-r2.opt` | INITIAL |
| `1013.26.28` | `surgical-procedure.opt` | INITIAL |
| `1013.26.175` | `surgical-procedures-to-treat-dr.opt` | INITIAL |
| `1013.26.1230` | `tetanus-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.350` | `transport-access-summary-item-r2.opt` | INITIAL |
| `1013.26.40` | `treat-registry-report.opt` | DRAFT |
| `1013.26.1222` | `typhoid-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.368` | `vaccination-item-r1.opt` | INITIAL |
| `1013.26.362` | `vaccination-list-item-r1.opt` | INITIAL |
| `1013.26.115` | `visual-acuity-study.opt` | INITIAL |
| `1013.26.380` | `vital-signs.opt` | DRAFT |
| `1013.26.351` | `waist-circumference-item-r2.opt` | INITIAL |
| `1013.26.1335` | `yellow-fever-case-investigation-form-jm.opt` | INITIAL |
| `1013.26.1102` | `ziehl-neelson-stain-findings.opt` | INITIAL |

## Unreachable at fetch time

CKM lists these and returned no export for them. They are recorded
rather than silently skipped.

| cid | intended file |
|---|---|
| `1013.26.14` | `heart-failure-clinic-first-visit-summary.opt` |
