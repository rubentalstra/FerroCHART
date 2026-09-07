// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A web template `validation` range, in both directions.
//!
//! A range is stated as `minOp`, `min`, `maxOp`, `max`, where each operator is
//! a comparison spelled as text. An operator that admits its own end (`>=`,
//! `<=`) makes the end included, and one that does not (`>`, `<`) makes it
//! excluded, which is the inclusivity
//! [`ferrochart_form::range::Range`] carries.

use ferrochart_form::range::Range;
use serde_json::{Map, Number, Value};

/// The `minOp` and `maxOp` an included end is spelled with.
const INCLUSIVE_OPS: (&str, &str) = (">=", "<=");

/// The `minOp` and `maxOp` an excluded end is spelled with.
const EXCLUSIVE_OPS: (&str, &str) = (">", "<");

/// The raw ends of a range object.
struct Ends<'a> {
    min: Option<&'a Value>,
    max: Option<&'a Value>,
    min_included: bool,
    max_included: bool,
}

fn ends(range: &Map<String, Value>) -> Ends<'_> {
    let stated = |member: &str| match range.get(member) {
        None | Some(&Value::Null) => None,
        Some(value) => Some(value),
    };
    let included = |member: &str, exclusive: &str| {
        range
            .get(member)
            .and_then(Value::as_str)
            .is_none_or(|op| op != exclusive)
    };
    Ends {
        min: stated("min"),
        max: stated("max"),
        min_included: included("minOp", EXCLUSIVE_OPS.0),
        max_included: included("maxOp", EXCLUSIVE_OPS.1),
    }
}

/// The `validation` member `name` of a leaf, as a real-valued range.
pub(crate) fn real(validation: Option<&Map<String, Value>>, name: &str) -> Option<Range<f64>> {
    let range = validation?.get(name)?.as_object()?;
    let ends = ends(range);
    let min = ends.min.and_then(Value::as_f64);
    let max = ends.max.and_then(Value::as_f64);
    (min.is_some() || max.is_some())
        .then(|| Range::new(min, max, ends.min_included, ends.max_included))
}

/// The `validation` member `name` of a leaf, as a whole-number range.
pub(crate) fn integer(validation: Option<&Map<String, Value>>, name: &str) -> Option<Range<i64>> {
    let range = validation?.get(name)?.as_object()?;
    let ends = ends(range);
    let min = ends.min.and_then(Value::as_i64);
    let max = ends.max.and_then(Value::as_i64);
    (min.is_some() || max.is_some())
        .then(|| Range::new(min, max, ends.min_included, ends.max_included))
}

/// The `validation` member `name` of a leaf, as a range of ISO 8601 text.
pub(crate) fn text(validation: Option<&Map<String, Value>>, name: &str) -> Option<Range<String>> {
    let range = validation?.get(name)?.as_object()?;
    let ends = ends(range);
    let min = ends.min.and_then(Value::as_str).map(str::to_owned);
    let max = ends.max.and_then(Value::as_str).map(str::to_owned);
    (min.is_some() || max.is_some())
        .then(|| Range::new(min, max, ends.min_included, ends.max_included))
}

/// A range object as a web template states it.
fn object(
    min: Option<Value>,
    max: Option<Value>,
    min_included: bool,
    max_included: bool,
) -> Option<Value> {
    if min.is_none() && max.is_none() {
        return None;
    }
    let mut out = Map::new();
    if let Some(min) = min {
        let op = if min_included {
            INCLUSIVE_OPS.0
        } else {
            EXCLUSIVE_OPS.0
        };
        out.insert("minOp".to_owned(), Value::String(op.to_owned()));
        out.insert("min".to_owned(), min);
    }
    if let Some(max) = max {
        let op = if max_included {
            INCLUSIVE_OPS.1
        } else {
            EXCLUSIVE_OPS.1
        };
        out.insert("maxOp".to_owned(), Value::String(op.to_owned()));
        out.insert("max".to_owned(), max);
    }
    Some(Value::Object(out))
}

/// The real-valued range as a web template states it.
pub(crate) fn real_value(range: &Range<f64>) -> Option<Value> {
    object(
        range.minimum.and_then(Number::from_f64).map(Value::Number),
        range.maximum.and_then(Number::from_f64).map(Value::Number),
        range.minimum_included,
        range.maximum_included,
    )
}

/// The whole-number range as a web template states it.
pub(crate) fn integer_value(range: &Range<i64>) -> Option<Value> {
    object(
        range.minimum.map(Value::from),
        range.maximum.map(Value::from),
        range.minimum_included,
        range.maximum_included,
    )
}

/// The ISO 8601 text range as a web template states it.
pub(crate) fn text_value(range: &Range<String>) -> Option<Value> {
    object(
        range.minimum.clone().map(Value::String),
        range.maximum.clone().map(Value::String),
        range.minimum_included,
        range.maximum_included,
    )
}

#[cfg(test)]
mod tests {
    use super::{integer, integer_value, real, real_value};
    use ferrochart_form::range::Range;
    use serde_json::json;

    fn validation(value: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        value.as_object().cloned().expect("an object")
    }

    #[test]
    fn an_operator_that_does_not_admit_its_end_makes_the_end_excluded() {
        let stated =
            validation(&json!({"range": {"minOp": ">", "min": 0.0, "maxOp": "<=", "max": 9.5}}));
        let read = real(Some(&stated), "range").expect("a range");
        assert!(!read.minimum_included);
        assert!(read.maximum_included);
    }

    #[test]
    fn a_range_with_one_end_states_only_that_end() {
        let range = Range::new(Some(1_i64), None, true, true);
        assert_eq!(
            integer_value(&range),
            Some(json!({"minOp": ">=", "min": 1}))
        );
    }

    #[test]
    fn a_whole_number_range_reads_back_as_it_was_written() {
        let range = Range::new(Some(1_i64), Some(4_i64), true, false);
        let written = integer_value(&range).expect("a range");
        let stated = validation(&json!({"precision": written}));
        assert_eq!(integer(Some(&stated), "precision"), Some(range));
    }

    #[test]
    fn a_range_with_no_end_at_all_is_no_range() {
        let range: Range<f64> = Range::new(None, None, true, true);
        assert_eq!(real_value(&range), None);
    }
}
