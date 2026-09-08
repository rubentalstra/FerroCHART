// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_BOOLEAN`: a two-way control, or none at all.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.2 constrains a boolean
//! with `true_valid` and `false_valid`, and the two are independent. Where
//! only one is admitted the template has already decided the value and
//! nothing is entered; where neither is, the template admits no boolean at
//! all.

use ferrochart_form::field::BooleanField;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::Refusal;
use crate::control::{RefusalNote, Slot};
use crate::kit::notice::Notice;
use crate::kit::segmented::{Segment, Segmented};
use crate::kit::tone::Tone;

/// The values the template admits, in the order a control offers them.
pub(crate) fn admitted(field: BooleanField) -> Vec<bool> {
    let mut values = Vec::new();
    if field.true_allowed {
        values.push(true);
    }
    if field.false_allowed {
        values.push(false);
    }
    values
}

/// Whether the template already decided the value, so nothing is entered.
pub(crate) fn is_decided(field: BooleanField) -> bool {
    field.true_allowed != field.false_allowed
}

/// The value `chosen` records, where the template admits it.
pub(crate) fn admit(field: BooleanField, chosen: bool) -> Result<Datum, Refusal> {
    match (chosen, field.true_allowed, field.false_allowed) {
        (_, false, false) => Err(Refusal::NothingAdmitted),
        (true, true, _) | (false, _, true) => Ok(Datum::Boolean(chosen)),
        (true, false, _) | (false, _, false) => Err(Refusal::NotEnumerated),
    }
}

/// The word a control shows for one of the two values.
const fn word(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

/// A two-way control over `DV_BOOLEAN.value`.
#[component]
pub(crate) fn BooleanControl(
    /// What the template admits.
    field: BooleanField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let label = slot.label.clone();
    let refused = RwSignal::new(None::<Refusal>);
    let admitted_values = admitted(field);

    if admitted_values.is_empty() {
        return view! {
            <Notice
                tone=Tone::Warn
                title="The template admits no value here."
                detail="Neither true nor false is allowed, so there is nothing to enter."
            />
        }
        .into_any();
    }

    let chosen = {
        let slot = slot.clone();
        Signal::derive(move || match slot.datum() {
            Some(Datum::Boolean(value)) => Some(value.to_string()),
            _ => None,
        })
    };

    let segments = admitted_values
        .iter()
        .map(|value| Segment::new(value.to_string(), word(*value)))
        .collect();

    let decided = is_decided(field);
    let on_choose = {
        let slot = slot.clone();
        Callback::new(move |value: String| {
            slot.apply(admit(field, value == "true"), refused);
        })
    };

    view! {
        <Segmented
            name=slot.id.clone()
            legend=label
            segments=segments
            chosen=chosen
            on_choose=on_choose
        />
        <Show when=move || decided>
            <p class="mt-1 text-xs text-ink-muted">"The template admits only this value."</p>
        </Show>
        <RefusalNote refused=refused />
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::BooleanField;
    use ferrochart_form::values::Datum;

    use super::{admit, admitted, is_decided};
    use crate::control::admit::Refusal;

    /// A boolean constraint stated as the two flags.
    const fn field(true_allowed: bool, false_allowed: bool) -> BooleanField {
        BooleanField {
            true_allowed,
            false_allowed,
        }
    }

    #[test]
    fn both_flags_set_admits_both_values() {
        assert_eq!(admitted(field(true, true)), [true, false]);
        assert_eq!(admit(field(true, true), true), Ok(Datum::Boolean(true)));
        assert_eq!(admit(field(true, true), false), Ok(Datum::Boolean(false)));
        assert!(!is_decided(field(true, true)));
    }

    #[test]
    fn one_flag_set_refuses_the_other_value() {
        assert_eq!(admitted(field(true, false)), [true]);
        assert_eq!(admit(field(true, false), true), Ok(Datum::Boolean(true)));
        assert_eq!(
            admit(field(true, false), false),
            Err(Refusal::NotEnumerated)
        );
        assert_eq!(admit(field(false, true), true), Err(Refusal::NotEnumerated));
        assert!(is_decided(field(true, false)));
        assert!(is_decided(field(false, true)));
    }

    #[test]
    fn neither_flag_set_admits_nothing_at_all() {
        assert!(admitted(field(false, false)).is_empty());
        assert_eq!(
            admit(field(false, false), true),
            Err(Refusal::NothingAdmitted)
        );
        assert_eq!(
            admit(field(false, false), false),
            Err(Refusal::NothingAdmitted)
        );
    }
}
