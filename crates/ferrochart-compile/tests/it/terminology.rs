// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the terminology enumerators return, against a fixture that states all
//! four tables.
//!
//! The fixture states a term definition in two languages, a term binding on an
//! `at`-code, a term binding on an `ac`-code (which the reader records as a
//! constraint binding) and an enumerated value set, so every enumerator has
//! something to return and each one can be held to exactly what the archetype
//! states.

use ferrochart_compile::adl2;
use ferrochart_compile::model::ids::{ArchetypeId, LanguageTag, LocalCode};
use ferrochart_compile::model::node::ConstraintTemplate;
use ferrochart_compile::model::terminology::Terminology;

const SOURCE: &str = include_str!("../fixtures/ferro_adl2_terminology.v1.0.0.adls");
const ARCHETYPE: &str = "openEHR-EHR-OBSERVATION.ferro_terminology.v1.0.0";

fn template() -> ConstraintTemplate {
    adl2::from_source(SOURCE, &[]).expect("the fixture reads")
}

fn terminology(template: &ConstraintTemplate) -> &Terminology {
    template
        .terminology(&ArchetypeId::new(ARCHETYPE))
        .expect("the fixture states a terminology")
}

fn rubrics(terminology: &Terminology, language: &str) -> Vec<(String, String)> {
    terminology
        .definitions(&LanguageTag::new(language))
        .map(|(code, term)| (code.as_str().to_owned(), term.text().to_owned()))
        .collect()
}

#[test]
fn every_definition_the_fixture_states_comes_back_in_code_order() {
    // The fixture states the codes in tree order and the enumerator returns
    // them in code order, so the walk is deterministic whatever order the
    // archetype states them in (`.claude/rules/reliability.md`).
    let template = template();
    let english = rubrics(terminology(&template), "en");
    assert_eq!(
        english,
        [
            ("ac1".to_owned(), "Posture choice".to_owned()),
            ("at1".to_owned(), "Sitting".to_owned()),
            ("at2".to_owned(), "Standing".to_owned()),
            ("id1".to_owned(), "Ferro terminology observation".to_owned()),
            ("id2".to_owned(), "Tree".to_owned()),
            ("id3".to_owned(), "Posture".to_owned()),
            ("id4".to_owned(), "Posture value".to_owned()),
        ]
    );
}

#[test]
fn each_language_enumerates_only_its_own_rubrics() {
    let template = template();
    let terminology = terminology(&template);
    let dutch: Vec<String> = rubrics(terminology, "nl")
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    assert_eq!(
        dutch,
        [
            "Houdingskeuze",
            "Zittend",
            "Staand",
            "Ferro terminologie observatie",
            "Boom",
            "Houding",
            "Houdingswaarde",
        ]
    );
    let languages: Vec<&str> = terminology.languages().map(LanguageTag::as_str).collect();
    assert_eq!(languages, ["en", "nl"]);
}

#[test]
fn a_language_the_archetype_states_nothing_in_enumerates_nothing() {
    let template = template();
    assert_eq!(
        terminology(&template)
            .definitions(&LanguageTag::new("de"))
            .count(),
        0
    );
}

#[test]
fn a_definition_comes_back_whole_rather_than_as_its_text() {
    let template = template();
    let terminology = terminology(&template);
    let (code, term) = terminology
        .definitions(&LanguageTag::new("en"))
        .find(|(code, _)| code.as_str() == "at1")
        .expect("the fixture defines at1 in English");
    assert_eq!(code.as_str(), "at1");
    assert_eq!(term.text(), "Sitting");
    assert_eq!(term.description(), Some("Seated."));
}

#[test]
fn the_enumerated_definitions_are_the_ones_the_lookup_finds() {
    // The enumerator and the by-code lookup read one table, and this is what
    // says so: every enumerated pair is what `rubric` returns for that code.
    let template = template();
    let terminology = terminology(&template);
    let english = LanguageTag::new("en");
    for (code, term) in terminology.definitions(&english) {
        assert_eq!(terminology.rubric(&english, code), Some(term), "{code}");
    }
}

#[test]
fn only_the_binding_on_a_value_set_code_is_a_constraint_binding() {
    // openEHR AM Release-2.3.0 AOM2.html section 7.3.1 gives
    // `ARCHETYPE_TERMINOLOGY.term_bindings` one inner hash for both kinds of
    // code, "e.g. \"at4\", \"ac13\"", so the prefix says which a row is.
    let template = template();
    let terminology = terminology(&template);
    let bindings: Vec<(&str, &str, &str)> = terminology
        .all_bindings()
        .map(|(code, target)| (code.as_str(), target.terminology().as_str(), target.value()))
        .collect();
    assert_eq!(
        bindings,
        [(
            "at1",
            "ferro_test",
            "terminology://ferro.example/code/sitting"
        )]
    );
    let constraint_bindings: Vec<(&str, &str, &str)> = terminology
        .all_constraint_bindings()
        .map(|(code, target)| (code.as_str(), target.terminology().as_str(), target.value()))
        .collect();
    assert_eq!(
        constraint_bindings,
        [(
            "ac1",
            "ferro_test",
            "terminology://ferro.example/valueset/synthetic-postures"
        )]
    );
}

#[test]
fn a_value_set_keeps_the_member_order_the_archetype_states() {
    // The fixture states the members out of code order, so a sorted result
    // would fail here: openEHR AM Release-2.3.0 AOM2.html section 7.3.2 types
    // the `members` a `VALUE_SET` inherits as a `List<String>`.
    let template = template();
    let sets: Vec<(&str, Vec<&str>)> = terminology(&template)
        .value_sets()
        .map(|(code, members)| {
            (
                code.as_str(),
                members.iter().map(LocalCode::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(sets, [("ac1", vec!["at2", "at1"])]);
}

#[test]
fn the_enumerated_value_set_is_the_one_the_lookup_finds() {
    let template = template();
    let terminology = terminology(&template);
    for (code, members) in terminology.value_sets() {
        assert_eq!(terminology.value_set(code), Some(members), "{code}");
    }
}

#[test]
fn an_empty_terminology_enumerates_nothing_at_all() {
    let empty = Terminology::new();
    assert_eq!(empty.definitions(&LanguageTag::new("en")).count(), 0);
    assert_eq!(empty.all_bindings().count(), 0);
    assert_eq!(empty.all_constraint_bindings().count(), 0);
    assert_eq!(empty.value_sets().count(), 0);
    assert!(empty.is_empty());
}
