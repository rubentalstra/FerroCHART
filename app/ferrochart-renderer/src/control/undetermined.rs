// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Content the operational template did not determine.
//!
//! It captures nothing and it is never skipped. The whole point of recording
//! it is that a person can see the template left a hole rather than that the
//! compiler lost one, so it is drawn where the content would have been.

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
    let label = crate::label::of(&content.label, &language, &content.key);
    let title = if label.is_empty() {
        format!(
            "The template leaves {} here undetermined.",
            crate::plain::describe(content.rm_type.as_str())
        )
    } else {
        format!("{label} is left undetermined.")
    };
    let path = content.key.to_string();

    view! {
        <Notice tone=Tone::Warn title=title detail=why(&content.reason)>
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
}
