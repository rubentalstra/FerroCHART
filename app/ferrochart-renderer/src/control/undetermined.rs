// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Content the operational template did not determine, and content it left
//! room for.
//!
//! Neither captures anything and neither is ever skipped, because the point of
//! recording one is that a person can see the template left it rather than
//! that the compiler lost it. The two read differently, though: an open
//! archetype slot is a place the template meant to leave open, and a value it
//! never typed is a hole (issue #180).

use ferrochart_form::group::{UndeterminedContent, UndeterminedReason};
use ferrochart_form::ids::LanguageTag;
use leptos::prelude::*;

use crate::kit::notice::Notice;
use crate::kit::surface::CODE;
use crate::kit::tone::Tone;

/// Why the template left the hole, in one line a person reads.
pub(crate) fn why(reason: &UndeterminedReason) -> String {
    match *reason {
        UndeterminedReason::OpenSlot {
            ref includes,
            ref excludes,
        } => open_slot(includes.len(), excludes.len()),
        UndeterminedReason::UnconstrainedValue => {
            "The template asks for a value here and never says what kind of value, so there is \
             no control to draw."
                .to_owned()
        }
        UndeterminedReason::UntypedInterval => {
            "The template asks for a range here and never says what the two ends measure, so \
             there is no control to draw."
                .to_owned()
        }
        _ => "The template left this undetermined for a reason this renderer does not know."
            .to_owned(),
    }
}

/// The one line an open slot gets, counting what the template admits.
fn open_slot(admitted: usize, refused: usize) -> String {
    let admits = if admitted == 0 {
        "The template leaves room here for extra content and never says what fits.".to_owned()
    } else {
        format!(
            "The template leaves room here for extra content, and names {admitted} {} it \
             accepts.",
            plural(admitted, "kind", "kinds")
        )
    };
    if refused == 0 {
        admits
    } else {
        format!(
            "{admits} It rules out {refused} {}.",
            plural(refused, "kind", "kinds")
        )
    }
}

/// The singular or the plural of a word, by a count.
fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// Whether the reason is a place to add content rather than a hole.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.8 gives `ARCHETYPE_SLOT`
/// the attribute `is_closed`, "closed to further filling either in further
/// specialisations or at runtime", and defaults it to false. So an open slot
/// is a runtime extension point the template meant to leave open, and drawing
/// it as a warning said the template had gone wrong (issue #180).
const fn is_a_place_to_add(reason: &UndeterminedReason) -> bool {
    matches!(*reason, UndeterminedReason::OpenSlot { .. })
}

/// The hole the template left, drawn where the content would have been.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn UndeterminedView(
    /// The content the template did not determine.
    content: UndeterminedContent,
    /// The language a label is shown in.
    language: LanguageTag,
) -> impl IntoView {
    let label = crate::label::of(
        &content.label,
        &language,
        &content.key,
        &crate::plain::describe(content.rm_type.as_str()),
    );
    let extendable = is_a_place_to_add(&content.reason);
    let title = if extendable {
        format!("{label} takes extra content.")
    } else {
        format!("{label} is left undetermined.")
    };
    let tone = if extendable {
        Tone::Neutral
    } else {
        Tone::Warn
    };
    let path = content.key.to_string();

    view! {
        <Notice tone=tone title=title detail=why(&content.reason)>
            <p class=CODE>{path}</p>
        </Notice>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::group::UndeterminedReason;

    use super::why;

    #[test]
    fn every_reason_says_something_a_person_can_act_on() {
        let reasons = [
            UndeterminedReason::OpenSlot {
                includes: vec![],
                excludes: vec![],
            },
            UndeterminedReason::UnconstrainedValue,
            UndeterminedReason::UntypedInterval,
        ];
        for reason in &reasons {
            let text = why(reason);
            assert!(!text.is_empty(), "{reason:?}");
            assert!(text.ends_with('.'), "{text}");
        }
    }

    #[test]
    fn an_open_slot_says_how_many_kinds_of_content_it_admits() {
        let reason = UndeterminedReason::OpenSlot {
            includes: vec![ferrochart_form::group::SlotAssertion::ArchetypeIdPattern(
                "a.*".to_owned(),
            )],
            excludes: Vec::new(),
        };
        assert!(
            why(&reason).contains("names 1 kind it accepts"),
            "{}",
            why(&reason)
        );
    }
    #[test]
    fn no_reason_names_a_reference_model_class() {
        // The audience came to build a form, so the screen never spells a
        // Reference Model class name at them.
        let reasons = [
            UndeterminedReason::OpenSlot {
                includes: vec![],
                excludes: vec![],
            },
            UndeterminedReason::UnconstrainedValue,
            UndeterminedReason::UntypedInterval,
        ];
        for reason in &reasons {
            let text = why(reason);
            assert!(
                !text.contains("DV_") && !text.contains("ELEMENT") && !text.contains("CLUSTER"),
                "{text}"
            );
        }
    }

    #[test]
    fn the_counts_read_as_english() {
        assert!(super::open_slot(1, 1).contains("names 1 kind it accepts"));
        assert!(super::open_slot(1, 1).contains("rules out 1 kind."));
        assert!(super::open_slot(3, 2).contains("names 3 kinds it accepts"));
        assert!(super::open_slot(3, 2).contains("rules out 2 kinds."));
        assert!(super::open_slot(0, 0).contains("never says what fits"));
    }
    #[test]
    fn an_open_slot_is_a_place_to_add_and_not_a_hole() {
        // openEHR AM Release-2.3.0 `AOM2.html` section 4.5.8 defaults
        // `is_closed` to false, so a slot that survived into the template is
        // one it meant to leave open (#180).
        assert!(super::is_a_place_to_add(&UndeterminedReason::OpenSlot {
            includes: vec![],
            excludes: vec![],
        }));
    }

    #[test]
    fn a_value_the_template_never_typed_is_still_a_hole() {
        assert!(!super::is_a_place_to_add(
            &UndeterminedReason::UnconstrainedValue
        ));
        assert!(!super::is_a_place_to_add(
            &UndeterminedReason::UntypedInterval
        ));
    }
}
