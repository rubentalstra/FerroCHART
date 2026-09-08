// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The builder over the whole committed template pack.
//!
//! Synthetic content invented for these tests. No patient data: every value
//! comes from `crate::filler`, which picks a code the template enumerates or a
//! number inside a range the template states.
//!
//! All citations are openEHR RM Release-1.1.0.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compose::build;
use ferrochart_compose::error::BuildError;
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::envelope::{CATEGORY_EVENT, Composer, Envelope, Setting, Subject, UTF8};
use ferrochart_form::group::{FormGroup, FormItem};

use crate::filler;

/// Every committed operational template, in a stable order.
fn templates() -> Vec<PathBuf> {
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

/// The form every committed template derives to.
pub(crate) fn forms() -> Vec<(PathBuf, FormDefinition)> {
    templates()
        .into_iter()
        .filter_map(|path| {
            let xml = fs::read_to_string(&path).ok()?;
            let template = ferrochart_compile::adl14::from_xml(&xml).ok()?;
            let form = ferrochart_compile::derive::form(&template).ok()?;
            Some((path, form))
        })
        .collect()
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

#[test]
fn every_template_that_describes_a_document_builds_one() {
    // A ratchet over the committed pack. The two numbers are the finding:
    // what builds, and what is refused because a template rooted at a
    // fragment class cannot become a document at all.
    let mut built = 0;
    let mut refused: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (_, form) in forms() {
        match build::composition(&form, &filler::fill(&form), &envelope()) {
            Ok(_) => built += 1,
            Err(BuildError::Invariant { invariant, .. }) => {
                *refused.entry(invariant).or_default() += 1;
            }
            Err(other) => panic!("an unexpected refusal: {other:?}"),
        }
    }
    assert_eq!(built, 78, "the templates that build a COMPOSITION");
    assert_eq!(
        refused.get("COMPOSITION.content").copied(),
        Some(36),
        "a CLUSTER-rooted template describes a fragment, not a document, so \
         it is refused by name rather than wrapped in an invented entry"
    );
    assert_eq!(
        refused.get("INTERVAL_EVENT.width").copied(),
        Some(4),
        "an INTERVAL_EVENT needs a width and a math function no form supplies"
    );
    assert_eq!(
        refused.get("EVENT.data").copied(),
        Some(3),
        "an event whose data carries nothing the filler can enter"
    );
}

#[test]
fn the_pack_still_derives_the_repeats_the_addressing_exists_for() {
    // A floor, not a target. The occurrence path costs nothing on a template
    // with no repeating group, so a derivation that stopped seeing repeats
    // would leave every test above green while the case this addressing
    // exists for went untested. Deriving more repeats is not a failure.
    let mut with_repeat = 0_usize;
    let mut under_repeat = 0_usize;
    for (_, form) in forms() {
        if form
            .root
            .walk_groups()
            .any(|group| group.occurrences.is_repeatable())
        {
            with_repeat = with_repeat.saturating_add(1);
        }
        count_repeats(&form.root, false, &mut under_repeat);
    }
    assert!(
        with_repeat >= 67,
        "only {with_repeat} templates of the pack carry a repeatable group, \
         which is fewer than the derivation used to find"
    );
    assert!(
        under_repeat >= 1868,
        "only {under_repeat} fields of the pack sit under a repeat, which is \
         fewer than the derivation used to find"
    );
}

/// Counts the fields of a subtree that sit under at least one repeating group.
fn count_repeats(group: &FormGroup, under: bool, fields: &mut usize) {
    for item in &group.items {
        match *item {
            FormItem::Field(_) if under => *fields = fields.saturating_add(1),
            FormItem::Group(ref child) => {
                count_repeats(child, under || child.occurrences.is_repeatable(), fields);
            }
            _ => {}
        }
    }
}

#[test]
fn every_built_composition_carries_what_the_reference_model_requires() {
    // The attributes the Reference Model declares 1..1 and a form never
    // shows. A CDR refusing a COMPOSITION FerroCHART built is always our bug,
    // so each of these is asserted rather than assumed.
    for (path, form) in forms() {
        let Ok(composition) = build::composition(&form, &filler::fill(&form), &envelope()) else {
            continue;
        };
        let json = serde_json::to_value(&composition).expect("a composition serialises");
        let name = path.display();

        assert_eq!(json["_type"], "COMPOSITION", "{name}");
        assert!(
            !json["archetype_node_id"].as_str().unwrap_or("").is_empty(),
            "{name}"
        );
        assert_eq!(json["language"]["code_string"], "en", "{name}");
        assert_eq!(json["territory"]["code_string"], "NL", "{name}");
        assert_eq!(
            json["category"]["defining_code"]["code_string"], "433",
            "{name}"
        );
        assert_eq!(
            json["category"]["defining_code"]["terminology_id"]["value"], "openehr",
            "{name}"
        );
        assert!(json["composer"].is_object(), "{name}");

        // `Archetyped_valid: is_archetype_root xor archetype_details = Void`,
        // and `Rm_version_valid: not rm_version.is_empty`.
        assert_eq!(json["archetype_details"]["rm_version"], "1.1.0", "{name}");
        assert!(
            !json["archetype_details"]["template_id"]["value"]
                .as_str()
                .unwrap_or("")
                .is_empty(),
            "{name}"
        );

        // `LOCATABLE.uid` is 0..1 and the client cannot know the version
        // identity before the commit, so it is never written.
        assert!(
            json.get("uid").is_none(),
            "{name} carries a uid it invented"
        );

        // `Content_valid: content /= Void implies not content.is_empty`.
        if let Some(content) = json.get("content") {
            assert!(
                !content.as_array().is_some_and(Vec::is_empty),
                "{name} carries an empty content list, which the invariant forbids"
            );
        }
    }
}

#[test]
fn a_null_flavour_outside_the_openehr_group_is_refused() {
    // `Inv_null_flavour_valid` tests membership of the openEHR `null
    // flavours` group, which has four members. A code outside it would build
    // a document the CDR refuses.
    use ferrochart_form::ids::LocalCode;
    use ferrochart_form::values::Entered;

    // A form the builder otherwise accepts, so the refusal under test is the
    // null flavour rather than an empty entry higher up.
    let Some((_, form)) = forms().into_iter().find(|(_, form)| {
        build::composition(form, &filler::fill(form), &envelope()).is_ok()
            && form.fields().count() > 1
    }) else {
        return;
    };
    // The slot the filler already entered, so the null flavour lands where the
    // builder walks: a field under a repeating group is addressed by that
    // group's occurrence as well as by its key.
    let mut values = filler::fill(&form);
    let (key, group_path, occurrence) = {
        let (slot, _) = values.iter().next().expect("the filler entered a value");
        (slot.key.clone(), slot.group_path.clone(), slot.occurrence)
    };
    values.set_in(
        key,
        group_path,
        occurrence,
        Entered::Null {
            code: LocalCode::new("999"),
            reason: None,
        },
    );

    match build::composition(&form, &values, &envelope()) {
        Err(BuildError::UnknownNullFlavour { code }) => assert_eq!(code, "999"),
        Ok(_) => panic!("a null flavour outside the group was accepted"),
        Err(other) => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_category_outside_the_openehr_group_is_refused() {
    // The group has four members in TERM Release-3.0.0, and `815|report|` is
    // the one the Reference Model prose leaves out. A list of three would
    // refuse a document the specification permits, so the test pins both
    // directions.
    let Some((_, form)) = forms()
        .into_iter()
        .find(|(_, form)| build::composition(form, &filler::fill(form), &envelope()).is_ok())
    else {
        return;
    };

    let mut report = envelope();
    report.category = "815".to_owned();
    report.category_rubric = "report".to_owned();
    let outcome = build::composition(&form, &filler::fill(&form), &report);
    assert!(
        !matches!(outcome, Err(BuildError::UnknownCategory { .. })),
        "815|report| is in the openEHR composition category group"
    );

    let mut invented = envelope();
    invented.category = "999".to_owned();
    match build::composition(&form, &filler::fill(&form), &invented) {
        Err(BuildError::UnknownCategory { code }) => assert_eq!(code, "999"),
        Ok(_) => panic!("a category outside the group was accepted"),
        Err(other) => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_wrapped_entry_names_the_composition_archetype_and_not_its_own() {
    // 113 of the 123 committed templates root below COMPOSITION, so
    // FerroCHART supplies the document around them. `ehr.html` section 5.4.1
    // makes `Is_archetype_root` an invariant of COMPOSITION and
    // `common.html` section 3.2.2 requires a root's `archetype_node_id` to be
    // the stringified archetype id, so writing the entry's archetype there
    // would claim a COMPOSITION is an OBSERVATION.
    let Some((path, form)) = forms()
        .into_iter()
        .find(|(_, form)| form.root.rm_type.as_str() != "COMPOSITION")
    else {
        panic!("the pack has no template rooted below COMPOSITION");
    };
    let Ok(composition) = build::composition(&form, &filler::fill(&form), &envelope()) else {
        return;
    };
    let json = serde_json::to_value(&composition).expect("a composition serialises");
    let wrapper = "openEHR-EHR-COMPOSITION.encounter.v1";
    assert_eq!(json["archetype_node_id"], wrapper, "{}", path.display());
    assert_eq!(
        json["archetype_details"]["archetype_id"]["value"],
        wrapper,
        "{}",
        path.display()
    );
    // The entry keeps its own identity one level down.
    assert_eq!(
        json["content"][0]["archetype_node_id"],
        form.root
            .archetype_id
            .as_ref()
            .map(ferrochart_form::ids::ArchetypeId::as_str)
            .unwrap_or_default()
    );
}

#[test]
fn a_wrapped_entry_with_no_composition_archetype_is_refused() {
    // Inventing one would put a clinical classification in a document nobody
    // chose, so the build refuses and names what the configuration is missing.
    let Some((_, form)) = forms()
        .into_iter()
        .find(|(_, form)| form.root.rm_type.as_str() != "COMPOSITION")
    else {
        return;
    };
    let mut unconfigured = envelope();
    unconfigured.composition_archetype = None;
    match build::composition(&form, &filler::fill(&form), &unconfigured) {
        Err(BuildError::Invariant { invariant, .. }) => {
            assert_eq!(invariant, "COMPOSITION.Is_archetype_root");
        }
        Ok(_) => panic!("a composition was built with no archetype of its own"),
        Err(other) => panic!("the refusal is {other:?}"),
    }
}
