// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a clinician fills, written as a node's `inputs`.
//!
//! The inverse of [`crate::kind::read`], row for row. A field the web template
//! format has no shape for is written as the node without inputs, and the
//! caller is told which fact did not fit so the loss is visible rather than
//! silent.

use ferrochart_form::field::{
    BooleanField, CountField, DateField, DateTimeField, DurationField, FieldKind, IdentifierField,
    MultimediaField, OrdinalField, ParsableField, ProportionField, ProportionKind, QuantityField,
    TextField, TimeField, UriField,
};
use ferrochart_form::text::Localized;
use ferrochart_form::value::{CodedOption, ValueSet};
use serde_json::{Map, Value};

use crate::error::WriteError;
use crate::json;
use crate::kind::read::{DURATION_SUFFIXES, PROPORTION_KIND_NAMES};
use crate::kind::{InputType, range, temporal};

/// What a field's inputs came out as, with the facts the format could not
/// carry.
pub(crate) struct WrittenInputs {
    /// The `inputs` array.
    pub(crate) inputs: Vec<Value>,
    /// The `proportionTypes` array, for a proportion field.
    pub(crate) proportion_types: Vec<Value>,
    /// The facts the web template format has no member for.
    pub(crate) not_carried: Vec<&'static str>,
}

/// One input, with the members a web template states in the order it states
/// them.
fn input(input_type: InputType, suffix: Option<&str>) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(suffix) = suffix {
        out.insert("suffix".to_owned(), Value::String(suffix.to_owned()));
    }
    out.insert(
        "type".to_owned(),
        Value::String(input_type.name().to_owned()),
    );
    out
}

fn with_validation(mut entry: Map<String, Value>, validation: Map<String, Value>) -> Value {
    if !validation.is_empty() {
        entry.insert("validation".to_owned(), Value::Object(validation));
    }
    Value::Object(entry)
}

fn pattern_validation(pattern: Option<String>) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(pattern) = pattern {
        out.insert("pattern".to_owned(), Value::String(pattern));
    }
    out
}

fn text_input(field: &TextField, suffix: Option<&str>) -> Value {
    let mut entry = input(InputType::Text, suffix);
    if !field.options.is_empty() {
        let list: Vec<Value> = field
            .options
            .iter()
            .map(|option| {
                let mut out = Map::new();
                out.insert("value".to_owned(), Value::String(option.clone()));
                out.insert("label".to_owned(), Value::String(option.clone()));
                Value::Object(out)
            })
            .collect();
        entry.insert("list".to_owned(), Value::Array(list));
        entry.insert("listOpen".to_owned(), Value::Bool(!field.options_closed));
    }
    with_validation(entry, pattern_validation(field.patterns.first().cloned()))
}

fn coded_entry(option: &CodedOption, default_language: &str) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("value".to_owned(), Value::String(option.code.code.clone()));
    if let Some(label) = label_in(&option.label, default_language) {
        out.insert("label".to_owned(), Value::String(label.to_owned()));
    }
    if let Some(labels) = json::localized_value(&option.label) {
        out.insert("localizedLabels".to_owned(), labels);
    }
    if let Some(descriptions) = json::localized_value(&option.description) {
        out.insert("localizedDescriptions".to_owned(), descriptions);
    }
    out
}

/// The display text of `text` in the document's default language, falling back
/// to whatever language it is stated in.
fn label_in<'a>(text: &'a Localized, default_language: &str) -> Option<&'a str> {
    text.get(&ferrochart_form::ids::LanguageTag::new(
        default_language.to_owned(),
    ))
    .or_else(|| text.by_language.values().next().map(String::as_str))
}

/// The terminology member a coded input states, which is left out for the
/// archetype's own.
fn terminology_member(entry: &mut Map<String, Value>, terminology: &str) {
    if !terminology.is_empty() && terminology != "local" {
        entry.insert(
            "terminology".to_owned(),
            Value::String(terminology.to_owned()),
        );
    }
}

fn coded_inputs(field: &ferrochart_form::field::CodedField, default_language: &str) -> Vec<Value> {
    match field.value_set {
        ValueSet::Enumerated(ref set) => {
            let mut entry = input(InputType::CodedText, Some("code"));
            let list: Vec<Value> = set
                .options
                .iter()
                .map(|option| Value::Object(coded_entry(option, default_language)))
                .collect();
            entry.insert("list".to_owned(), Value::Array(list));
            terminology_member(&mut entry, set.terminology.as_str());
            vec![Value::Object(entry)]
        }
        ValueSet::OpenTerminology { ref terminology } => {
            open_coded_inputs(Some(terminology.as_str()))
        }
        _ => open_coded_inputs(None),
    }
}

/// The `code` and `value` pair a web template states for a coded value it does
/// not enumerate.
fn open_coded_inputs(terminology: Option<&str>) -> Vec<Value> {
    ["code", "value"]
        .into_iter()
        .map(|suffix| {
            let mut entry = input(InputType::Text, Some(suffix));
            if let Some(terminology) = terminology {
                terminology_member(&mut entry, terminology);
            }
            Value::Object(entry)
        })
        .collect()
}

fn ordinal_inputs(field: &OrdinalField, default_language: &str) -> Vec<Value> {
    let mut entry = input(InputType::CodedText, None);
    let mut terminology = None;
    let list: Vec<Value> = field
        .options
        .iter()
        .map(|option| {
            terminology = Some(option.symbol.terminology.as_str());
            let mut out = coded_entry(
                &CodedOption {
                    code: option.symbol.clone(),
                    label: option.label.clone(),
                    description: option.description.clone(),
                },
                default_language,
            );
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a web template states an ordinal as a whole number, and a score that is not one is written as the scale it is"
            )]
            if option.score.fract() == 0.0 {
                out.insert("ordinal".to_owned(), Value::from(option.score as i64));
            } else {
                out.insert("scale".to_owned(), Value::from(option.score));
            }
            Value::Object(out)
        })
        .collect();
    entry.insert("list".to_owned(), Value::Array(list));
    if let Some(terminology) = terminology {
        terminology_member(&mut entry, terminology);
    }
    vec![Value::Object(entry)]
}

fn boolean_inputs(field: BooleanField) -> Vec<Value> {
    let mut entry = input(InputType::Boolean, None);
    let fixed = match (field.true_allowed, field.false_allowed) {
        (true, false) => Some("true"),
        (false, true) => Some("false"),
        _ => None,
    };
    if let Some(fixed) = fixed {
        let mut option = Map::new();
        option.insert("value".to_owned(), Value::String(fixed.to_owned()));
        option.insert("label".to_owned(), Value::String(fixed.to_owned()));
        entry.insert("list".to_owned(), Value::Array(vec![Value::Object(option)]));
    }
    vec![Value::Object(entry)]
}

fn quantity_inputs(field: &QuantityField) -> Vec<Value> {
    let magnitude = input(InputType::Decimal, Some("magnitude"));
    let mut units = input(InputType::CodedText, Some("unit"));
    let list: Vec<Value> = field
        .units
        .iter()
        .map(|unit| {
            let mut out = Map::new();
            out.insert("value".to_owned(), Value::String(unit.units.clone()));
            out.insert("label".to_owned(), Value::String(unit.units.clone()));
            let mut validation = Map::new();
            if let Some(range) = unit.magnitude.as_ref().and_then(range::real_value) {
                validation.insert("range".to_owned(), range);
            }
            if let Some(precision) = unit.decimals.as_ref().and_then(range::integer_value) {
                validation.insert("precision".to_owned(), precision);
            }
            with_validation(out, validation)
        })
        .collect();
    if !list.is_empty() {
        units.insert("list".to_owned(), Value::Array(list));
    }
    vec![Value::Object(magnitude), Value::Object(units)]
}

fn count_inputs(field: &CountField) -> Vec<Value> {
    let entry = input(InputType::Integer, None);
    let mut validation = Map::new();
    if let Some(range) = field.ranges.first().and_then(range::integer_value) {
        validation.insert("range".to_owned(), range);
    }
    vec![with_validation(entry, validation)]
}

fn proportion_inputs(
    field: &ProportionField,
    at: &str,
) -> Result<(Vec<Value>, Vec<Value>), WriteError> {
    let mut types = Vec::with_capacity(field.kinds.len());
    for kind in &field.kinds {
        let index = match *kind {
            ProportionKind::Ratio => 0_usize,
            ProportionKind::Unitary => 1,
            ProportionKind::Percent => 2,
            ProportionKind::Fraction => 3,
            ProportionKind::IntegerFraction => 4,
            ProportionKind::Other(code) => {
                return Err(WriteError::UnnameableProportionKind {
                    at: at.to_owned(),
                    kind: code.to_string(),
                });
            }
            _ => {
                return Err(WriteError::UnnameableProportionKind {
                    at: at.to_owned(),
                    kind: format!("{kind:?}"),
                });
            }
        };
        let name = PROPORTION_KIND_NAMES
            .get(index)
            .copied()
            .unwrap_or(PROPORTION_KIND_NAMES[0]);
        types.push(Value::String(name.to_owned()));
    }
    let input_type = if field.is_integral == Some(true) {
        InputType::Integer
    } else {
        InputType::Decimal
    };
    let part = |suffix: &str, real: &ferrochart_form::field::RealField| {
        let entry = input(input_type, Some(suffix));
        let mut validation = Map::new();
        if let Some(range) = real.ranges.first().and_then(range::real_value) {
            validation.insert("range".to_owned(), range);
        }
        with_validation(entry, validation)
    };
    Ok((
        vec![
            part("numerator", &field.numerator),
            part("denominator", &field.denominator),
        ],
        types,
    ))
}

fn temporal_inputs(
    input_type: InputType,
    pattern: String,
    ranges: &[ferrochart_form::range::Range<String>],
) -> Vec<Value> {
    let entry = input(input_type, None);
    let mut validation = pattern_validation(Some(pattern));
    if let Some(range) = ranges.first().and_then(range::text_value) {
        validation.insert("range".to_owned(), range);
    }
    vec![with_validation(entry, validation)]
}

fn duration_inputs(field: &DurationField) -> Vec<Value> {
    DURATION_SUFFIXES
        .iter()
        .filter(|(_, component)| field.components.contains(component))
        .map(|(suffix, _)| Value::Object(input(InputType::Integer, Some(suffix))))
        .collect()
}

fn identifier_inputs(field: &IdentifierField) -> Vec<Value> {
    vec![
        text_input(&field.id, Some("id")),
        text_input(&field.identifier_type, Some("type")),
        text_input(&field.issuer, Some("issuer")),
        text_input(&field.assigner, Some("assigner")),
    ]
}

fn parsable_inputs(field: &ParsableField) -> Vec<Value> {
    vec![
        text_input(&field.value, Some("value")),
        text_input(&field.formalism, Some("formalism")),
    ]
}

fn multimedia_inputs(field: &MultimediaField) -> Vec<Value> {
    let uri = field.uri.clone().unwrap_or_default();
    vec![text_input(&uri, None)]
}

fn uri_inputs(field: &UriField) -> Vec<Value> {
    let text = TextField {
        patterns: field.patterns.clone(),
        options: Vec::new(),
        options_closed: false,
    };
    vec![text_input(&text, None)]
}

fn date_inputs(field: &DateField) -> Vec<Value> {
    temporal_inputs(
        InputType::Date,
        temporal::date_pattern(field.month, field.day),
        &field.ranges,
    )
}

fn time_inputs(field: &TimeField) -> Vec<Value> {
    temporal_inputs(
        InputType::Time,
        temporal::time_pattern(field.minute, field.second),
        &field.ranges,
    )
}

fn date_time_inputs(field: &DateTimeField) -> Vec<Value> {
    temporal_inputs(
        InputType::Datetime,
        temporal::date_time_pattern((
            field.month,
            field.day,
            field.hour,
            field.minute,
            field.second,
        )),
        &field.ranges,
    )
}

/// The `inputs` and `proportionTypes` a field is stated as.
///
/// # Errors
/// [`WriteError::UnnameableProportionKind`] when a proportion field admits a
/// kind the Reference Model does not name, which a web template has no
/// spelling for.
pub(crate) fn inputs_of(
    kind: &FieldKind,
    default_language: &str,
    at: &str,
) -> Result<WrittenInputs, WriteError> {
    let mut not_carried = Vec::new();
    let mut proportion_types = Vec::new();
    let inputs = match *kind {
        FieldKind::Boolean(field) => boolean_inputs(field),
        FieldKind::Text(ref field) => vec![text_input(field, None)],
        FieldKind::Uri(ref field) => uri_inputs(field),
        FieldKind::Coded(ref field) => coded_inputs(field, default_language),
        FieldKind::Ordinal(ref field) => ordinal_inputs(field, default_language),
        FieldKind::Count(ref field) => count_inputs(field),
        FieldKind::Quantity(ref field) => {
            if field.property.is_some() {
                not_carried.push("the property a quantity measures");
            }
            quantity_inputs(field)
        }
        FieldKind::Proportion(ref field) => {
            let (inputs, types) = proportion_inputs(field, at)?;
            proportion_types = types;
            inputs
        }
        FieldKind::Date(ref field) => date_inputs(field),
        FieldKind::Time(ref field) => time_inputs(field),
        FieldKind::DateTime(ref field) => date_time_inputs(field),
        FieldKind::Duration(ref field) => duration_inputs(field),
        FieldKind::Identifier(ref field) => identifier_inputs(field),
        FieldKind::Multimedia(ref field) => multimedia_inputs(field),
        FieldKind::Parsable(ref field) => parsable_inputs(field),
        FieldKind::Interval(_) => {
            not_carried.push("the bounds of an interval");
            Vec::new()
        }
        FieldKind::State(_) => {
            not_carried.push("the states of a state machine");
            Vec::new()
        }
        FieldKind::Choice(_) => {
            not_carried.push("the alternatives of a choice");
            Vec::new()
        }
        _ => {
            not_carried.push("a field kind this format has no shape for");
            Vec::new()
        }
    };
    Ok(WrittenInputs {
        inputs,
        proportion_types,
        not_carried,
    })
}
