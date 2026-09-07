// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The field derivation table, one case per Reference Model class, and the
//! whole vendored pack derived end to end.
//!
//! The sections cited are openEHR RM Release-1.1.0 `data_types.html` and
//! `data_structures.html`, and openEHR AM Release-2.3.0 `AOM1.4.html` and
//! `AOM2.html`.

use std::collections::BTreeMap;
use std::fs;

use ferrochart_compile::derive::error::DeriveError;
use ferrochart_compile::{adl2, adl14, derive};
use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::field::{
    ComponentValidity, DurationComponent, FieldKind, FormField, ProportionKind,
};
use ferrochart_form::group::{GroupShape, UndeterminedReason};
use ferrochart_form::ids::{ArchetypeId, LanguageTag};
use ferrochart_form::key::NodeKey;
use ferrochart_form::value::{ExpansionSource, Prefill, ValueSet};

use crate::support::templates;

const GRID: &str = include_str!("../fixtures/ferro_datatype_grid.opt");
const OBSERVATION: &str = include_str!("../fixtures/ferro_test_observation.opt");
const XSD_ONLY: &str = include_str!("../fixtures/ferro_xsd_only.opt");
const ADL2_OBSERVATION: &str = include_str!("../fixtures/ferro_test_observation.v1.0.0.adls");
const ADL2_EXTRAS: &str = include_str!("../fixtures/ferro_adl2_extras.v1.0.0.adls");
const ADL2_UNIT_BOUNDS: &str = include_str!("../fixtures/ferro_adl2_unit_bounds.v1.0.0.adls");
const MAGNITUDE_LIST: &str = include_str!("../fixtures/ferro_quantity_magnitude_list.opt");
const ADL2_INTERVAL_BAND: &str = include_str!("../fixtures/ferro_adl2_interval_band.v1.0.0.adls");
const UNMODELLED_ATTRIBUTE: &str = include_str!("../fixtures/ferro_unmodelled_attribute.opt");
const UNMODELLED_DATA_VALUE: &str = include_str!("../fixtures/ferro_unmodelled_data_value.opt");

fn from_opt14(xml: &str) -> FormDefinition {
    let template = adl14::from_xml(xml).expect("the fixture reads");
    derive::form(&template).expect("the fixture derives")
}

fn from_adl2(source: &str) -> FormDefinition {
    let template = adl2::from_source(source, &[]).expect("the fixture reads");
    derive::form(&template).expect("the fixture derives")
}

fn refusal(xml: &str) -> DeriveError {
    let template = adl14::from_xml(xml).expect("the fixture reads");
    derive::form(&template).expect_err("the fixture is refused")
}

fn refusal_adl2(source: &str) -> DeriveError {
    let template = adl2::from_source(source, &[]).expect("the fixture reads");
    derive::form(&template).expect_err("the fixture is refused")
}

/// The field whose terminal key step carries `node_id`.
fn field<'a>(form: &'a FormDefinition, node_id: &str) -> &'a FormField {
    form.fields()
        .find(|field| {
            field
                .key
                .terminal()
                .and_then(|step| step.node_id.as_ref())
                .is_some_and(|code| code.as_str() == node_id)
        })
        .unwrap_or_else(|| panic!("no field {node_id}"))
}

fn kind<'a>(form: &'a FormDefinition, node_id: &str) -> &'a FieldKind {
    &field(form, node_id).kind
}

fn english() -> LanguageTag {
    LanguageTag::new("en")
}

#[test]
fn a_boolean_with_one_admitted_value_is_fixed_and_one_with_two_is_a_control() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 6.2.2: `C_BOOLEAN` carries
    // `true_valid` and `false_valid`, and one of the two admits one value.
    let form = from_opt14(GRID);
    assert_eq!(
        *kind(&form, "at0010"),
        FieldKind::Boolean(ferrochart_form::field::BooleanField {
            true_allowed: true,
            false_allowed: false,
        })
    );
    assert!(field(&form, "at0010").is_fixed);
    assert_eq!(
        *kind(&form, "at0011"),
        FieldKind::Boolean(ferrochart_form::field::BooleanField {
            true_allowed: true,
            false_allowed: true,
        })
    );
    assert!(!field(&form, "at0011").is_fixed);
}

#[test]
fn a_count_collects_the_range_its_integer_constraint_states() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.9 collects
    // `magnitude`; AOM1.4.html section 6.2.4 constrains it with a `range`.
    let form = from_opt14(GRID);
    let FieldKind::Count(ref count) = *kind(&form, "at0012") else {
        panic!("at0012 is not a count");
    };
    assert_eq!(count.ranges.len(), 1);
    let range = count.ranges.first().expect("one range");
    assert_eq!(range.minimum, Some(0));
    assert_eq!(range.maximum, Some(10));
    assert!(range.minimum_included && range.maximum_included);
}

#[test]
fn an_assumed_value_never_reaches_the_form() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.2.3.2: "default values do
    // appear in data, while assumed values don't". The grid's count states an
    // assumed value and no default.
    let form = from_opt14(GRID);
    assert_eq!(field(&form, "at0012").prefill, None);
}

#[test]
fn a_default_value_prefills_the_field() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.2.3.2, the other half of
    // the same sentence.
    let form = from_opt14(XSD_ONLY);
    assert_eq!(
        field(&form, "at0004").prefill,
        Some(Prefill::Text("A synthetic default".to_owned()))
    );
}

#[test]
fn a_date_pattern_is_honoured_component_by_component() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 6.2.6: "`YYYY-??-??` (date
    // with optional month and day)"; the grid forbids the day with `XX`.
    let form = from_opt14(GRID);
    let FieldKind::Date(ref date) = *kind(&form, "at0013") else {
        panic!("at0013 is not a date");
    };
    assert_eq!(date.month, ComponentValidity::Optional);
    assert_eq!(date.day, ComponentValidity::Prohibited);
}

#[test]
fn a_time_pattern_and_its_timezone_validity_are_both_honoured() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 6.2.7: "`HH:??:xx` (time
    // with optional minutes and seconds not allowed)", plus
    // `C_TIME.timezone_validity`.
    let form = from_opt14(GRID);
    let FieldKind::Time(ref time) = *kind(&form, "at0014") else {
        panic!("at0014 is not a time");
    };
    assert_eq!(time.minute, ComponentValidity::Optional);
    assert_eq!(time.second, ComponentValidity::Prohibited);
    assert_eq!(time.timezone, ComponentValidity::Prohibited);
}

#[test]
fn a_date_time_pattern_splits_at_the_designator() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 6.2.8 spells a date and
    // time as `YYYY-MM-DDT??:??:??`.
    let form = from_opt14(GRID);
    let FieldKind::DateTime(ref stamp) = *kind(&form, "at0015") else {
        panic!("at0015 is not a date and time");
    };
    assert_eq!(stamp.month, ComponentValidity::Mandatory);
    assert_eq!(stamp.day, ComponentValidity::Mandatory);
    assert_eq!(stamp.hour, ComponentValidity::Optional);
    assert_eq!(stamp.minute, ComponentValidity::Optional);
    assert_eq!(stamp.second, ComponentValidity::Optional);
}

#[test]
fn a_duration_pattern_names_the_slots_the_value_may_fill() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 6.2.9 gives the permitted
    // patterns as `P[Y|y][M|m][D|d][T[H|h][M|m][S|s]]` and `P[W|w]`.
    let form = from_opt14(GRID);
    let FieldKind::Duration(ref duration) = *kind(&form, "at0016") else {
        panic!("at0016 is not a duration");
    };
    assert_eq!(
        duration.components,
        [
            DurationComponent::Years,
            DurationComponent::Months,
            DurationComponent::Days
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn an_identifier_collects_its_four_parts() {
    // openEHR RM Release-1.1.0 data_types.html section 4.2.4.
    let form = from_opt14(GRID);
    let FieldKind::Identifier(ref identifier) = *kind(&form, "at0017") else {
        panic!("at0017 is not an identifier");
    };
    assert_eq!(identifier.issuer.options, ["Ferro synthetic registry"]);
    assert_eq!(identifier.assigner.options, ["Ferro synthetic assigner"]);
    assert_eq!(identifier.id.patterns, ["[0-9]{6}"]);
    assert_eq!(identifier.identifier_type.options, ["synthetic"]);
}

#[test]
fn a_multimedia_media_type_list_is_the_file_inputs_filter() {
    // openEHR RM Release-1.1.0 data_types.html section 9.2.2 constrains
    // `media_type` with a `CODE_PHRASE`.
    let form = from_opt14(GRID);
    let FieldKind::Multimedia(ref multimedia) = *kind(&form, "at0018") else {
        panic!("at0018 is not a multimedia value");
    };
    let ValueSet::Enumerated(ref set) = multimedia.media_types else {
        panic!("the media types are not enumerated");
    };
    let codes: Vec<&str> = set
        .options
        .iter()
        .map(|option| option.code.code.as_str())
        .collect();
    assert_eq!(codes, ["image/png", "image/jpeg"]);
    // The codes come from IANA rather than the archetype, so their display
    // text is a terminology lookup.
    assert!(set.needs_display_lookup);
}

#[test]
fn a_parsable_value_offers_the_formalisms_the_template_admits() {
    // openEHR RM Release-1.1.0 data_types.html section 9.2.3.
    let form = from_opt14(GRID);
    let FieldKind::Parsable(ref parsable) = *kind(&form, "at0019") else {
        panic!("at0019 is not a parsable value");
    };
    assert_eq!(parsable.formalism.options, ["text/plain", "text/markdown"]);
    assert!(parsable.formalism.options_closed);
}

#[test]
fn an_ehr_uri_requires_the_scheme_its_invariant_names() {
    // openEHR RM Release-1.1.0 data_types.html section 10.3.2 invariant
    // `Scheme_valid`: a `DV_EHR_URI` has the scheme name `ehr`.
    let form = from_opt14(GRID);
    let FieldKind::Uri(ref plain) = *kind(&form, "at0020") else {
        panic!("at0020 is not a URI");
    };
    assert_eq!(plain.required_scheme, None);
    assert_eq!(plain.patterns, ["https://.*"]);
    let FieldKind::Uri(ref ehr) = *kind(&form, "at0021") else {
        panic!("at0021 is not a URI");
    };
    assert_eq!(ehr.required_scheme.as_deref(), Some("ehr"));
}

#[test]
fn an_interval_collects_two_ends_of_its_element_type() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.2: both ends are
    // the same `DV_ORDERED` descendant, and each carries its own constraint.
    let form = from_opt14(GRID);
    let FieldKind::Interval(ref interval) = *kind(&form, "at0022") else {
        panic!("at0022 is not an interval");
    };
    assert_eq!(interval.element_rm_type.as_str(), "DV_COUNT");
    let FieldKind::Count(ref lower) = interval.lower else {
        panic!("the lower end is not a count");
    };
    let FieldKind::Count(ref upper) = interval.upper else {
        panic!("the upper end is not a count");
    };
    assert_eq!(
        lower.ranges.first().and_then(|range| range.maximum),
        Some(5)
    );
    assert_eq!(
        upper.ranges.first().and_then(|range| range.minimum),
        Some(6)
    );
}

#[test]
fn a_node_that_accepts_a_code_or_free_text_is_one_field_of_two_alternatives() {
    // openEHR AM Release-2.3.0 AOM2.html section 4.2.8.2 prescribes two
    // sibling nodes for a value that may be either.
    let form = from_opt14(GRID);
    let FieldKind::Choice(ref choice) = *kind(&form, "at0023") else {
        panic!("at0023 is not a choice");
    };
    let kinds: Vec<&str> = choice
        .alternatives
        .iter()
        .map(|alternative| alternative.kind.name())
        .collect();
    assert_eq!(kinds, ["coded", "text"]);
    // The field collects one of them, so it names the class they share.
    assert_eq!(field(&form, "at0023").rm_type.as_str(), "DATA_VALUE");
}

#[test]
fn an_element_whose_value_the_template_never_constrained_is_recorded_not_rendered() {
    // openEHR RM Release-1.1.0 data_structures.html section 5.2.3 types
    // `ELEMENT.value` as a `DATA_VALUE`, so an unconstrained one admits every
    // data type there is.
    let form = from_opt14(GRID);
    let content = form
        .undetermined()
        .find(|content| {
            content
                .key
                .terminal()
                .and_then(|step| step.node_id.as_ref())
                .is_some_and(|code| code.as_str() == "at0024")
        })
        .expect("the unconstrained element is recorded");
    assert_eq!(content.reason, UndeterminedReason::UnconstrainedValue);
    assert!(
        form.fields().all(|field| field.key != content.key),
        "the unconstrained element was rendered"
    );
}

#[test]
fn an_interval_whose_element_type_the_template_never_states_is_recorded_not_rendered() {
    let form = from_opt14(GRID);
    let content = form
        .undetermined()
        .find(|content| {
            content
                .key
                .terminal()
                .and_then(|step| step.node_id.as_ref())
                .is_some_and(|code| code.as_str() == "at0029")
        })
        .expect("the untyped interval is recorded");
    assert_eq!(content.reason, UndeterminedReason::UntypedInterval);
}

#[test]
fn occurrences_decide_repeatability_optionality_and_the_null_flavour() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.3.6, plus the `ELEMENT`
    // null mechanism of RM Release-1.1.0 data_structures.html section 5.2.3.
    let form = from_opt14(GRID);
    let repeatable = field(&form, "at0025");
    assert!(repeatable.occurrences.is_repeatable());
    assert!(repeatable.occurrences.is_optional());
    assert!(repeatable.null_flavour.is_offered);

    let optional = field(&form, "at0023");
    assert!(!optional.occurrences.is_repeatable());
    assert!(optional.occurrences.is_optional());
    assert!(optional.null_flavour.is_offered);

    let mandatory = field(&form, "at0026");
    assert!(mandatory.occurrences.is_mandatory());
    assert!(!mandatory.null_flavour.is_offered);
    assert!(mandatory.null_flavour.options.is_empty());
    assert!(!mandatory.null_flavour.accepts_reason);
}

#[test]
fn the_null_flavour_options_are_the_four_the_spec_names() {
    // openEHR RM Release-1.1.0 data_structures.html section 4.1 names
    // 253|unknown|, 271|no information|, 272|masked| and 273|not applicable|.
    let form = from_opt14(GRID);
    let offered = field(&form, "at0025");
    let codes: Vec<&str> = offered
        .null_flavour
        .options
        .iter()
        .map(|option| option.code.code.as_str())
        .collect();
    assert_eq!(codes, ["253", "271", "272", "273"]);
    assert!(
        offered
            .null_flavour
            .options
            .iter()
            .all(|option| option.code.terminology.as_str() == "openehr")
    );
    assert!(offered.null_flavour.accepts_reason);
}

#[test]
fn a_name_the_template_leaves_open_is_recorded_beside_the_field() {
    // openEHR RM Release-1.1.0 common.html gives every `LOCATABLE` a mandatory
    // `name`. No specification governs whether a name the template leaves open
    // is entered, so the constraint is recorded rather than made an item.
    let form = from_opt14(GRID);
    let named = field(&form, "at0027");
    let constraint = named
        .name_constraint
        .as_ref()
        .expect("the open name is recorded");
    assert_eq!(constraint.rm_type.as_str(), "DV_CODED_TEXT");
    let FieldKind::Coded(ref coded) = constraint.kind else {
        panic!("the name is not a selection");
    };
    let ValueSet::Enumerated(ref set) = coded.value_set else {
        panic!("the name set is not enumerated");
    };
    assert_eq!(set.options.len(), 2);
    // A field whose name the template pins carries no name constraint.
    assert_eq!(field(&form, "at0026").name_constraint, None);
}

#[test]
fn a_proportion_carries_the_kind_the_integer_names() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.11:
    // `pk_percent : Integer = 2`, "Denominator is 100".
    let form = from_opt14(GRID);
    let FieldKind::Proportion(ref proportion) = *kind(&form, "at0028") else {
        panic!("at0028 is not a proportion");
    };
    assert_eq!(proportion.kinds, [ProportionKind::Percent]);
    assert_eq!(
        proportion
            .numerator
            .ranges
            .first()
            .and_then(|range| range.maximum),
        Some(100.0)
    );
    assert_eq!(proportion.decimals.options, [1]);
}

#[test]
fn changing_a_quantitys_unit_changes_its_range_and_its_decimals() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.8: each permitted
    // unit carries its own magnitude interval and its own precision, which the
    // `C_QUANTITY_ITEM` it came from states.
    let form = from_opt14(GRID);
    let FieldKind::Quantity(ref quantity) = *kind(&form, "at0034") else {
        panic!("at0034 is not a quantity");
    };
    assert_eq!(
        quantity.property.as_ref().map(|code| code.code.as_str()),
        Some("125")
    );
    assert_eq!(quantity.units.len(), 2);
    let millimetres = quantity.units.first().expect("the first unit");
    let kilopascals = quantity.units.get(1).expect("the second unit");
    assert_eq!(millimetres.units, "mm[Hg]");
    assert_eq!(kilopascals.units, "kPa");
    assert_eq!(
        millimetres
            .magnitude
            .as_ref()
            .and_then(|range| range.maximum),
        Some(400.0)
    );
    assert_eq!(
        kilopascals
            .magnitude
            .as_ref()
            .and_then(|range| range.maximum),
        Some(53.3)
    );
    assert_eq!(
        millimetres
            .decimals
            .as_ref()
            .and_then(|range| range.maximum),
        Some(0)
    );
    assert_eq!(
        kilopascals
            .decimals
            .as_ref()
            .and_then(|range| range.maximum),
        Some(1)
    );
}

#[test]
fn a_local_code_list_carries_the_rubrics_the_archetype_states() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 7.3.2 makes
    // `ARCHETYPE_TERM.text` mandatory per code per language, so a local set
    // needs no terminology server.
    let form = from_opt14(OBSERVATION);
    let FieldKind::Coded(ref coded) = *kind(&form, "at0003") else {
        panic!("at0003 is not a coded field");
    };
    let ValueSet::Enumerated(ref set) = coded.value_set else {
        panic!("the set is not enumerated");
    };
    assert!(!set.needs_display_lookup);
    let labels: Vec<Option<&str>> = set
        .options
        .iter()
        .map(|option| option.label.get(&english()))
        .collect();
    assert_eq!(labels, [Some("Sitting"), Some("Standing")]);
}

#[test]
fn a_label_is_the_short_rubric_and_the_help_text_is_the_long_one() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 7.3.2 gives a term a `text`
    // and a `description`; which is which for a form is our own design.
    let form = from_opt14(OBSERVATION);
    let pressure = field(&form, "at0002");
    assert_eq!(pressure.label.get(&english()), Some("Pressure"));
    assert_eq!(pressure.help.get(&english()), Some("A synthetic pressure."));
}

#[test]
fn a_set_the_template_names_but_does_not_enumerate_is_expanded_at_render_time() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.3.14: a `CONSTRAINT_REF`
    // names an entry the archetype resolves through its constraint bindings.
    let form = from_opt14(XSD_ONLY);
    let FieldKind::Coded(ref by_uri) = *kind(&form, "at0001") else {
        panic!("at0001 is not a coded field");
    };
    let ValueSet::Expansion(ExpansionSource::ReferenceSet { ref uri }) = by_uri.value_set else {
        panic!("the reference set is not marked for expansion");
    };
    assert!(uri.contains("synthetic-positions"), "{uri}");

    let FieldKind::Coded(ref by_code) = *kind(&form, "at0002") else {
        panic!("at0002 is not a coded field");
    };
    let ValueSet::Expansion(ExpansionSource::ConstraintCode {
        ref code,
        ref bindings,
        ..
    }) = by_code.value_set
    else {
        panic!("the constraint reference is not marked for expansion");
    };
    assert_eq!(code.as_str(), "ac0001");
    assert_eq!(bindings.len(), 1);
}

#[test]
fn a_state_machine_carries_the_transitions_out_of_every_state() {
    // openEHR RM Release-1.1.0 data_types.html section 4.2.3. `C_DV_STATE` is
    // declared only in the openEHR ITS-XML 2.0.0 OpenehrProfile.xsd.
    let form = from_opt14(XSD_ONLY);
    let FieldKind::State(ref state) = *kind(&form, "at0003") else {
        panic!("at0003 is not a state");
    };
    let names: Vec<&str> = state
        .states
        .iter()
        .map(|option| option.name.as_str())
        .collect();
    assert_eq!(names, ["planned", "completed"]);
    let planned = state.states.first().expect("the first state");
    assert_eq!(
        planned
            .transitions
            .first()
            .and_then(|transition| transition.next_state.as_deref()),
        Some("completed")
    );
    assert!(state.states.get(1).is_some_and(|last| last.is_terminal));
}

#[test]
fn an_ordinal_keeps_list_order_and_stores_the_symbols_code() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.4: the integer is
    // the score, the symbol is what a composition stores, and the list is
    // ordered.
    let form = from_adl2(ADL2_EXTRAS);
    let FieldKind::Ordinal(ref ordinal) = *kind(&form, "id3") else {
        panic!("id3 is not an ordinal");
    };
    let scores: Vec<f64> = ordinal.options.iter().map(|option| option.score).collect();
    assert_eq!(scores, [0.0, 1.0]);
    let codes: Vec<&str> = ordinal
        .options
        .iter()
        .map(|option| option.symbol.code.as_str())
        .collect();
    assert_eq!(codes, ["at1", "at2"]);
    let labels: Vec<Option<&str>> = ordinal
        .options
        .iter()
        .map(|option| option.label.get(&english()))
        .collect();
    assert_eq!(labels, [Some("Mild"), Some("Severe")]);
}

#[test]
fn a_scale_is_an_ordinal_with_a_real_score() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.5: a `DV_SCALE` is
    // a `DV_ORDINAL` whose value is a real. The openEHR ITS-XML 2.0.0
    // `OpenehrProfile.xsd` declares no `C_DV_SCALE`, so only an ADL 2 template
    // can state one.
    let form = from_adl2(ADL2_EXTRAS);
    let scale = field(&form, "id12");
    assert_eq!(scale.rm_type.as_str(), "DV_SCALE");
    let FieldKind::Ordinal(ref ordinal) = scale.kind else {
        panic!("id12 is not an ordinal");
    };
    let scores: Vec<f64> = ordinal.options.iter().map(|option| option.score).collect();
    assert_eq!(scores, [0.0, 0.5]);
}

#[test]
fn an_adl2_value_set_the_archetype_enumerates_needs_no_terminology_server() {
    // openEHR AM Release-2.3.0 ADL2.html section 7.13.5.1: an `ac`-code names
    // a value set, and the archetype's terminology may list its members.
    let form = from_adl2(ADL2_OBSERVATION);
    let FieldKind::Coded(ref position) = *kind(&form, "id5") else {
        panic!("id5 is not a coded field");
    };
    let ValueSet::Enumerated(ref set) = position.value_set else {
        panic!("the locally enumerated set was not resolved");
    };
    let members: Vec<&str> = set
        .options
        .iter()
        .map(|option| option.code.code.as_str())
        .collect();
    assert_eq!(members, ["at1", "at2"]);
    assert!(!set.needs_display_lookup);
}

#[test]
fn an_adl2_value_set_the_archetype_only_binds_is_expanded_at_render_time() {
    let form = from_adl2(ADL2_EXTRAS);
    let FieldKind::Coded(ref coded) = *kind(&form, "id5") else {
        panic!("id5 is not a coded field");
    };
    assert!(
        matches!(
            coded.value_set,
            ValueSet::Expansion(ExpansionSource::ConstraintCode { .. })
        ),
        "{:?}",
        coded.value_set
    );
}

#[test]
fn an_adl2_attribute_tuple_of_one_row_is_a_constraint_per_attribute() {
    // openEHR AM Release-2.3.0 AOM2.html section 4.5.25: a `C_ATTRIBUTE_TUPLE`
    // carries one row per admitted combination, and a single row says exactly
    // what a constraint per attribute says.
    let form = from_adl2(ADL2_EXTRAS);
    let FieldKind::Proportion(ref proportion) = *kind(&form, "id10") else {
        panic!("id10 is not a proportion");
    };
    assert_eq!(
        proportion
            .numerator
            .ranges
            .first()
            .and_then(|range| range.maximum),
        Some(100.0)
    );
    assert_eq!(
        proportion
            .denominator
            .ranges
            .first()
            .and_then(|range| range.minimum),
        Some(100.0)
    );
}

#[test]
fn an_attribute_the_derivation_does_not_model_is_refused_by_name() {
    let error = refusal(UNMODELLED_ATTRIBUTE);
    let DeriveError::UnmodelledAttribute {
        ref rm_type,
        ref attribute,
        ..
    } = error
    else {
        panic!("the refusal is {error}");
    };
    assert_eq!(rm_type, "DV_TEXT");
    assert_eq!(attribute, "formatting");
}

#[test]
fn a_data_value_the_derivation_does_not_model_is_refused_by_name() {
    let error = refusal(UNMODELLED_DATA_VALUE);
    let DeriveError::UnmodelledDataValue { ref rm_type, .. } = error else {
        panic!("the refusal is {error}");
    };
    assert_eq!(rm_type, "DV_ENCAPSULATED");
}

#[test]
fn the_root_group_carries_the_archetype_and_the_template_identity() {
    let form = from_opt14(OBSERVATION);
    assert_eq!(form.format_version, FORMAT_VERSION);
    assert_eq!(
        form.template_id.as_str(),
        "openEHR-EHR-OBSERVATION.ferro_test.v1.0.0"
    );
    assert_eq!(form.default_language, english());
    assert_eq!(form.languages, [english()]);
    assert_eq!(
        form.root.archetype_id.as_ref().map(ArchetypeId::as_str),
        Some("openEHR-EHR-OBSERVATION.ferro_test.v1.0.0")
    );
    let tree = form
        .groups()
        .find(|group| group.rm_type.as_str() == "ITEM_TREE")
        .expect("the item tree is a group");
    assert_eq!(tree.shape, GroupShape::Tree);
}

#[test]
fn an_ordered_container_marks_the_items_it_holds_as_ordered() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.3.5: `CARDINALITY`
    // carries `is_ordered`, and the fixture's `items` attribute states it.
    let form = from_opt14(OBSERVATION);
    assert!(
        form.fields().all(|field| field.is_ordered),
        "the fixture holds every element in an ordered container"
    );
    assert!(form.fields().all(|field| !field.is_unique));
    // The item tree sits under a single-valued attribute, which has no
    // cardinality at all.
    let tree = form
        .groups()
        .find(|group| group.rm_type.as_str() == "ITEM_TREE")
        .expect("the item tree is a group");
    assert!(!tree.is_ordered);
}

#[test]
fn every_template_the_reader_reads_derives_a_form() {
    let mut derived = 0_usize;
    let mut refused = Vec::new();
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        match derive::form(&template) {
            Ok(_) => derived += 1,
            Err(error) => refused.push(format!("{}: {error}", path.display())),
        }
    }
    assert!(refused.is_empty(), "{refused:#?}");
    // A ratchet: the reader reads 121 of the 123 committed templates, and
    // every one of them derives.
    assert_eq!(derived, 121);
}

#[test]
fn the_pack_derives_the_field_kinds_it_carries() {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut fields = 0_usize;
    let mut groups = 0_usize;
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let form = derive::form(&template).expect("every template derives");
        groups += form.groups().count();
        for field in form.fields() {
            fields += 1;
            *kinds.entry(field.kind.name()).or_default() += 1;
        }
    }
    let expected: BTreeMap<&str, usize> = [
        ("boolean", 24),
        ("choice", 126),
        ("coded", 506),
        ("count", 115),
        ("date", 10),
        ("date-time", 202),
        ("duration", 30),
        ("identifier", 67),
        ("multimedia", 7),
        ("ordinal", 1),
        ("parsable", 5),
        ("proportion", 24),
        ("quantity", 111),
        ("text", 1394),
        ("uri", 36),
    ]
    .into_iter()
    .collect();
    assert_eq!(kinds, expected);
    assert_eq!(fields, 2658);
    assert_eq!(groups, 1789);
}

#[test]
fn an_open_slot_is_recorded_and_never_rendered() {
    // openEHR AM Release-2.3.0 OPT2.html section 3.3 removes only closed
    // slots, so an open one survives into an operational template holding
    // undetermined content.
    let mut slots = 0_usize;
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let form = derive::form(&template).expect("every template derives");
        for content in form.undetermined() {
            if matches!(content.reason, UndeterminedReason::OpenSlot { .. }) {
                slots += 1;
                assert!(
                    form.fields().all(|field| field.key != content.key),
                    "{} rendered an open slot",
                    path.display()
                );
            }
        }
    }
    assert_eq!(slots, 1195);
}

#[test]
fn a_null_flavour_affordance_exists_wherever_occurrences_permit_omission() {
    // openEHR RM Release-1.1.0 data_structures.html section 5.2.3: a field has
    // either a value or a null flavour, never both and never neither.
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let form = derive::form(&template).expect("every template derives");
        for field in form.fields() {
            if field.null_flavour.is_offered {
                assert!(
                    field.occurrences.is_optional(),
                    "{} offers a null flavour on a mandatory field",
                    path.display()
                );
                assert_eq!(field.null_flavour.options.len(), 4);
                assert!(field.null_flavour.accepts_reason);
            } else {
                assert!(field.null_flavour.options.is_empty());
                assert!(!field.null_flavour.accepts_reason);
            }
        }
    }
}

#[test]
fn the_same_template_derives_the_same_document_every_time() {
    // The serialisation is byte-deterministic, so a recompile of an unchanged
    // template produces an identical document.
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let first = serde_json::to_string(&derive::form(&template).expect("derives"))
            .expect("a form serializes");
        let second = serde_json::to_string(&derive::form(&template).expect("derives"))
            .expect("a form serializes");
        assert_eq!(first, second, "{}", path.display());
    }
}

#[test]
fn a_form_round_trips_through_its_serialisation() {
    let form = from_opt14(GRID);
    let json = serde_json::to_string(&form).expect("a form serializes");
    let back: FormDefinition = serde_json::from_str(&json).expect("a form deserializes");
    assert_eq!(back, form);
    assert_eq!(
        serde_json::to_string(&back).expect("a form serializes"),
        json
    );
}

#[test]
fn a_key_is_positional_exactly_where_a_sibling_ties() {
    // The flag used to mean "this step carries no identifying code", which is
    // true of every bare attribute step and disjoint from an actual tie (#67).
    // It now means what architecture section 6.2 decided.
    let mut positional = 0_usize;
    let mut positional_without_code = 0_usize;
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let form = derive::form(&template).expect("every template derives");
        let keys = form
            .groups()
            .map(|group| &group.key)
            .chain(form.fields().map(|field| &field.key));
        for key in keys {
            if key.is_positional {
                positional += 1;
                let last = key.steps.last().expect("a key has a step");
                if last.node_id.is_none() && last.archetype_id.is_none() {
                    positional_without_code += 1;
                }
            }
        }
    }
    assert_eq!(positional, 102, "the tie count over the committed pack");
    // Every tie in this pack is between siblings that carry a node id and
    // differ in nothing else, which is why the old proxy found none of them.
    assert_eq!(positional_without_code, 0);
}

#[test]
fn a_printed_key_tells_apart_every_key_that_does_not_match() {
    // The property #83 was filed for. A printed key is what a replay report
    // shows a person when it asks them to act on a difference, so two keys
    // that print alike must be the same key.
    let mut collisions = Vec::new();
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let form = derive::form(&template).expect("every template derives");
        let mut seen: BTreeMap<String, NodeKey> = BTreeMap::new();
        let keys = form
            .groups()
            .map(|group| group.key.clone())
            .chain(form.fields().map(|field| field.key.clone()));
        for key in keys {
            let printed = key.to_string();
            match seen.get(&printed) {
                Some(held) if *held != key => {
                    collisions.push(format!("{}: {printed}", path.display()));
                }
                _ => {
                    seen.insert(printed, key);
                }
            }
        }
    }
    assert!(
        collisions.is_empty(),
        "{} distinct keys print alike:\n  {}",
        collisions.len(),
        collisions.join("\n  ")
    );
}

#[test]
fn a_magnitude_stated_as_an_enumeration_is_refused() {
    // openEHR RM Release-1.1.0 `data_types.html` section 6.2.8 gives one
    // permitted unit one magnitude interval, so a `QuantityUnitOption` has
    // nowhere to carry an enumeration. Reading only the range would leave the
    // field admitting every magnitude the unit allows, which the template
    // refuses (#86). ADL 1.4 states the enumeration: `C_REAL` carries a `list`
    // beside its single `range` (openEHR AM Release-2.3.0 `AOM1.4.html`
    // section 6.2.5).
    match refusal(MAGNITUDE_LIST) {
        DeriveError::UnrepresentableUnitBound {
            attribute,
            units,
            found,
            ..
        } => {
            assert_eq!(attribute, "magnitude");
            assert_eq!(units, "mL");
            assert_eq!(found, "an enumeration of 3 values");
        }
        other => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_magnitude_stated_as_several_ranges_is_refused() {
    // AOM 2 types `C_REAL.constraint` as a list of intervals (openEHR AM
    // Release-2.3.0 `AOM2.html` section 4.5.15), so an ADL 2 archetype states
    // the same overconstraint as several ranges, and a stated single value is
    // one point interval. `QuantityUnitOption` carries one range, so more than
    // one is refused rather than trimmed to the first (#86).
    match refusal_adl2(ADL2_UNIT_BOUNDS) {
        DeriveError::UnrepresentableUnitBound {
            attribute,
            units,
            found,
            ..
        } => {
            assert_eq!(attribute, "magnitude");
            assert_eq!(units, "mL");
            assert_eq!(found, "3 separate ranges");
        }
        other => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_reference_band_on_an_interval_end_is_refused() {
    // Both ends of a `DV_INTERVAL` are `DV_ORDERED` (openEHR RM Release-1.1.0
    // `data_types.html` section 6.2.2), so an end may state a band.
    // `IntervalField` has nowhere to put one, and it is refused rather than
    // dropped (#87).
    match refusal_adl2(ADL2_INTERVAL_BAND) {
        DeriveError::UnmodelledAttribute {
            rm_type, attribute, ..
        } => {
            assert_eq!(rm_type, "DV_COUNT");
            assert_eq!(attribute, "normal_range");
        }
        other => panic!("the refusal is {other:?}"),
    }
}
