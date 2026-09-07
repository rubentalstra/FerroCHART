// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A node's `inputs` read as what a clinician fills.
//!
//! The row a node takes is chosen by the Reference Model class it states, and
//! the class decides which suffixes the inputs carry. A node whose inputs do
//! not have the shape its class gives them is refused rather than read into a
//! field that would admit more than the template does.

use ferrochart_form::field::{
    BooleanField, CountField, DateField, DateTimeField, DurationComponent, DurationField,
    FieldKind, IdentifierField, MultimediaField, OrdinalField, OrdinalOption, ParsableField,
    ProportionField, ProportionKind, QuantityField, QuantityUnitOption, RealField, TextField,
    TimeField, UriField,
};
use ferrochart_form::ids::TerminologyName;
use ferrochart_form::text::Localized;
use ferrochart_form::value::{Code, CodedOption, EnumeratedSet, ValueSet};

use crate::error::ReadError;
use crate::json;
use crate::kind::{InputType, ParsedInput, ParsedOption, base_rm_type, range, temporal};

/// The terminology a coded input draws from when it names none.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3 reserves `local`
/// for the codes an archetype defines itself, and the published
/// implementations leave the member out for those, so an unnamed terminology
/// is the archetype's own.
fn terminology_of(input: &ParsedInput<'_>) -> TerminologyName {
    input
        .terminology
        .map_or_else(ferrochart_form::ids::local_terminology, |name| {
            TerminologyName::new(name)
        })
}

/// The five `PROPORTION_KIND` names, in the integer order openEHR RM
/// Release-1.1.0 `data_types.html` section 6.2.11 assigns them.
pub(crate) const PROPORTION_KIND_NAMES: [&str; 5] = [
    "ratio",
    "unitary",
    "percent",
    "fraction",
    "integer_fraction",
];

/// The `DV_DURATION` slot each input suffix fills.
///
/// The per-suffix split is the published implementations' own; no openEHR
/// specification governs it. The slots themselves are the ISO 8601 designators
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.9 names.
pub(crate) const DURATION_SUFFIXES: [(&str, DurationComponent); 7] = [
    ("year", DurationComponent::Years),
    ("month", DurationComponent::Months),
    ("day", DurationComponent::Days),
    ("week", DurationComponent::Weeks),
    ("hour", DurationComponent::Hours),
    ("minute", DurationComponent::Minutes),
    ("second", DurationComponent::Seconds),
];

/// What the inputs of a node of class `rm_type` collect.
///
/// `None` where the form definition has no field for the class, which is the
/// case for the `PARTY_PROXY` family a web template states for a composition's
/// context. The node is then carried as content the form renders nothing for.
///
/// # Errors
/// [`ReadError::UnreadableInputs`] when the inputs do not have the shape the
/// class gives them, and [`ReadError::UnknownProportionType`] when a
/// proportion type is not one the Reference Model names.
pub(crate) fn field_kind(
    rm_type: &str,
    proportion_types: &[String],
    inputs: &[ParsedInput<'_>],
    at: &str,
) -> Result<Option<FieldKind>, ReadError> {
    let unreadable = |reason: &str| ReadError::UnreadableInputs {
        at: at.to_owned(),
        rm_type: rm_type.to_owned(),
        reason: reason.to_owned(),
    };
    let kind = match base_rm_type(rm_type) {
        "DV_TEXT" => {
            FieldKind::Text(text_field(single(inputs).ok_or_else(|| {
                unreadable("a text value is stated by exactly one input")
            })?))
        }
        "DV_URI" | "DV_EHR_URI" => FieldKind::Uri(UriField {
            patterns: patterns(
                single(inputs).ok_or_else(|| unreadable("a URI is stated by exactly one input"))?,
            ),
            required_scheme: (base_rm_type(rm_type) == "DV_EHR_URI").then(|| "ehr".to_owned()),
        }),
        "DV_MULTIMEDIA" => FieldKind::Multimedia(MultimediaField {
            // NOTE: RM Release-1.1.0 `data_types.html` section 9.2.2 gives
            // `media_type` a code list, and a web template states one text
            // input for the whole value, so no media type reaches here.
            media_types: ValueSet::Unconstrained,
            uri: single(inputs).map(text_field),
        }),
        "DV_CODED_TEXT" | "CODE_PHRASE" | "DV_STATE" => FieldKind::Coded(coded_field(inputs)),
        "DV_ORDINAL" | "DV_SCALE" => ordinal_or_coded(inputs, &unreadable)?,
        "DV_BOOLEAN" => FieldKind::Boolean(boolean_field(
            single(inputs).ok_or_else(|| unreadable("a boolean is stated by exactly one input"))?,
        )),
        "DV_COUNT" => FieldKind::Count(CountField {
            options: Vec::new(),
            ranges: range::integer(
                single(inputs)
                    .ok_or_else(|| unreadable("a count is stated by exactly one input"))?
                    .validation,
                "range",
            )
            .into_iter()
            .collect(),
        }),
        "DV_QUANTITY" => FieldKind::Quantity(quantity_field(inputs)),
        "DV_PROPORTION" => FieldKind::Proportion(proportion_field(proportion_types, inputs, at)?),
        "DV_DATE" => FieldKind::Date(date_field(inputs, &unreadable)?),
        "DV_TIME" => FieldKind::Time(time_field(inputs, &unreadable)?),
        "DV_DATE_TIME" => FieldKind::DateTime(date_time_field(inputs, &unreadable)?),
        "DV_DURATION" => FieldKind::Duration(duration_field(inputs)),
        "DV_IDENTIFIER" => FieldKind::Identifier(IdentifierField {
            issuer: by_suffix(inputs, "issuer").map_or_else(TextField::default, text_field),
            assigner: by_suffix(inputs, "assigner").map_or_else(TextField::default, text_field),
            id: by_suffix(inputs, "id").map_or_else(TextField::default, text_field),
            identifier_type: by_suffix(inputs, "type").map_or_else(TextField::default, text_field),
        }),
        "DV_PARSABLE" => FieldKind::Parsable(ParsableField {
            formalism: by_suffix(inputs, "formalism").map_or_else(TextField::default, text_field),
            value: by_suffix(inputs, "value").map_or_else(TextField::default, text_field),
        }),
        _ => return Ok(None),
    };
    Ok(Some(kind))
}

/// The suffix an open value set's unlisted value is filled through.
///
/// openEHR ITS-REST Release-1.1.0 `simplified_formats.html` section 4.7 gives
/// the `|other` suffix that meaning in the FLAT format, and the published
/// implementations state an input for it beside the listed one. It is not the
/// node's own value, so it never decides what the field collects.
pub(crate) const OTHER_SUFFIX: &str = "other";

/// The one input a node of this class states, where it states exactly one.
///
/// The `other` input is not counted: it describes the escape from a value set
/// rather than the value.
fn single<'a, 'b>(inputs: &'a [ParsedInput<'b>]) -> Option<&'a ParsedInput<'b>> {
    let mut stated = inputs
        .iter()
        .filter(|input| input.suffix != Some(OTHER_SUFFIX));
    let only = stated.next()?;
    stated.next().is_none().then_some(only)
}

/// The input filling `suffix`.
fn by_suffix<'a, 'b>(inputs: &'a [ParsedInput<'b>], suffix: &str) -> Option<&'a ParsedInput<'b>> {
    inputs
        .iter()
        .find(|input| input.suffix.is_some_and(|stated| stated == suffix))
}

fn patterns(input: &ParsedInput<'_>) -> Vec<String> {
    input
        .validation
        .and_then(|validation| validation.get("pattern"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .into_iter()
        .collect()
}

fn text_field(input: &ParsedInput<'_>) -> TextField {
    TextField {
        patterns: patterns(input),
        options: input
            .list
            .iter()
            .map(|option| option.value.to_owned())
            .collect(),
        options_closed: !input.list_open.unwrap_or(false),
    }
}

fn coded_option(option: &ParsedOption<'_>, terminology: &TerminologyName) -> CodedOption {
    let mut label = option
        .localized_labels
        .map(localized_of)
        .unwrap_or_default();
    if label.is_empty()
        && let Some(stated) = option.label
    {
        label.insert(ferrochart_form::ids::LanguageTag::new(""), stated);
    }
    CodedOption {
        code: Code::new(terminology.clone(), option.value),
        label,
        description: option
            .localized_descriptions
            .map(localized_of)
            .unwrap_or_default(),
    }
}

/// A per-language object as display text, ignoring a member that is not text.
fn localized_of(map: &serde_json::Map<String, serde_json::Value>) -> Localized {
    let mut text = Localized::empty();
    for (language, value) in map {
        if let Some(stated) = value.as_str() {
            text.insert(
                ferrochart_form::ids::LanguageTag::new(language.clone()),
                stated,
            );
        }
    }
    text
}

fn coded_field(inputs: &[ParsedInput<'_>]) -> ferrochart_form::field::CodedField {
    let listed = inputs.iter().find(|input| !input.list.is_empty());
    let value_set = match listed {
        Some(input) => {
            let terminology = terminology_of(input);
            let options: Vec<CodedOption> = input
                .list
                .iter()
                .map(|option| coded_option(option, &terminology))
                .collect();
            let needs_display_lookup = options.iter().any(|option| option.label.is_empty());
            ValueSet::Enumerated(EnumeratedSet {
                terminology,
                options,
                needs_display_lookup,
            })
        }
        None => match inputs.iter().find_map(|input| input.terminology) {
            Some(name) => ValueSet::OpenTerminology {
                terminology: TerminologyName::new(name),
            },
            None => ValueSet::Unconstrained,
        },
    };
    ferrochart_form::field::CodedField {
        value_set,
        // NOTE: AM Release-2.3.0 `AOM2.html` section 4.5.10 carries binding
        // strictness in `constraint_status`, and a web template states no
        // member for it.
        strictness: None,
        rubric: None,
    }
}

/// An ordinal or scale field, or a coded one where the document carries no
/// score.
///
/// A web template states a score per option for the constrainer classes that
/// have one. The generic constraint form carries the symbol codes and no
/// numbers at all, and an ordinal option of the form definition carries a
/// mandatory score, so a scoreless list is read as a selection over the
/// symbols rather than as an ordinal with invented scores.
fn ordinal_or_coded(
    inputs: &[ParsedInput<'_>],
    unreadable: &dyn Fn(&str) -> ReadError,
) -> Result<FieldKind, ReadError> {
    let input =
        single(inputs).ok_or_else(|| unreadable("an ordinal is stated by exactly one input"))?;
    if input.list.is_empty()
        || input
            .list
            .iter()
            .any(|option| option.ordinal.is_none() && option.scale.is_none())
    {
        return Ok(FieldKind::Coded(coded_field(inputs)));
    }
    let terminology = terminology_of(input);
    let options = input
        .list
        .iter()
        .map(|option| {
            let coded = coded_option(option, &terminology);
            OrdinalOption {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "an ordinal score is a small integer and the form definition holds every score as a real, so the two classes share one option type"
                )]
                score: option
                    .scale
                    .unwrap_or_else(|| option.ordinal.unwrap_or_default() as f64),
                symbol: coded.code,
                label: coded.label,
                description: coded.description,
            }
        })
        .collect();
    Ok(FieldKind::Ordinal(OrdinalField { options }))
}

fn boolean_field(input: &ParsedInput<'_>) -> BooleanField {
    let listed = |wanted: &str| {
        input
            .list
            .iter()
            .any(|option| option.value.eq_ignore_ascii_case(wanted))
    };
    if input.list.is_empty() {
        return BooleanField {
            true_allowed: true,
            false_allowed: true,
        };
    }
    BooleanField {
        true_allowed: listed("true"),
        false_allowed: listed("false"),
    }
}

fn quantity_field(inputs: &[ParsedInput<'_>]) -> QuantityField {
    let units = by_suffix(inputs, "unit").map_or_else(Vec::new, |input| {
        input
            .list
            .iter()
            .map(|option| QuantityUnitOption {
                units: option.value.to_owned(),
                magnitude: range::real(option.validation, "range"),
                decimals: range::integer(option.validation, "precision"),
            })
            .collect()
    });
    QuantityField {
        // NOTE: RM Release-1.1.0 `data_types.html` section 6.2.8 names the
        // measured property on the constraint, and a web template states no
        // member for it.
        property: None,
        units,
    }
}

fn proportion_field(
    proportion_types: &[String],
    inputs: &[ParsedInput<'_>],
    at: &str,
) -> Result<ProportionField, ReadError> {
    let mut kinds = Vec::new();
    for stated in proportion_types {
        let code = PROPORTION_KIND_NAMES
            .iter()
            .position(|name| *name == stated.as_str())
            .ok_or_else(|| ReadError::UnknownProportionType {
                at: at.to_owned(),
                found: stated.clone(),
            })?;
        kinds.push(ProportionKind::from_code(i64::try_from(code).unwrap_or(0)));
    }
    let part = |suffix: &str| RealField {
        options: Vec::new(),
        ranges: by_suffix(inputs, suffix)
            .and_then(|input| range::real(input.validation, "range"))
            .into_iter()
            .collect(),
    };
    let is_integral = by_suffix(inputs, "numerator")
        .map(|input| input.input_type == InputType::Integer)
        .and_then(|integral| integral.then_some(true));
    Ok(ProportionField {
        kinds,
        numerator: part("numerator"),
        denominator: part("denominator"),
        decimals: CountField::default(),
        is_integral,
    })
}

/// The pattern the one temporal input of a node states.
fn temporal_pattern<'a>(
    inputs: &'a [ParsedInput<'a>],
    unreadable: &dyn Fn(&str) -> ReadError,
) -> Result<(&'a ParsedInput<'a>, Option<&'a str>), ReadError> {
    let input = single(inputs)
        .ok_or_else(|| unreadable("a temporal value is stated by exactly one input"))?;
    let pattern = input
        .validation
        .and_then(|validation| validation.get("pattern"))
        .and_then(serde_json::Value::as_str);
    Ok((input, pattern))
}

fn date_field(
    inputs: &[ParsedInput<'_>],
    unreadable: &dyn Fn(&str) -> ReadError,
) -> Result<DateField, ReadError> {
    let (input, pattern) = temporal_pattern(inputs, unreadable)?;
    let (month, day) = temporal::date(pattern);
    Ok(DateField {
        month,
        day,
        ranges: range::text(input.validation, "range").into_iter().collect(),
    })
}

fn time_field(
    inputs: &[ParsedInput<'_>],
    unreadable: &dyn Fn(&str) -> ReadError,
) -> Result<TimeField, ReadError> {
    let (input, pattern) = temporal_pattern(inputs, unreadable)?;
    let (minute, second) = temporal::time(pattern);
    Ok(TimeField {
        minute,
        second,
        // NOTE: AM Release-2.3.0 `AOM1.4.html` section 6.2.7 carries
        // `timezone_validity` on the constraint, and a web template states no
        // member for it, so a timezone is neither required nor forbidden.
        timezone: ferrochart_form::field::ComponentValidity::Optional,
        ranges: range::text(input.validation, "range").into_iter().collect(),
    })
}

fn date_time_field(
    inputs: &[ParsedInput<'_>],
    unreadable: &dyn Fn(&str) -> ReadError,
) -> Result<DateTimeField, ReadError> {
    let (input, pattern) = temporal_pattern(inputs, unreadable)?;
    let (month, day, hour, minute, second) = temporal::date_time(pattern);
    Ok(DateTimeField {
        month,
        day,
        hour,
        minute,
        second,
        timezone: ferrochart_form::field::ComponentValidity::Optional,
        ranges: range::text(input.validation, "range").into_iter().collect(),
    })
}

fn duration_field(inputs: &[ParsedInput<'_>]) -> DurationField {
    let components = DURATION_SUFFIXES
        .iter()
        .filter(|(suffix, _)| by_suffix(inputs, suffix).is_some())
        .map(|&(_, component)| component)
        .collect();
    DurationField {
        components,
        // NOTE: AM Release-2.3.0 `AOM1.4.html` section 6.2.9 carries a
        // duration range on the constraint, and a web template splits the
        // value into per-slot inputs that state none.
        ranges: Vec::new(),
    }
}

/// The `inputs` array of a node, read.
///
/// # Errors
/// [`ReadError`] when an entry is not an object, states no `type`, or states a
/// type this format does not define.
pub(crate) fn parse_inputs<'a>(
    values: &'a [serde_json::Value],
    at: &str,
) -> Result<Vec<ParsedInput<'a>>, ReadError> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let obj = json::object(value, at)?;
        let name = json::required_str(obj, "type", at)?;
        let input_type = InputType::from_name(name).ok_or_else(|| ReadError::UnknownInputType {
            at: at.to_owned(),
            found: name.to_owned(),
        })?;
        let mut list = Vec::new();
        for entry in json::optional_array(obj, "list", at)? {
            let entry = json::object(entry, at)?;
            list.push(ParsedOption {
                value: json::required_str(entry, "value", at)?,
                label: json::optional_str(entry, "label", at)?,
                localized_labels: json::optional_object(entry, "localizedLabels", at)?,
                localized_descriptions: json::optional_object(entry, "localizedDescriptions", at)?,
                validation: json::optional_object(entry, "validation", at)?,
                ordinal: json::optional_i64(entry, "ordinal", at)?,
                scale: entry.get("scale").and_then(serde_json::Value::as_f64),
            });
        }
        out.push(ParsedInput {
            suffix: json::optional_str(obj, "suffix", at)?,
            input_type,
            list,
            list_open: json::optional_bool(obj, "listOpen", at)?,
            validation: json::optional_object(obj, "validation", at)?,
            terminology: json::optional_str(obj, "terminology", at)?,
        });
    }
    Ok(out)
}
