// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The value sets that resolve without a server.
//!
//! `docs/architecture.md` section 7 measures 97.6% of coded fields as getting
//! their membership from the template, so this is the default path and a
//! network call is the exception. Nothing in this module opens a socket.
//!
//! Two sources answer here. An archetype-local `at`-code carries its rubric in
//! the template's own terminology, which openEHR AM Release-2.3.0
//! `AOM1.4.html` section 7.3.2 defines as an `ARCHETYPE_TERM` with a mandatory
//! `text` and an optional `description`, per language. A code from openEHR's
//! own support terminology carries its rubric in openEHR TERM Release-3.0.0,
//! which `openehr-term` embeds.
//!
//! The identifier newtypes of the internal constraint model and of the form
//! definition share their names, so the constraint model's are spelled by
//! their full path rather than imported under an alias
//! (`.claude/rules/rust-style.md`).

use ferrochart_compile::model::terminology::Terminology;
use ferrochart_form::ids::{LanguageTag, LocalCode, TerminologyName, local_terminology};
use ferrochart_form::value::Code;

use crate::resolution::{Origin, Resolution, ResolvedCode};

/// The members of the archetype-local value set `code` names, with their
/// rubrics in `language`.
///
/// This is the 70.6% path of `docs/architecture.md` section 7, and it issues
/// no request: the members come from the archetype's own value sets and each
/// rubric from its `ARCHETYPE_TERM` in the requested language. A member the
/// archetype states no rubric for in that language comes back with no display
/// text rather than with a rubric from another language, because a form that
/// silently mixed languages would show a clinician text they did not ask for.
///
/// Returns `None` where the archetype does not enumerate a set under `code`.
#[must_use]
pub fn archetype_value_set(
    terminology: &Terminology,
    code: &LocalCode,
    language: &LanguageTag,
) -> Option<Resolution> {
    let named = ferrochart_compile::model::ids::LocalCode::new(code.as_str());
    let members = terminology.value_set(&named)?;
    let tag = ferrochart_compile::model::ids::LanguageTag::new(language.as_str());
    let codes = members
        .iter()
        .map(|member| {
            let mut resolved = ResolvedCode::new(Code::new(local_terminology(), member.as_str()));
            if let Some(term) = terminology.rubric(&tag, member) {
                resolved.display = Some(term.text().to_owned());
                resolved.description = term.description().map(str::to_owned);
            }
            resolved
        })
        .collect();
    Some(Resolution::complete(
        codes,
        Origin::Template,
        language.clone(),
    ))
}

/// Every language the archetype states a rubric in, in tag order.
///
/// A caller uses this to ask for a language the template actually carries
/// rather than discovering the absence one field at a time.
#[must_use]
pub fn languages(terminology: &Terminology) -> Vec<LanguageTag> {
    terminology
        .languages()
        .map(|language| LanguageTag::new(language.as_str()))
        .collect()
}

/// Every concept in the openEHR support-terminology group `group`, with its
/// rubric in `language`.
///
/// openEHR TERM Release-3.0.0 publishes the groups the Reference Model
/// mandates, `null_flavours` and `setting` among them, and `openehr-term`
/// embeds them, so this issues no request either. Returns `None` where the
/// terminology has no such group.
#[must_use]
pub fn openehr_group(group: &str, language: &LanguageTag) -> Option<Resolution> {
    let bundle = openehr_term::bundle::openehr();
    let concepts = bundle.concepts_in_group(group);
    if concepts.is_empty() {
        return None;
    }
    let terminology = TerminologyName::new(crate::system::OPENEHR);
    let codes = concepts
        .iter()
        .map(|concept| {
            let resolved = ResolvedCode::new(Code::new(terminology.clone(), concept.id.as_str()));
            match bundle.rubric(group, &concept.id, language.as_str()) {
                Some(text) => resolved.with_display(text),
                None => resolved,
            }
        })
        .collect();
    Some(Resolution::complete(
        codes,
        Origin::OpenehrTerminology,
        language.clone(),
    ))
}

/// The display text openEHR's own terminology carries for `code` in
/// `language`.
///
/// An openEHR concept code is unique across the groups, so the lookup does not
/// need to know which group the code belongs to.
#[must_use]
pub fn openehr_display(code: &str, language: &LanguageTag) -> Option<String> {
    openehr_term::bundle::openehr()
        .concept_rubric(code, language.as_str())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{archetype_value_set, languages, openehr_display, openehr_group};
    use ferrochart_compile::model::terminology::{TermDefinition, Terminology};
    use ferrochart_form::ids::{LanguageTag, LocalCode};
    use std::collections::BTreeMap;

    fn model_code(code: &str) -> ferrochart_compile::model::ids::LocalCode {
        ferrochart_compile::model::ids::LocalCode::new(code)
    }

    fn model_language(tag: &str) -> ferrochart_compile::model::ids::LanguageTag {
        ferrochart_compile::model::ids::LanguageTag::new(tag)
    }

    /// Synthetic content invented for this test. No patient data.
    fn archetype() -> Terminology {
        let mut terminology = Terminology::new();
        terminology.insert_value_set(
            model_code("ac0.1"),
            vec![model_code("at0011"), model_code("at0012")],
        );
        terminology.insert_definition(
            model_language("en"),
            model_code("at0011"),
            TermDefinition::new("Fingertip", Some("On a finger".to_owned()), BTreeMap::new()),
        );
        terminology.insert_definition(
            model_language("en"),
            model_code("at0012"),
            TermDefinition::new("Earlobe", None, BTreeMap::new()),
        );
        terminology.insert_definition(
            model_language("nl"),
            model_code("at0011"),
            TermDefinition::new("Vingertop", None, BTreeMap::new()),
        );
        terminology
    }

    #[test]
    fn a_local_value_set_resolves_with_its_rubrics_in_the_requested_language() {
        let resolution = archetype_value_set(
            &archetype(),
            &LocalCode::new("ac0.1"),
            &LanguageTag::new("en"),
        )
        .expect("the archetype enumerates the set");
        let shown: Vec<(&str, Option<&str>)> = resolution
            .codes
            .iter()
            .map(|code| (code.code.code.as_str(), code.display.as_deref()))
            .collect();
        assert_eq!(
            shown,
            [("at0011", Some("Fingertip")), ("at0012", Some("Earlobe"))]
        );
        assert_eq!(
            resolution
                .codes
                .first()
                .and_then(|code| code.description.as_deref()),
            Some("On a finger")
        );
    }

    #[test]
    fn a_member_the_archetype_states_no_rubric_for_shows_no_text_from_another_language() {
        let resolution = archetype_value_set(
            &archetype(),
            &LocalCode::new("ac0.1"),
            &LanguageTag::new("nl"),
        )
        .expect("the archetype enumerates the set");
        let shown: Vec<Option<&str>> = resolution
            .codes
            .iter()
            .map(|code| code.display.as_deref())
            .collect();
        assert_eq!(shown, [Some("Vingertop"), None]);
    }

    #[test]
    fn a_code_the_archetype_does_not_name_a_set_for_resolves_to_nothing() {
        assert!(
            archetype_value_set(
                &archetype(),
                &LocalCode::new("ac9.9"),
                &LanguageTag::new("en")
            )
            .is_none()
        );
    }

    #[test]
    fn the_languages_come_back_in_tag_order() {
        let stated = languages(&archetype());
        let tags: Vec<&str> = stated.iter().map(LanguageTag::as_str).collect();
        assert_eq!(tags, ["en", "nl"]);
    }

    #[test]
    fn an_openehr_group_resolves_from_the_embedded_terminology() {
        // openEHR RM Release-1.1.0 `data_structures.html` section 4.1 draws
        // the null flavours from the openEHR terminology's group of that name,
        // and `openehr-term` embeds it, so no server is involved.
        let resolution = openehr_group("null_flavours", &LanguageTag::new("en"))
            .expect("the RM-mandated group is embedded");
        assert!(!resolution.codes.is_empty());
        assert!(
            resolution
                .codes
                .iter()
                .all(|code| code.code.terminology.as_str() == "openehr")
        );
    }

    #[test]
    fn a_group_the_openehr_terminology_does_not_publish_resolves_to_nothing() {
        assert!(openehr_group("ferro_not_a_group", &LanguageTag::new("en")).is_none());
    }

    #[test]
    fn an_openehr_concept_code_resolves_its_display_text_without_its_group() {
        assert!(openehr_display("433", &LanguageTag::new("en")).is_some());
        assert_eq!(
            openehr_display("ferro-not-a-code", &LanguageTag::new("en")),
            None
        );
    }
}
