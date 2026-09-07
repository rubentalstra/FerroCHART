// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The committed template pack, compiled and built, ready to judge.
//!
//! Synthetic content invented for these tests. No patient data.

use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compose::build;
use ferrochart_compose::envelope::{CATEGORY_EVENT, Composer, Envelope, Setting, Subject, UTF8};
use ferrochart_form::definition::FormDefinition;
use ferrochart_validate::template::TemplateValidator;
use openehr_rm::v1_2::composition::composition::Composition;

use crate::filler;

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
    }
}

/// Every committed operational template, in a stable order.
pub(crate) fn templates() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/templates/ckm");
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "opt"))
        .collect();
    paths.sort();
    paths
}

/// One template, compiled, filled and built.
pub(crate) struct Case {
    /// The form the template derives to.
    pub(crate) definition: FormDefinition,
    /// The document the filler's entries build.
    pub(crate) composition: Composition,
    /// The gate for that template.
    pub(crate) validator: TemplateValidator,
}

/// The case `path` describes, where the template compiles and builds a
/// document.
///
/// A template that describes a fragment rather than a document builds no
/// COMPOSITION at all (`ferrochart_compose::error::BuildError`), and the
/// composition tests already pin how many do, so those are skipped here rather
/// than counted twice.
pub(crate) fn case(path: &Path) -> Option<Case> {
    let xml = fs::read_to_string(path).ok()?;
    let template = ferrochart_compile::adl14::from_xml(&xml).ok()?;
    let definition = ferrochart_compile::derive::form(&template).ok()?;
    let composition =
        build::composition(&definition, &filler::fill(&definition), &envelope()).ok()?;
    let validator = TemplateValidator::from_opt14_xml(&xml).ok()?;
    Some(Case {
        definition,
        composition,
        validator,
    })
}

/// Every case the committed pack yields.
pub(crate) fn cases() -> Vec<(PathBuf, Case)> {
    templates()
        .into_iter()
        .filter_map(|path| {
            let built = case(&path)?;
            Some((path, built))
        })
        .collect()
}
