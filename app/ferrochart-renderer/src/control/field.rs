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
use ferrochart_form::value::Prefill;
use ferrochart_form::values::Entered;
use leptos::prelude::*;

use crate::control::boolean::BooleanControl;
use crate::control::choice::ChoiceControl;
use crate::control::coded::CodedControl;
use crate::control::count::CountControl;
use crate::control::duration::DurationControl;
use crate::control::identifier::IdentifierControl;
use crate::control::interval::IntervalControl;
use crate::control::multimedia::MultimediaControl;
use crate::control::null_flavour::{NoValueButton, NullFlavourControl};
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
use crate::kit::field::{FIELD_LABEL, HINT};
use crate::kit::notice::Notice;
use crate::kit::surface::WELL;
use crate::kit::tone::Tone;
use crate::overlay::Laid;
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

/// The name over one field, the text under it, and the value it starts with.
///
/// What a person authored wins over the archetype rubric, and a layout that
/// says nothing leaves the rubric exactly where it was. The authored default
/// is returned beside them because it comes from the same record.
fn words(field: &FormField, language: &LanguageTag) -> (String, String, Option<Prefill>) {
    let laid = Laid::from_context();
    let label = laid.label(&field.key, language).unwrap_or_else(|| {
        crate::label::of(
            &field.label,
            language,
            &field.key,
            crate::plain::of_kind(&field.kind),
        )
    });
    let help = laid
        .help(&field.key, language)
        .unwrap_or_else(|| localized(&field.help, language));
    let authored = laid
        .at(&field.key)
        .and_then(|layout| layout.default.clone());
    (label, help, authored)
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
    let (label, help, authored) = words(&field, &language);
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
        // A field always draws one control: its label is drawn whether or not
        // a value is entered, and a row with a label and nothing under it
        // would be a question with no way to answer it.
        move || (0..state.shown(&key, &path, (minimum as usize).max(1))).collect::<Vec<_>>()
    };

    let bodies = {
        let field = field.clone();
        let language = language.clone();
        let path = path.clone();
        let label = label.clone();
        let authored = authored.clone();
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
                    view! { <Occurrence field=field.clone() at=slot repeatable=repeatable authored=authored.clone() /> }
                })
                .collect::<Vec<_>>()
        }
    };

    let add = {
        let key = key.clone();
        let path = path.clone();
        Callback::new(move |()| {
            state.add_occurrence(&key, &path, minimum, maximum);
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
        Signal::derive(move || !state.can_add(&key, &path, minimum, maximum))
    };
    let at_floor = Signal::derive(move || !state.can_remove(&key, &path, minimum));

    view! {
        <div class="flex flex-col gap-1.5">
            <div class="flex flex-wrap items-baseline gap-2">
                <span class=FIELD_LABEL>{label.clone()}</span>
                <Show when=move || mandatory>
                    <span class=format!("{BADGE} {}", Tone::Accent.subtle())>"Required"</span>
                </Show>
                <Show when=move || deprecated>
                    <StatusPill tone=Tone::Warn label="Deprecated" />
                </Show>
            </div>
            // The description sits with the name it describes. Under the
            // control it read as a caption for the field below it, because
            // that field's name was the next line (issue #190).
            <Show when={
                let help = help.clone();
                move || !help.is_empty()
            }>
                <span class=HINT>{help.clone()}</span>
            </Show>
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
            {bands.map(|stated| view! { <Bands bands=*stated /> })}
        </div>
    }
}

/// The value the template fixed, in words a person reads.
///
/// A fixed field is not a question: the template already answered it, and
/// showing the answer is more use than saying that one exists (issue #190).
/// A shape this build cannot say keeps its own text, for the reason
/// `crate::plain` gives.
fn fixed_as_text(prefill: &Prefill) -> String {
    match *prefill {
        Prefill::Boolean(value) => (if value { "Yes" } else { "No" }).to_owned(),
        Prefill::Integer(value) => value.to_string(),
        Prefill::Real(value) => value.to_string(),

        Prefill::Coded {
            ref code,
            ref rubric,
        } => rubric.clone().unwrap_or_else(|| code.code.clone()),
        Prefill::Quantity {
            magnitude,
            ref units,
            ..
        } => format!("{magnitude} {units}"),
        Prefill::Ordinal { value, ref symbol } => format!("{value} ({})", symbol.code),
        Prefill::Text(ref value) | Prefill::Temporal(ref value) | Prefill::Opaque(ref value) => {
            value.clone()
        }
        _ => "a value this build cannot show".to_owned(),
    }
}

/// The value the template fixed for `field`, in words, where it fixed one.
///
/// The template states the value two ways and either one is the answer: a
/// `default_value`, which reaches the definition as a prefill, and a
/// constraint that admits exactly one value, which
/// [`ferrochart_form::field::FieldKind::only_admitted`] reads. A field the
/// second way used to draw the sentence "The template fixed this value",
/// which named an answer it then declined to show (issue #207).
fn fixed(field: &FormField) -> Option<String> {
    if !field.is_fixed {
        return None;
    }
    field
        .prefill
        .clone()
        .or_else(|| field.kind.only_admitted())
        .as_ref()
        .map(fixed_as_text)
}

/// A field whose value the template fixed and whose element it says may be
/// absent.
///
/// The question is not which value the field carries, because the template
/// answered that. It is whether the element is recorded at all: openEHR AM
/// Release-2.3.0 `AOM1.4.html` section 4.3.6 makes `occurrences` the count a
/// node may appear in data, so a lower bound of zero leaves that decision to
/// the person. Ticking records the value the template fixed and clearing
/// leaves the element out.
#[component]
fn FixedOptIn(
    /// The field, as the compiler derived it.
    field: FormField,
    /// Where the value goes.
    at: Slot,
    /// The fixed value, in words.
    said: String,
) -> impl IntoView {
    let slot = at;
    let id = slot.part("fixed");
    let recorded = {
        let slot = slot.clone();
        Signal::derive(move || matches!(slot.entered(), Some(Entered::Value(_))))
    };
    let on_change = {
        let slot = slot.clone();
        let field = field.clone();
        move |event: leptos::ev::Event| {
            if !event_target_checked(&event) {
                slot.clear();
                return;
            }
            let Some(prefill) = field.prefill.clone().or_else(|| field.kind.only_admitted()) else {
                return;
            };
            if let Some(datum) = prefill::datum(&prefill, &field.kind, &field.rm_type) {
                slot.set(datum);
            }
        }
    };
    view! {
        <label class="inline-flex items-center gap-2 text-sm text-ink" for=id.clone()>
            <input
                id=id.clone()
                type="checkbox"
                class="h-4 w-4 accent-accent"
                prop:checked=move || recorded.get()
                on:change=on_change
            />
            {said}
        </label>
    }
}

/// How wide a field of this kind is allowed to run.
///
/// No specification governs this: our own design. A date, a count and a unit
/// are short answers, and stretching them across the content column made a
/// form of short questions read as a wall of full-width boxes (issue #190).
/// Prose, an attachment and a choice keep the width, because their answers
/// genuinely use it.
const fn widest(kind: &FieldKind) -> &'static str {
    match *kind {
        FieldKind::Text(_)
        | FieldKind::Parsable(_)
        | FieldKind::Multimedia(_)
        | FieldKind::Uri(_)
        | FieldKind::Choice(_)
        // The three temporal kinds cap their own control, because a partial
        // value is a row of component boxes and a pinned one is a picker,
        // and the two want different widths (issue #197).
        | FieldKind::Date(_)
        | FieldKind::Time(_)
        | FieldKind::DateTime(_) => "",
        _ => "max-w-md",
    }
}

/// One repeat of one field: its value, or the reason there is not one.
///
/// The repeat is a `<fieldset>` whose `<legend>` carries the field's label,
/// so every input inside it has an accessible name even where the control
/// draws several (a quantity draws a magnitude and a unit, and neither one is
/// the field). The legend is read aloud rather than drawn, because the field
/// header above already shows the label. The fieldset draws no box: the group
/// around it is already a card, and a third border around every value was
/// most of what made a form feel like a wall (issue #190).
///
/// A field shows its value control OR its null flavour, never both, because
/// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
/// `ELEMENT` no state that carries the two together.
#[component]
fn Occurrence(
    /// The field, as the compiler derived it.
    field: FormField,
    /// Where this repeat's value goes.
    at: Slot,
    /// Whether the field repeats, which decides whether the legend numbers
    /// this repeat.
    repeatable: bool,
    /// The value a person authored for the field, where they authored one.
    authored: Option<Prefill>,
) -> impl IntoView {
    let slot = at;
    seed(&field, authored.as_ref(), &slot);
    let legend = if repeatable {
        format!(
            "{}, {}",
            slot.label,
            slot.address.occurrence.saturating_add(1)
        )
    } else {
        slot.label.clone()
    };
    // A value the template fixed is not one a person can decline to give, so
    // a fixed field offers no null flavour whatever the affordance says. The
    // screen was asking "is the family member deceased?", answering it itself,
    // and then offering "No value" beside the answer.
    let offered = field.null_flavour.is_offered && !field.is_fixed;
    // Whether the reader asked for the flavour picker before choosing one.
    // The stored value answers every other case, so this signal is only ever
    // true between the click and the choice.
    let choosing = RwSignal::new(false);
    let holds_null = {
        let slot = slot.clone();
        Signal::derive(move || matches!(slot.entered(), Some(Entered::Null { .. })))
    };
    let instead = move || offered && (choosing.get() || holds_null.get());

    let value_row = {
        let field = field.clone();
        let slot = slot.clone();
        move || {
            let body = match fixed(&field) {
                Some(said) if field.occurrences.minimum == 0 => {
                    view! { <FixedOptIn field=field.clone() at=slot.clone() said=said /> }
                        .into_any()
                }
                Some(said) => view! { <p class="text-sm text-ink">{said}</p> }.into_any(),
                None => control(&field.kind, &field.rm_type, &slot),
            };
            view! {
                // `w-full` rather than `grow`, so a capped control is exactly
                // its cap wide and the "No value" beside it lands in the same
                // column on every field instead of tracking each control's
                // own width.
                <div class="flex items-start gap-3">
                    <div class=format!("min-w-0 w-full {}", widest(&field.kind))>{body}</div>
                    <Show when=move || offered>
                        <NoValueButton on_start=Callback::new(move |()| choosing.set(true)) />
                    </Show>
                </div>
            }
        }
    };
    let null_row = {
        let affordance = field.null_flavour.clone();
        let slot = slot.clone();
        move || {
            view! {
                <NullFlavourControl
                    affordance=affordance.clone()
                    at=slot.clone()
                    on_cancel=Callback::new(move |()| choosing.set(false))
                />
            }
        }
    };

    view! {
        <fieldset class="min-w-0">
            <legend class="sr-only">{legend}</legend>
            <Show when=instead fallback=value_row>
                {null_row()}
            </Show>
        </fieldset>
    }
}

/// Writes a prefill into a slot nothing has been entered into.
///
/// A value a person authored wins over the one the template states, because
/// the authored one is the later decision and the layout exists to carry it.
/// Neither can widen what the field admits: a prefill whose shape the kind
/// does not name is written nowhere.
///
/// A field the template both fixed and requires is written too, from the one
/// value its constraint admits. The template answered the question and the
/// node has to be there, so leaving the document without it would commit a
/// COMPOSITION the template refuses (issue #207). A fixed field the template
/// says may be absent is NOT written: whether the element is there is the
/// reader's decision, and [`FixedOptIn`] is where they make it.
fn seed(field: &FormField, authored: Option<&Prefill>, slot: &Slot) {
    if slot.entered().is_some() {
        return;
    }
    let required = field.occurrences.minimum > 0;
    let Some(prefill) = authored
        .cloned()
        .or_else(|| field.prefill.clone())
        .or_else(|| required.then(|| field.kind.only_admitted()).flatten())
    else {
        return;
    };
    if let Some(datum) = prefill::datum(&prefill, &field.kind, &field.rm_type) {
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
    fn a_field_the_constraint_fixed_shows_the_value_rather_than_a_sentence() {
        // The template states the value as a constraint rather than as a
        // `default_value`, so `prefill` is empty and the value is still
        // knowable (issue #207).
        let mut field = fixed_field();
        field.prefill = None;
        assert_eq!(super::fixed(&field).as_deref(), Some("Yes"));
    }

    #[test]
    fn a_field_the_template_did_not_fix_shows_no_fixed_value() {
        let mut field = fixed_field();
        field.is_fixed = false;
        assert_eq!(super::fixed(&field), None);
    }

    #[test]
    fn a_stated_default_outranks_the_one_admitted_value() {
        let mut field = fixed_field();
        field.prefill = Some(ferrochart_form::value::Prefill::Text("Deceased".to_owned()));
        assert_eq!(super::fixed(&field).as_deref(), Some("Deceased"));
    }

    /// A boolean field the template narrowed to `true` and says may be
    /// absent, which is the flag idiom the family history template uses.
    fn fixed_field() -> ferrochart_form::field::FormField {
        let kind = FieldKind::Boolean(BooleanField {
            true_allowed: true,
            false_allowed: false,
        });
        ferrochart_form::field::FormField {
            key: ferrochart_form::key::NodeKey::root(),
            rm_type: RmTypeName::new("DV_BOOLEAN"),
            label: ferrochart_form::text::Localized::empty(),
            help: ferrochart_form::text::Localized::empty(),
            occurrences: ferrochart_form::occurrences::Occurrences::bounded(0, 1),
            kind,
            name_constraint: None,
            reference_ranges: None,
            is_ordered: false,
            is_unique: false,
            null_flavour: ferrochart_form::field::NullFlavour::absent(),
            prefill: None,
            is_deprecated: false,
            is_fixed: true,
        }
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
