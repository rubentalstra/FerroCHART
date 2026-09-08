// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Timezones, from the list the platform already carries.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 7.2.4 types
//! `DV_DATE_TIME` on `Iso8601_date_time`, so what a value carries is a UTC
//! offset and not the name of a zone. A person does not know their offset:
//! they know where they are, and whether it is summer is not their problem.
//!
//! So the reader picks a zone and this module resolves it to the offset at the
//! instant they entered. The list is the IANA Time Zone Database, whose
//! registry RFC 6557 puts with IANA, and every current browser already carries
//! it: ECMA-402 exposes the identifiers as `Intl.supportedValuesOf('timeZone')`
//! and the offset through `Intl.DateTimeFormat`. Nothing is vendored and the
//! bundle carries no table (issue #197).

use web_sys::wasm_bindgen::JsCast;
use web_sys::wasm_bindgen::JsValue;

/// Every timezone identifier the platform knows, in the order it lists them.
///
/// Empty where the platform does not answer, which is what the caller falls
/// back on: a build that cannot read the list offers the offset field instead
/// of offering nothing.
pub(crate) fn every() -> Vec<String> {
    let Some(intl) = namespace() else {
        return Vec::new();
    };
    let Ok(supported) = js_sys::Reflect::get(&intl, &JsValue::from_str("supportedValuesOf")) else {
        return Vec::new();
    };
    let Some(call) = supported.dyn_ref::<js_sys::Function>() else {
        return Vec::new();
    };
    let Ok(answered) = call.call1(&intl, &JsValue::from_str("timeZone")) else {
        return Vec::new();
    };
    js_sys::Array::from(&answered)
        .iter()
        .filter_map(|value| value.as_string())
        .collect()
}

/// The zone the reader's own browser is set to, where it names one.
pub(crate) fn here() -> Option<String> {
    let format = js_sys::Intl::DateTimeFormat::new(&js_sys::Array::new(), &js_sys::Object::new());
    let resolved = format.resolved_options();
    js_sys::Reflect::get(&resolved, &JsValue::from_str("timeZone"))
        .ok()
        .and_then(|value| value.as_string())
}

/// The UTC offset `zone` is on at `instant`, as ISO 8601 writes it.
///
/// `instant` is the value the reader typed, so a zone whose offset changes
/// across the year resolves to the one in force then: `Europe/Amsterdam` is
/// `+01:00` in January and `+02:00` in July, and a form that wrote the wrong
/// one would put a time in a record an hour from where it happened.
pub(crate) fn offset_at(zone: &str, instant: &str) -> Option<String> {
    let options = js_sys::Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("timeZone"),
        &JsValue::from_str(zone),
    )
    .ok()?;
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("timeZoneName"),
        &JsValue::from_str("longOffset"),
    )
    .ok()?;
    let locales = js_sys::Array::of1(&JsValue::from_str("en-GB"));
    let format = js_sys::Intl::DateTimeFormat::new(&locales, &options);
    let date = js_sys::Date::new(&JsValue::from_str(instant));
    if date.get_time().is_nan() {
        return None;
    }
    let parts = js_sys::Reflect::get(&format, &JsValue::from_str("formatToParts")).ok()?;
    let call = parts.dyn_ref::<js_sys::Function>()?;
    let listed = call.call1(&format, &date).ok()?;
    let named = js_sys::Array::from(&listed).iter().find_map(|part| {
        let kind = js_sys::Reflect::get(&part, &JsValue::from_str("type"))
            .ok()?
            .as_string()?;
        (kind == "timeZoneName")
            .then(|| {
                js_sys::Reflect::get(&part, &JsValue::from_str("value"))
                    .ok()?
                    .as_string()
            })
            .flatten()
    })?;
    iso_offset(&named)
}

/// The `Intl` namespace, where the platform has one.
fn namespace() -> Option<js_sys::Object> {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Intl"))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Object>().ok())
}

/// The ISO 8601 offset a `longOffset` zone name spells.
///
/// ECMA-402 writes it as `GMT+02:00`, and `GMT` on its own where the offset is
/// zero, which ISO 8601 writes as `Z`.
pub(crate) fn iso_offset(named: &str) -> Option<String> {
    let rest = named.strip_prefix("GMT")?;
    if rest.is_empty() {
        return Some("Z".to_owned());
    }
    let (sign, digits) = rest.split_at_checked(1)?;
    if sign != "+" && sign != "-" {
        return None;
    }
    let (hours, minutes) = match digits.split_once(':') {
        Some((hours, minutes)) => (hours, minutes),
        // A whole-hour offset can come back as `GMT+2`.
        None => (digits, "00"),
    };
    let hours: u8 = hours.parse().ok()?;
    let minutes: u8 = minutes.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(format!("{sign}{hours:02}:{minutes:02}"))
}

#[cfg(test)]
mod tests {
    use super::iso_offset;

    #[test]
    fn a_whole_hour_offset_is_written_with_its_minutes() {
        assert_eq!(iso_offset("GMT+2").as_deref(), Some("+02:00"));
        assert_eq!(iso_offset("GMT+02:00").as_deref(), Some("+02:00"));
        assert_eq!(iso_offset("GMT-05:00").as_deref(), Some("-05:00"));
    }

    #[test]
    fn an_offset_of_zero_is_written_the_way_iso_8601_writes_it() {
        assert_eq!(iso_offset("GMT").as_deref(), Some("Z"));
    }

    #[test]
    fn an_offset_that_is_not_a_whole_hour_keeps_its_minutes() {
        // Asia/Kolkata and Australia/Eucla are the reason this is not an
        // hour count.
        assert_eq!(iso_offset("GMT+05:30").as_deref(), Some("+05:30"));
        assert_eq!(iso_offset("GMT+08:45").as_deref(), Some("+08:45"));
    }

    #[test]
    fn text_that_is_not_an_offset_names_none() {
        assert_eq!(iso_offset("Central European Summer Time"), None);
        assert_eq!(iso_offset("UTC+02:00"), None);
        assert_eq!(iso_offset("GMT*02:00"), None);
        assert_eq!(iso_offset("GMT+99:00"), None);
        assert_eq!(iso_offset(""), None);
    }
}
