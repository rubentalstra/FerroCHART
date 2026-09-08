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
        } => format!(
            "An archetype slot the template leaves open: {} archetypes admitted, {} refused.",
            includes.len(),
            excludes.len()
        ),
        UndeterminedReason::UnconstrainedValue => {
            "An ELEMENT whose value the template constrains not at all, so every data type \
             there is would be a guess."
                .to_owned()
        }
        UndeterminedReason::UntypedInterval => {
            "A DV_INTERVAL whose element type the template never states, so neither end has a \
             control."
                .to_owned()
        }
        _ => "The template left this undetermined for a reason this renderer does not know."
            .to_owned(),
    }
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
        format!("{} is left undetermined.", content.rm_type)
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
    fn an_open_slot_says_how_many_archetypes_it_admits() {
        let reason = UndeterminedReason::OpenSlot {
            includes: vec![ferrochart_form::group::SlotAssertion::ArchetypeIdPattern(
                "a.*".to_owned(),
            )],
            excludes: Vec::new(),
        };
        assert!(why(&reason).contains("1 archetypes admitted"));
    }
}
