// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One entered value becomes one `DATA_VALUE`, and back again.
//!
//! All citations are openEHR RM Release-1.1.0 `data_types.html`. The optional
//! attributes every quantity class inherits from `DV_ORDERED` and
//! `DV_QUANTIFIED` are left absent: a reference band is display metadata the
//! form shows beside a value rather than something a clinician enters
//! (`docs/architecture.md` section 5), and `accuracy` and `magnitude_status`
//! are measurements about the measurement that no form field produces.

use ferrochart_form::key::NodeKey;
use openehr_rm::v1_2::data_types::basic::data_value::DataValue;
use openehr_rm::v1_2::data_types::basic::dv_boolean::DvBoolean;
use openehr_rm::v1_2::data_types::basic::dv_identifier::DvIdentifier;
use openehr_rm::v1_2::data_types::encapsulated::dv_parsable::DvParsable;
use openehr_rm::v1_2::data_types::quantity::date_time::dv_date::DvDate;
use openehr_rm::v1_2::data_types::quantity::date_time::dv_date_time::DvDateTime;
use openehr_rm::v1_2::data_types::quantity::date_time::dv_duration::DvDuration;
use openehr_rm::v1_2::data_types::quantity::date_time::dv_time::DvTime;
use openehr_rm::v1_2::data_types::quantity::dv_count::DvCount;
use openehr_rm::v1_2::data_types::quantity::dv_interval::DvInterval;
use openehr_rm::v1_2::data_types::quantity::dv_ordered::DvOrdered;
use openehr_rm::v1_2::data_types::quantity::dv_ordinal::DvOrdinal;
use openehr_rm::v1_2::data_types::quantity::dv_proportion::DvProportion;
use openehr_rm::v1_2::data_types::quantity::dv_quantity::DvQuantity;
use openehr_rm::v1_2::data_types::quantity::dv_scale::DvScale;
use openehr_rm::v1_2::data_types::text::dv_coded_text::DvCodedText;
use openehr_rm::v1_2::data_types::text::dv_text::{DvText, DvTextData};
use openehr_rm::v1_2::data_types::uri::dv_ehr_uri::DvEhrUri;
use openehr_rm::v1_2::data_types::uri::dv_uri::{DvUri, DvUriData};

use crate::envelope::code_phrase;
use crate::error::{BuildError, ReadError};
use crate::values::Datum;

/// The `PROPORTION_KIND` code for a unitary proportion.
///
/// Section 6.2.11. Named because `Unitary_validity` and `Percent_validity`
/// both key off it.
const PK_UNITARY: i64 = 1;

/// The `PROPORTION_KIND` code for a percentage.
const PK_PERCENT: i64 = 2;

/// The `PROPORTION_KIND` code for a fraction.
const PK_FRACTION: i64 = 3;

/// The `PROPORTION_KIND` code for an integer fraction.
const PK_INTEGER_FRACTION: i64 = 4;

/// The highest `PROPORTION_KIND` code section 6.2.11 defines.
const PK_MAX: i64 = 4;

/// Builds the `DATA_VALUE` for `datum`.
///
/// `rm_type` is the class the form definition derived the field to, which
/// decides the two cases the datum alone cannot: a `DV_URI` against a
/// `DV_EHR_URI`, and the element class of an interval.
///
/// # Errors
/// [`BuildError::Invariant`] when the value violates a Reference Model
/// invariant, naming the invariant, and [`BuildError::WrongDatum`] when the
/// datum does not fit the class.
pub fn build(key: &NodeKey, rm_type: &str, datum: &Datum) -> Result<DataValue, BuildError> {
    match *datum {
        Datum::Boolean(value) => Ok(DataValue::DvBoolean(DvBoolean { value })),
        Datum::Text(ref value) => Ok(DataValue::DvText(DvText::DvText(text(value)))),
        Datum::Coded {
            ref terminology,
            ref code,
            ref rubric,
        } => Ok(DataValue::DvText(DvText::DvCodedText(coded(
            terminology,
            code,
            rubric,
        )))),
        Datum::Ordinal {
            ref terminology,
            ref code,
            ref rubric,
            value,
        } => {
            let value = i32::try_from(value).map_err(|_| BuildError::Invariant {
                key: key.clone(),
                invariant: "DV_ORDINAL.value",
                detail: format!("{value} does not fit the Integer the class declares"),
            })?;
            Ok(DataValue::DvOrdinal(DvOrdinal {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: None,
                symbol: coded(terminology, code, rubric),
                value,
            }))
        }
        Datum::Scale {
            ref terminology,
            ref code,
            ref rubric,
            value,
        } => Ok(DataValue::DvScale(DvScale {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: None,
            symbol: coded(terminology, code, rubric),
            value,
        })),
        Datum::Count(magnitude) => Ok(DataValue::DvCount(count(magnitude))),
        Datum::Quantity {
            magnitude,
            ref units,
            precision,
        } => quantity(key, magnitude, units, precision),
        Datum::Proportion {
            numerator,
            denominator,
            kind,
            precision,
        } => proportion(key, numerator, denominator, kind, precision),
        Datum::Date(ref value) => Ok(DataValue::DvDate(date(value))),
        Datum::Time(ref value) => Ok(DataValue::DvTime(time(value))),
        Datum::DateTime(ref value) => Ok(DataValue::DvDateTime(date_time(value))),
        Datum::Duration(ref value) => Ok(DataValue::DvDuration(duration(value))),
        Datum::Identifier {
            ref id,
            ref issuer,
            ref assigner,
            ref identifier_type,
        } => identifier(
            key,
            id,
            issuer.as_ref(),
            assigner.as_ref(),
            identifier_type.as_ref(),
        ),
        Datum::Uri(ref value) => uri(key, rm_type, value),
        Datum::Parsable {
            ref value,
            ref formalism,
        } => parsable(key, value, formalism),
        Datum::Interval {
            ref lower,
            ref upper,
            lower_included,
            upper_included,
        } => interval(
            key,
            lower.as_deref(),
            upper.as_deref(),
            lower_included,
            upper_included,
        ),
    }
}

/// A `DV_QUANTITY`.
///
/// The Reference Model states no invariant on the class, so the one check is
/// the serialisation's: the canonical XSD types `units` as `NonEmptyString`.
fn quantity(
    key: &NodeKey,
    magnitude: f64,
    units: &str,
    precision: Option<i32>,
) -> Result<DataValue, BuildError> {
    if units.is_empty() {
        return Err(BuildError::Invariant {
            key: key.clone(),
            invariant: "DV_QUANTITY.units",
            detail: "the units are empty, and the canonical XSD types them NonEmptyString"
                .to_owned(),
        });
    }
    Ok(DataValue::DvQuantity(DvQuantity {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        accuracy_is_percent: None,
        magnitude,
        precision,
        units: units.to_owned(),
        units_system: None,
        units_display_name: None,
    }))
}

/// A `DV_IDENTIFIER`. Section 4.2.4: `Id_valid: not id.is_empty`.
fn identifier(
    key: &NodeKey,
    id: &str,
    issuer: Option<&String>,
    assigner: Option<&String>,
    identifier_type: Option<&String>,
) -> Result<DataValue, BuildError> {
    if id.is_empty() {
        return Err(BuildError::Invariant {
            key: key.clone(),
            invariant: "DV_IDENTIFIER.Id_valid",
            detail: "the identifier is empty".to_owned(),
        });
    }
    Ok(DataValue::DvIdentifier(DvIdentifier {
        issuer: issuer.cloned(),
        assigner: assigner.cloned(),
        id: id.to_owned(),
        r#type: identifier_type.cloned(),
    }))
}

/// A `DV_URI`, or a `DV_EHR_URI` where the template named that class.
///
/// Section 10.3.1: `Value_valid: not value.is_empty`. Section 10.3.2 adds
/// `Scheme_valid: scheme.is_equal ("ehr")`, so the class the template named
/// decides which of the two this is and whether the scheme is checked.
fn uri(key: &NodeKey, rm_type: &str, value: &str) -> Result<DataValue, BuildError> {
    if value.is_empty() {
        return Err(BuildError::Invariant {
            key: key.clone(),
            invariant: "DV_URI.Value_valid",
            detail: "the URI is empty".to_owned(),
        });
    }
    if rm_type == "DV_EHR_URI" {
        if !value.starts_with("ehr:") {
            return Err(BuildError::Invariant {
                key: key.clone(),
                invariant: "DV_EHR_URI.Scheme_valid",
                detail: format!("`{value}` does not use the `ehr` scheme"),
            });
        }
        return Ok(DataValue::DvUri(DvUri::DvEhrUri(DvEhrUri {
            value: value.to_owned(),
        })));
    }
    Ok(DataValue::DvUri(DvUri::DvUri(DvUriData {
        value: value.to_owned(),
    })))
}

/// A `DV_PARSABLE`. Section 9.2.3: `Formalism_valid: not formalism.is_empty`,
/// and the value "may validly be empty in some syntaxes".
fn parsable(key: &NodeKey, value: &str, formalism: &str) -> Result<DataValue, BuildError> {
    if formalism.is_empty() {
        return Err(BuildError::Invariant {
            key: key.clone(),
            invariant: "DV_PARSABLE.Formalism_valid",
            detail: "the formalism is empty".to_owned(),
        });
    }
    Ok(DataValue::DvParsable(DvParsable {
        charset: None,
        language: None,
        value: value.to_owned(),
        formalism: formalism.to_owned(),
    }))
}

/// A `DV_TEXT` carrying `value` and nothing else.
fn text(value: &str) -> DvTextData {
    DvTextData {
        value: value.to_owned(),
        hyperlink: None,
        formatting: None,
        mappings: None,
        language: None,
        encoding: None,
    }
}

/// A `DV_CODED_TEXT` whose `value` is the rubric of `code`.
///
/// Section 5.2.4: a coded text is "a combination of a `CODE_PHRASE` and the
/// rubric of that term, from a terminology service, in the language in which
/// the data were authored".
fn coded(terminology: &str, code: &str, rubric: &str) -> DvCodedText {
    DvCodedText {
        value: rubric.to_owned(),
        hyperlink: None,
        formatting: None,
        mappings: None,
        language: None,
        encoding: None,
        defining_code: code_phrase(terminology, code),
    }
}

/// A `DV_COUNT` carrying `magnitude`.
fn count(magnitude: i64) -> DvCount {
    DvCount {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        accuracy_is_percent: None,
        magnitude,
    }
}

/// A `DV_DATE` carrying `value`, which section 7.2.2 permits to be partial.
fn date(value: &str) -> DvDate {
    DvDate {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        value: value.to_owned(),
    }
}

/// A `DV_TIME` carrying `value`, which section 7.2.3 permits to be partial.
fn time(value: &str) -> DvTime {
    DvTime {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        value: value.to_owned(),
    }
}

/// A `DV_DURATION` carrying `value`.
fn duration(value: &str) -> DvDuration {
    DvDuration {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        accuracy_is_percent: None,
        value: value.to_owned(),
    }
}

/// A `DV_DATE_TIME` carrying `value`.
pub(crate) fn date_time(value: &str) -> DvDateTime {
    DvDateTime {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        value: value.to_owned(),
    }
}

/// A `DV_PROPORTION`, with every invariant section 6.2.10 states checked.
///
/// This is the richest invariant set in the Reference Model, and every one of
/// them is a way for a form to produce a document a CDR refuses.
#[expect(
    clippy::float_cmp,
    reason = "openEHR RM Release-1.1.0 data_types.html section 6.2.10 states               these invariants as exact comparisons: Valid_denominator is               `denominator /= 0.0`, Unitary_validity is `denominator = 1` and               Percent_validity is `denominator = 100`. A margin of error would               accept a denominator of 99.9999 as a percentage."
)]
fn proportion(
    key: &NodeKey,
    numerator: f64,
    denominator: f64,
    kind: i64,
    precision: Option<i32>,
) -> Result<DataValue, BuildError> {
    let refuse = |invariant: &'static str, detail: String| BuildError::Invariant {
        key: key.clone(),
        invariant,
        detail,
    };

    if !(0..=PK_MAX).contains(&kind) {
        return Err(refuse(
            "DV_PROPORTION.Type_validity",
            format!("`{kind}` is not a PROPORTION_KIND"),
        ));
    }
    if denominator == 0.0 {
        return Err(refuse(
            "DV_PROPORTION.Valid_denominator",
            "the denominator is zero".to_owned(),
        ));
    }
    let is_integral = numerator.floor() == numerator && denominator.floor() == denominator;
    if precision == Some(0) && !is_integral {
        return Err(refuse(
            "DV_PROPORTION.Precision_validity",
            format!(
                "a precision of 0 requires whole numbers, and {numerator}/{denominator} is not"
            ),
        ));
    }
    if (kind == PK_FRACTION || kind == PK_INTEGER_FRACTION) && !is_integral {
        return Err(refuse(
            "DV_PROPORTION.Fraction_validity",
            format!("a fraction requires whole numbers, and {numerator}/{denominator} is not"),
        ));
    }
    if kind == PK_UNITARY && denominator != 1.0 {
        return Err(refuse(
            "DV_PROPORTION.Unitary_validity",
            format!("a unitary proportion has a denominator of 1, and this has {denominator}"),
        ));
    }
    if kind == PK_PERCENT && denominator != 100.0 {
        return Err(refuse(
            "DV_PROPORTION.Percent_validity",
            format!("a percentage has a denominator of 100, and this has {denominator}"),
        ));
    }
    let r#type = i32::try_from(kind).map_err(|_| {
        refuse(
            "DV_PROPORTION.Type_validity",
            format!("`{kind}` does not fit the Integer the class declares"),
        )
    })?;
    Ok(DataValue::DvProportion(DvProportion {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        accuracy_is_percent: None,
        numerator,
        denominator,
        r#type,
        precision,
    }))
}

/// A `DV_INTERVAL`, whose ends must be the same `DV_ORDERED` descendant.
///
/// Section 6.2.2: "The type parameter, T, must be a descendant of the type
/// `DV_ORDERED`", and `Limits_consistent` requires a bounded pair to be
/// strictly comparable and ordered.
fn interval(
    key: &NodeKey,
    lower: Option<&Datum>,
    upper: Option<&Datum>,
    lower_included: bool,
    upper_included: bool,
) -> Result<DataValue, BuildError> {
    let bound = |datum: Option<&Datum>| -> Result<Option<DvOrdered>, BuildError> {
        match datum {
            None => Ok(None),
            Some(datum) => ordered(key, datum).map(Some),
        }
    };
    let lower_value = bound(lower)?;
    let upper_value = bound(upper)?;
    let lower_unbounded = lower_value.is_none();
    let upper_unbounded = upper_value.is_none();
    // `Interval.Lower_included_valid` and `Upper_included_valid`: an
    // unbounded end cannot be included, whatever the caller asked for
    // (openEHR BASE Release-1.2.0 `foundation_types.html` section 5.2.1).
    Ok(DataValue::DvInterval(DvInterval {
        lower: lower_value,
        upper: upper_value,
        lower_unbounded,
        upper_unbounded,
        lower_included: lower_included && !lower_unbounded,
        upper_included: upper_included && !upper_unbounded,
    }))
}

/// One end of an interval, as the `DV_ORDERED` the Reference Model requires.
fn ordered(key: &NodeKey, datum: &Datum) -> Result<DvOrdered, BuildError> {
    let wrong = |found: &'static str| BuildError::WrongDatum {
        key: key.clone(),
        expected: "an interval end, which must be a DV_ORDERED",
        found,
    };
    match *datum {
        Datum::Count(magnitude) => Ok(DvOrdered::DvCount(count(magnitude))),
        Datum::Quantity {
            magnitude,
            ref units,
            precision,
        } => Ok(DvOrdered::DvQuantity(DvQuantity {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: None,
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            magnitude,
            precision,
            units: units.clone(),
            units_system: None,
            units_display_name: None,
        })),
        Datum::Date(ref value) => Ok(DvOrdered::DvDate(date(value))),
        Datum::Time(ref value) => Ok(DvOrdered::DvTime(time(value))),
        Datum::DateTime(ref value) => Ok(DvOrdered::DvDateTime(date_time(value))),
        Datum::Duration(ref value) => Ok(DvOrdered::DvDuration(duration(value))),
        Datum::Boolean(_) => Err(wrong("a DV_BOOLEAN")),
        Datum::Text(_) => Err(wrong("a DV_TEXT")),
        Datum::Coded { .. } => Err(wrong("a DV_CODED_TEXT")),
        Datum::Ordinal { .. } => Err(wrong("a DV_ORDINAL")),
        Datum::Scale { .. } => Err(wrong("a DV_SCALE")),
        Datum::Proportion { .. } => Err(wrong("a DV_PROPORTION")),
        Datum::Identifier { .. } => Err(wrong("a DV_IDENTIFIER")),
        Datum::Uri(_) => Err(wrong("a DV_URI")),
        Datum::Parsable { .. } => Err(wrong("a DV_PARSABLE")),
        Datum::Interval { .. } => Err(wrong("a DV_INTERVAL")),
    }
}

/// Reads a `DATA_VALUE` back into the datum a form holds.
///
/// The inverse of [`build`] for everything a form can produce. A class the
/// derivation never produces is reported rather than guessed at, because a
/// value the form cannot hold is content the round trip would lose.
///
/// # Errors
/// [`ReadError::WrongShape`] for a `DATA_VALUE` class no field derives to.
pub fn read(path: &str, value: &DataValue) -> Result<Datum, ReadError> {
    match *value {
        DataValue::DvBoolean(ref it) => Ok(Datum::Boolean(it.value)),
        DataValue::DvText(DvText::DvText(ref it)) => Ok(Datum::Text(it.value.clone())),
        DataValue::DvText(DvText::DvCodedText(ref it)) => Ok(read_coded(it)),
        DataValue::DvOrdinal(ref it) => {
            let (terminology, code, rubric) = parts(&it.symbol);
            Ok(Datum::Ordinal {
                terminology,
                code,
                rubric,
                value: i64::from(it.value),
            })
        }
        DataValue::DvScale(ref it) => {
            let (terminology, code, rubric) = parts(&it.symbol);
            Ok(Datum::Scale {
                terminology,
                code,
                rubric,
                value: it.value,
            })
        }
        DataValue::DvCount(ref it) => Ok(Datum::Count(it.magnitude)),
        DataValue::DvQuantity(ref it) => Ok(Datum::Quantity {
            magnitude: it.magnitude,
            units: it.units.clone(),
            precision: it.precision,
        }),
        DataValue::DvProportion(ref it) => Ok(Datum::Proportion {
            numerator: it.numerator,
            denominator: it.denominator,
            kind: i64::from(it.r#type),
            precision: it.precision,
        }),
        DataValue::DvDate(ref it) => Ok(Datum::Date(it.value.clone())),
        DataValue::DvTime(ref it) => Ok(Datum::Time(it.value.clone())),
        DataValue::DvDateTime(ref it) => Ok(Datum::DateTime(it.value.clone())),
        DataValue::DvDuration(ref it) => Ok(Datum::Duration(it.value.clone())),
        DataValue::DvIdentifier(ref it) => Ok(Datum::Identifier {
            id: it.id.clone(),
            issuer: it.issuer.clone(),
            assigner: it.assigner.clone(),
            identifier_type: it.r#type.clone(),
        }),
        DataValue::DvUri(DvUri::DvUri(ref it)) => Ok(Datum::Uri(it.value.clone())),
        DataValue::DvUri(DvUri::DvEhrUri(ref it)) => Ok(Datum::Uri(it.value.clone())),
        DataValue::DvParsable(ref it) => Ok(Datum::Parsable {
            value: it.value.clone(),
            formalism: it.formalism.clone(),
        }),
        DataValue::DvInterval(ref it) => {
            let end = |bound: Option<&DvOrdered>| -> Result<Option<Box<Datum>>, ReadError> {
                match bound {
                    None => Ok(None),
                    Some(bound) => read_ordered(path, bound).map(|datum| Some(Box::new(datum))),
                }
            };
            Ok(Datum::Interval {
                lower: end(it.lower.as_ref())?,
                upper: end(it.upper.as_ref())?,
                lower_included: it.lower_included,
                upper_included: it.upper_included,
            })
        }
        ref other => Err(ReadError::WrongShape {
            path: path.to_owned(),
            expected: "a value the field derivation produces",
            found: class_of(other).to_owned(),
        }),
    }
}

/// A coded text as the datum a form holds.
fn read_coded(it: &DvCodedText) -> Datum {
    let (terminology, code, rubric) = parts(it);
    Datum::Coded {
        terminology,
        code,
        rubric,
    }
}

/// The terminology, the code and the rubric of a coded text.
fn parts(it: &DvCodedText) -> (String, String, String) {
    (
        it.defining_code.terminology_id.value.clone(),
        it.defining_code.code_string.clone(),
        it.value.clone(),
    )
}

/// One end of an interval, back into a datum.
fn read_ordered(path: &str, bound: &DvOrdered) -> Result<Datum, ReadError> {
    match *bound {
        DvOrdered::DvCount(ref it) => Ok(Datum::Count(it.magnitude)),
        DvOrdered::DvQuantity(ref it) => Ok(Datum::Quantity {
            magnitude: it.magnitude,
            units: it.units.clone(),
            precision: it.precision,
        }),
        DvOrdered::DvDate(ref it) => Ok(Datum::Date(it.value.clone())),
        DvOrdered::DvTime(ref it) => Ok(Datum::Time(it.value.clone())),
        DvOrdered::DvDateTime(ref it) => Ok(Datum::DateTime(it.value.clone())),
        DvOrdered::DvDuration(ref it) => Ok(Datum::Duration(it.value.clone())),
        ref other => Err(ReadError::WrongShape {
            path: path.to_owned(),
            expected: "an interval end the field derivation produces",
            found: format!("{other:?}"),
        }),
    }
}

/// The Reference Model class name of a data value, for a report.
fn class_of(value: &DataValue) -> &'static str {
    match *value {
        DataValue::DvGeneralTimeSpecification(_) => "DV_GENERAL_TIME_SPECIFICATION",
        DataValue::DvMultimedia(_) => "DV_MULTIMEDIA",
        DataValue::DvParagraph(_) => "DV_PARAGRAPH",
        DataValue::DvPeriodicTimeSpecification(_) => "DV_PERIODIC_TIME_SPECIFICATION",
        DataValue::DvState(_) => "DV_STATE",
        _ => "a data value",
    }
}
