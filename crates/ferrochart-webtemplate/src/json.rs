// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Reading one member of a JSON object as the kind of value a web template
//! states there.
//!
//! Every accessor names the object it read from, so a refusal points at the
//! node rather than at the member alone.

use std::collections::BTreeMap;

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::text::Localized;
use serde_json::{Map, Value};

use crate::error::ReadError;

/// What kind of JSON value this is, for an error message.
pub(crate) fn kind_of(value: &Value) -> &'static str {
    match *value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// The object `value` is, or a refusal naming `at`.
pub(crate) fn object<'a>(value: &'a Value, at: &str) -> Result<&'a Map<String, Value>, ReadError> {
    value
        .as_object()
        .ok_or_else(|| ReadError::NotAnObject { at: at.to_owned() })
}

/// The string `member` states, or `None` where the object omits it.
pub(crate) fn optional_str<'a>(
    obj: &'a Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<Option<&'a str>, ReadError> {
    match obj.get(member) {
        None | Some(&Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(Some)
            .ok_or_else(|| ReadError::WrongType {
                at: at.to_owned(),
                member,
                expected: "a string",
                found: kind_of(value),
            }),
    }
}

/// The string `member` states, or a refusal where the object omits it.
pub(crate) fn required_str<'a>(
    obj: &'a Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<&'a str, ReadError> {
    optional_str(obj, member, at)?.ok_or_else(|| ReadError::MissingMember {
        at: at.to_owned(),
        member,
    })
}

/// The whole number `member` states, or `None` where the object omits it.
pub(crate) fn optional_i64(
    obj: &Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<Option<i64>, ReadError> {
    match obj.get(member) {
        None | Some(&Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| ReadError::WrongType {
                at: at.to_owned(),
                member,
                expected: "a whole number",
                found: kind_of(value),
            }),
    }
}

/// The boolean `member` states, or `None` where the object omits it.
pub(crate) fn optional_bool(
    obj: &Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<Option<bool>, ReadError> {
    match obj.get(member) {
        None | Some(&Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| ReadError::WrongType {
                at: at.to_owned(),
                member,
                expected: "a boolean",
                found: kind_of(value),
            }),
    }
}

/// The array `member` states, or the empty slice where the object omits it.
pub(crate) fn optional_array<'a>(
    obj: &'a Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<&'a [Value], ReadError> {
    match obj.get(member) {
        None | Some(&Value::Null) => Ok(&[]),
        Some(value) => value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| ReadError::WrongType {
                at: at.to_owned(),
                member,
                expected: "an array",
                found: kind_of(value),
            }),
    }
}

/// The object `member` states, or `None` where the object omits it.
pub(crate) fn optional_object<'a>(
    obj: &'a Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<Option<&'a Map<String, Value>>, ReadError> {
    match obj.get(member) {
        None | Some(&Value::Null) => Ok(None),
        Some(value) => value
            .as_object()
            .map(Some)
            .ok_or_else(|| ReadError::WrongType {
                at: at.to_owned(),
                member,
                expected: "an object",
                found: kind_of(value),
            }),
    }
}

/// A per-language text member (`localizedNames`, `localizedLabels`, …).
pub(crate) fn localized(
    obj: &Map<String, Value>,
    member: &'static str,
    at: &str,
) -> Result<Localized, ReadError> {
    let mut text = Localized::empty();
    let Some(map) = optional_object(obj, member, at)? else {
        return Ok(text);
    };
    for (language, value) in map {
        let Some(stated) = value.as_str() else {
            return Err(ReadError::WrongType {
                at: at.to_owned(),
                member,
                expected: "a string per language",
                found: kind_of(value),
            });
        };
        text.insert(LanguageTag::new(language.clone()), stated);
    }
    Ok(text)
}

/// A per-language text member as the JSON object a web template states it as,
/// or `None` where the text is stated in no language.
pub(crate) fn localized_value(text: &Localized) -> Option<Value> {
    if text.is_empty() {
        return None;
    }
    let members: BTreeMap<&str, &str> = text
        .by_language
        .iter()
        .map(|(language, stated)| (language.as_str(), stated.as_str()))
        .collect();
    serde_json::to_value(members).ok()
}
