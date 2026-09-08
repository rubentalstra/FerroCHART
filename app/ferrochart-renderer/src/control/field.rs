// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One field of a form: its label, its repeats, its control, and the null
//! flavour beside it.
//!
//! The dispatch from a [`FieldKind`] to a control happens in exactly one
//! place, [`control`], so a new variant is a compile error here rather than a
//! field that silently draws nothing.

use ferrochart_form::field::{FieldKind, FormField, ReferenceRanges};
use ferrochart_form::ids::{LanguageTag, RmTypeName};
use leptos::prelude::*;

use crate::control::boolean::BooleanControl;
use crate::control::choice::ChoiceControl;
use crate::control::coded::CodedControl;
use crate::control::count::CountControl;
use crate::control::duration::DurationControl;
use crate::control::identifier::IdentifierControl;
use crate::control::interval::IntervalControl;
use crate::control::multimedia::MultimediaControl;
use crate::control::null_flavour::NullFlavourControl;
use crate::control::ordinal::OrdinalControl;
use crate::control::parsable::ParsableControl;
use crate::control::proportion::ProportionControl;
use crate::control::quantity::QuantityControl;
use crate::control::repeats::Repeats;
use crate::control::state_machine::StateControl;
use crate::control::temporal::{
    DATE_PARTS, DATE_TIME_PARTS, TIME_PARTS, TemporalControl, date_depth, date_time_depth,
    native_date, native_date_time, native_time, time_depth,
};
use crate::control::text::TextControl;
use crate::control::uri::UriControl;
use crate::control::{Address, Slot, localized, prefill};
use crate::kit::badge::{BADGE, StatusPill};
use crate::kit::field::{HINT, LABEL};
use crate::kit::notice::Notice;
use crate::kit::surface::WELL;
use crate::kit::tone::Tone;
use crate::state::{FormState, Path};

/// The control the field's kind calls for.
///
/// One `match` over every variant. It returns an erased view rather than an
/// opaque one, because an interval draws its own ends with this function and
/// an opaque return type would be its own argument.
pub(crate) fn control(kind: &FieldKind, rm_type: &RmTypeName, slot: &Slot) -> AnyView {
    let slot = slot.clone();
    match *kind {
        FieldKind::Boolean(field) => view! { <BooleanControl field=field at=slot /> }.into_any(),
        FieldKind::Text(ref field) => {
            view! { <TextControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Uri(ref field) => {
            view! { <UriControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Coded(ref field) => {
            view! { <CodedControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Ordinal(ref field) => {
            view! { <OrdinalControl field=field.clone() rm_type=rm_type.clone() at=slot /> }
                .into_any()
        }
        FieldKind::Count(ref field) => {
            view! { <CountControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Quantity(ref field) => {
            view! { <QuantityControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Proportion(ref field) => {
            view! { <ProportionControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Date(_) | FieldKind::Time(_) | FieldKind::DateTime(_) => temporal(kind, slot),
        FieldKind::Duration(ref field) => {
            view! { <DurationControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Identifier(ref field) => {
            view! { <IdentifierControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Multimedia(ref field) => {
            view! { <MultimediaControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Parsable(ref field) => {
            view! { <ParsableControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Interval(ref field) => {
            view! { <IntervalControl field=(**field).clone() at=slot /> }.into_any()
        }
        FieldKind::State(ref field) => {
            view! { <StateControl field=field.clone() at=slot /> }.into_any()
        }
        FieldKind::Choice(ref field) => {
            view! { <ChoiceControl field=field.clone() at=slot /> }.into_any()
        }
        // `FieldKind` is `#[non_exhaustive]`, so the compiler asks for this
        // arm. A kind this renderer does not know is drawn as a hole rather
        // than skipped, for the reason undetermined content is.
        _ => view! {
            <Notice
                tone=Tone::Warn
                title="This renderer does not know what this field collects."
                detail=format!("The form definition names the kind `{}`.", kind.name())
            />
        }
        .into_any(),
    }
}

/// The control a date, a time, or a date and time calls for.
///
/// The three share one control, because a partial value of any of them is the
/// same question: how far to the right the template lets the value run.
fn temporal(kind: &FieldKind, slot: Slot) -> AnyView {
    match *kind {
        FieldKind::Date(ref field) => {
            let shape = date_depth(field);
            let field = field.clone();
            let admit = Callback::new(move |(typed, _zone): (String, String)| {
                crate::control::temporal::admit_date(&field, &typed)
            });
            view! {
                <TemporalControl
                    shape=shape
                    native=native_date(shape)
                    parts=&DATE_PARTS
                    timezone=None
                    at=slot
                    admit=admit
                />
            }
            .into_any()
        }
        FieldKind::Time(ref field) => {
            let shape = time_depth(field);
            let timezone = field.timezone;
            let field = field.clone();
            let admit = Callback::new(move |(typed, zone): (String, String)| {
                crate::control::temporal::admit_time(&field, &typed, &zone)
            });
            view! {
                <TemporalControl
                    shape=shape
                    native=native_time(shape)
                    parts=&TIME_PARTS
                    timezone=Some(timezone)
                    at=slot
                    admit=admit
                />
            }
            .into_any()
        }
        FieldKind::DateTime(ref field) => {
            let shape = date_time_depth(field);
            let timezone = field.timezone;
            let field = field.clone();
            let admit = Callback::new(move |(typed, zone): (String, String)| {
                crate::control::temporal::admit_date_time(&field, &typed, &zone)
            });
            view! {
                <TemporalControl
                    shape=shape
                    native=native_date_time(shape)
                    parts=&DATE_TIME_PARTS
                    timezone=Some(timezone)
                    at=slot
                    admit=admit
                />
            }
            .into_any()
        }
        _ => ().into_any(),
    }
}

/// One field of a form, with everything the template said about it.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn FieldView(
    /// The field, as the compiler derived it.
    field: FormField,
    /// The form the field writes into.
    state: FormState,
    /// Which occurrence of each repeating group above the field.
    path: Path,
    /// The language a label is shown in.
    language: LanguageTag,
) -> impl IntoView {
    let label = localized(&field.label, &language);
    let help = localized(&field.help, &language);
    let repeatable = field.occurrences.is_repeatable();
    let mandatory = field.occurrences.is_mandatory();
    let deprecated = field.is_deprecated;
    let bands = field.reference_ranges.clone();
    let key = field.key.clone();
    let minimum = field.occurrences.minimum;
    let maximum = field.occurrences.maximum;

    let repeats = {
        let key = key.clone();
        let path = path.clone();
        move || (0..state.shown(&key, &path)).collect::<Vec<_>>()
    };

    let bodies = {
        let field = field.clone();
        let language = language.clone();
        let path = path.clone();
        let label = label.clone();
        move || {
            repeats()
                .into_iter()
                .map(|occurrence| {
                    let slot = Slot::new(
                        state,
                        Address::new(field.key.clone(), path.clone(), occurrence),
                        language.clone(),
                        label.clone(),
                    );
                    view! { <Occurrence field=field.clone() at=slot repeatable=repeatable /> }
                })
                .collect::<Vec<_>>()
        }
    };

    let add = {
        let key = key.clone();
        let path = path.clone();
        Callback::new(move |()| {
            state.add_occurrence(&key, &path, maximum);
        })
    };
    let remove = {
        let key = key.clone();
        let path = path.clone();
        Callback::new(move |()| {
            state.remove_field_occurrence(&key, &path, minimum);
        })
    };
    let at_ceiling = {
        let key = key.clone();
        let path = path.clone();
        Signal::derive(move || !state.can_add(&key, &path, maximum))
    };
    let at_floor = Signal::derive(move || !state.can_remove(&key, &path, minimum));

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex flex-wrap items-center gap-2">
                <span class=LABEL>{label.clone()}</span>
                <Show when=move || mandatory>
                    <span class=format!("{BADGE} {}", Tone::Accent.subtle())>"Required"</span>
                </Show>
                <Show when=move || deprecated>
                    <StatusPill tone=Tone::Warn label="Deprecated" />
                </Show>
            </div>
            {bodies}
            <Show when=move || repeatable>
                <Repeats
                    add=add
                    remove=remove
                    at_ceiling=at_ceiling
                    at_floor=at_floor
                    add_label="Add"
                    remove_label="Remove"
                />
            </Show>
            <Show when={
                let help = help.clone();
                move || !help.is_empty()
            }>
                <span class=HINT>{help.clone()}</span>
            </Show>
            {bands.map(|stated| view! { <Bands bands=*stated /> })}
        </div>
    }
}

/// One repeat of one field: its control, and the null flavour beside it.
///
/// The repeat is a `<fieldset>` whose `<legend>` carries the field's label,
/// so every input inside it has an accessible name even where the control
/// draws several (a quantity draws a magnitude and a unit, and neither one is
/// the field). The legend is read aloud rather than drawn, because the field
/// header above already shows the label.
#[component]
fn Occurrence(
    /// The field, as the compiler derived it.
    field: FormField,
    /// Where this repeat's value goes.
    at: Slot,
    /// Whether the field repeats, which decides whether the legend numbers
    /// this repeat.
    repeatable: bool,
) -> impl IntoView {
    let slot = at;
    seed(&field, &slot);
    let legend = if repeatable {
        format!(
            "{}, {}",
            slot.label,
            slot.address.occurrence.saturating_add(1)
        )
    } else {
        slot.label.clone()
    };
    let body = if field.is_fixed {
        view! {
            <p class="text-sm text-ink-muted">
                "The template fixed this value, so nothing is entered."
            </p>
        }
        .into_any()
    } else {
        control(&field.kind, &field.rm_type, &slot)
    };
    let null = field.null_flavour.clone();
    let offered = null.is_offered;
    view! {
        <fieldset class="rounded-control border border-edge p-2">
            <legend class="sr-only">{legend}</legend>
            {body}
            <Show when=move || offered>
                <NullFlavourControl affordance=null.clone() at=slot.clone() />
            </Show>
        </fieldset>
    }
}

/// Writes the template's prefill into a slot nothing has been entered into.
fn seed(field: &FormField, slot: &Slot) {
    if slot.entered().is_some() {
        return;
    }
    let Some(prefill) = field.prefill.as_ref() else {
        return;
    };
    if let Some(datum) = prefill::datum(prefill, &field.kind, &field.rm_type) {
        slot.set(datum);
    }
}

/// The reference bands the template states beside a value.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 makes these a
/// reader's context rather than an entry: a laboratory sends them with a
/// result so a reader can tell the result from the band it is read against.
/// Nothing here is collected.
#[component]
fn Bands(
    /// The bands the template states.
    bands: ReferenceRanges,
) -> impl IntoView {
    if bands.is_empty() {
        return ().into_any();
    }
    let mut stated: Vec<String> = Vec::new();
    if bands.normal_status.is_some() {
        stated.push("a normal status".to_owned());
    }
    if bands.normal_range.is_some() {
        stated.push("a normal range".to_owned());
    }
    if !bands.other.is_empty() {
        stated.push(format!("{} other band(s)", bands.other.len()));
    }
    view! {
        <div class=WELL>
            <p class="text-xs text-ink-muted">
                {format!("The template states {} beside this value.", stated.join(", "))}
            </p>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{
        BooleanField, ComponentValidity, CountField, DateField, DateTimeField, DurationField,
        FieldKind, IdentifierField, IntervalField, MultimediaField, OrdinalField, ParsableField,
        ProportionField, QuantityField, RealField, StateField, TextField, TimeField, UriField,
    };
    use ferrochart_form::ids::RmTypeName;
    use ferrochart_form::value::ValueSet;

    /// One value of every variant, so a variant added upstream fails this
    /// list rather than silently drawing nothing.
    fn every_kind() -> Vec<FieldKind> {
        let temporal = ComponentValidity::Optional;
        vec![
            FieldKind::Boolean(BooleanField {
                true_allowed: true,
                false_allowed: true,
            }),
            FieldKind::Text(TextField::default()),
            FieldKind::Uri(UriField {
                patterns: Vec::new(),
                required_scheme: None,
            }),
            FieldKind::Coded(ferrochart_form::field::CodedField {
                value_set: ValueSet::Unconstrained,
                strictness: None,
                rubric: None,
            }),
            FieldKind::Ordinal(OrdinalField {
                options: Vec::new(),
            }),
            FieldKind::Count(CountField::default()),
            FieldKind::Quantity(QuantityField {
                property: None,
                units: Vec::new(),
            }),
            FieldKind::Proportion(ProportionField {
                kinds: Vec::new(),
                numerator: RealField::default(),
                denominator: RealField::default(),
                decimals: CountField::default(),
                is_integral: None,
            }),
            FieldKind::Date(DateField {
                month: temporal,
                day: temporal,
                ranges: Vec::new(),
            }),
            FieldKind::Time(TimeField {
                minute: temporal,
                second: temporal,
                timezone: temporal,
                ranges: Vec::new(),
            }),
            FieldKind::DateTime(DateTimeField {
                month: temporal,
                day: temporal,
                hour: temporal,
                minute: temporal,
                second: temporal,
                timezone: temporal,
                ranges: Vec::new(),
            }),
            FieldKind::Duration(DurationField {
                components: std::collections::BTreeSet::new(),
                ranges: Vec::new(),
            }),
            FieldKind::Identifier(IdentifierField::default()),
            FieldKind::Multimedia(MultimediaField {
                media_types: ValueSet::Unconstrained,
                uri: None,
            }),
            FieldKind::Parsable(ParsableField::default()),
            FieldKind::Interval(Box::new(IntervalField {
                element_rm_type: RmTypeName::new("DV_COUNT"),
                lower: FieldKind::Count(CountField::default()),
                upper: FieldKind::Count(CountField::default()),
                lower_included: None,
                upper_included: None,
                lower_unbounded: None,
                upper_unbounded: None,
            })),
            FieldKind::State(StateField { states: Vec::new() }),
            FieldKind::Choice(ferrochart_form::field::ChoiceField {
                alternatives: Vec::new(),
            }),
        ]
    }

    #[test]
    fn every_variant_of_the_derivation_table_has_a_control() {
        let kinds = every_kind();
        assert_eq!(kinds.len(), 18, "the derivation table has eighteen rows");
        let mut names: Vec<&str> = kinds.iter().map(FieldKind::name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 18, "two variants share one name");
    }
}
