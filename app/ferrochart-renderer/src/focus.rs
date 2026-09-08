// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Moving focus, and keeping it inside a region.
//!
//! No specification governs this: our own design, over the WCAG 2.2 AA and
//! keyboard-only commitment of `docs/architecture.md` section 10. Two regions
//! of this app trap focus, the rail overlay and the confirm dialog, and a
//! trap that leaks is how a keyboard reader ends up typing into a page they
//! cannot see. One implementation, so the second region cannot get it wrong.

use leptos::ev::KeyboardEvent;
use web_sys::HtmlElement;
use web_sys::wasm_bindgen::JsCast;

/// What a Tab press can land on inside a trapped region.
const FOCUSABLE: &str = "a[href], button:not([disabled]), input:not([disabled]), \
                         select:not([disabled]), textarea:not([disabled]), \
                         [tabindex]:not([tabindex='-1'])";

/// Moves focus, reporting a refusal rather than losing it silently.
///
/// A browser refuses when the element is not rendered yet, and a trap whose
/// entry quietly failed leaves the reader outside the region it guards.
pub(crate) fn focus(element: &HtmlElement) {
    if element.focus().is_err() {
        leptos::logging::warn!("the browser refused to move focus");
    }
}

/// Moves focus to the first thing inside a region.
pub(crate) fn focus_first(region: Option<HtmlElement>) {
    if let Some(first) = region.and_then(|region| focusable(&region).into_iter().next()) {
        focus(&first);
    }
}

/// Keeps Tab inside a region by wrapping it at either end.
pub(crate) fn trap(event: &KeyboardEvent, region: Option<HtmlElement>) {
    let Some(region) = region else { return };
    let stops = focusable(&region);
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return;
    };
    let Some(focused) = leptos::prelude::document().active_element() else {
        return;
    };
    let wrap_to = if event.shift_key() {
        (focused == **first).then_some(last)
    } else {
        (focused == **last).then_some(first)
    };
    if let Some(target) = wrap_to {
        event.prevent_default();
        focus(target);
    }
}

/// The element that had focus, so a region can put it back.
pub(crate) fn focused() -> Option<HtmlElement> {
    leptos::prelude::document()
        .active_element()
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
}

/// Everything inside a region that a Tab press can land on, in document
/// order.
fn focusable(region: &HtmlElement) -> Vec<HtmlElement> {
    let Ok(found) = region.query_selector_all(FOCUSABLE) else {
        return Vec::new();
    };
    (0..found.length())
        .filter_map(|at| found.item(at))
        .filter_map(|node| node.dyn_into::<HtmlElement>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::FOCUSABLE;

    #[test]
    fn the_selector_skips_what_a_tab_press_cannot_land_on() {
        assert!(
            FOCUSABLE.contains(":not([disabled])"),
            "a disabled control is not a tab stop"
        );
        assert!(
            FOCUSABLE.contains("[tabindex]:not([tabindex='-1'])"),
            "tabindex=-1 is programmatic focus, not a tab stop"
        );
        assert!(
            FOCUSABLE.contains("a[href]"),
            "an anchor without an href is not a tab stop"
        );
    }
}
