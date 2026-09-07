// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One committed template, compiled and ready to commit through the gate.
//!
//! Synthetic content invented for these tests. No patient data: every entry
//! comes from `crate::filler`, which picks a code the template enumerates or a
//! number inside a range the template states.

use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compose::envelope::{CATEGORY_EVENT, Composer, Envelope, Setting, Subject, UTF8};
use ferrochart_compose::values::FormValues;
use ferrochart_form::definition::FormDefinition;
use ferrochart_validate::template::TemplateValidator;

use crate::filler;

/// The committed template these tests commit.
///
/// It is rooted at COMPOSITION, which is what lets a CDR judge the document
/// FerroCHART sends against the template FerroCHART derived the form from.
/// The pack's other 113 templates are rooted at an ENTRY or a SECTION, and
/// what a CDR does with a COMPOSITION built around one of those is a separate
/// question from whether this gate works.
pub(crate) const TEMPLATE: &str = "openehr-suspected-covid-19-assessment-v0.opt";

/// The identifier that template states for itself.
pub(crate) const TEMPLATE_ID: &str = "openEHR-Suspected Covid-19 assessment.v0";

/// The template's canonical XML.
pub(crate) fn template_xml() -> String {
    fs::read_to_string(path()).expect("the committed template reads")
}

/// Where the template lives.
fn path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/templates/ckm")
        .join(TEMPLATE)
}

/// The form, the gate and a full set of entries.
pub(crate) fn case() -> (FormDefinition, TemplateValidator, FormValues) {
    let xml = template_xml();
    let template = ferrochart_compile::adl14::from_xml(&xml).expect("the template reads");
    let definition = ferrochart_compile::derive::form(&template).expect("the template derives");
    let validator = TemplateValidator::from_opt14_xml(&xml).expect("the template flattens");
    let values = filler::fill(&definition);
    (definition, validator, values)
}

/// A session's worth of the values a form never carries.
pub(crate) fn envelope() -> Envelope {
    Envelope {
        language: "en".to_owned(),
        territory: "NL".to_owned(),
        category: CATEGORY_EVENT.to_owned(),
        category_rubric: "event".to_owned(),
        composer: Composer::Identified {
            name: "Ferro test suite".to_owned(),
        },
        subject: Subject::SelfParty,
        encoding: UTF8.to_owned(),
        now: "2026-09-07T12:00:00Z".to_owned(),
        setting: Some(Setting {
            code: "238".to_owned(),
            rubric: "other care".to_owned(),
        }),
        // A template rooted below COMPOSITION is wrapped in this one.
        composition_archetype: Some("openEHR-EHR-COMPOSITION.encounter.v1".to_owned()),
    }
}
