// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The add and remove pair a repeatable item carries.
//!
//! One definition, used by a repeatable field and by a repeatable group.
//! `C_OBJECT.occurrences` (openEHR AM Release-2.3.0 `AOM1.4.html` section
//! 4.3.6) is the ceiling and the floor, and at either one the button is
//! disabled rather than offering an action that does nothing.

use leptos::prelude::*;

use crate::icon::{Decoration, Size};
use crate::kit::field::BTN_SECONDARY;

/// The pair of buttons that adds and removes an occurrence.
#[component]
pub(crate) fn Repeats(
    /// What adding one does.
    add: Callback<()>,
    /// What removing the last one does.
    remove: Callback<()>,
    /// Whether the template's ceiling is reached.
    at_ceiling: Signal<bool>,
    /// Whether the template's floor is reached.
    at_floor: Signal<bool>,
    /// What the adding button is called.
    add_label: &'static str,
    /// What the removing button is called.
    remove_label: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex gap-2">
            <button
                type="button"
                class=BTN_SECONDARY
                disabled=move || at_ceiling.get()
                on:click=move |_| add.run(())
            >
                <Decoration icon=icondata_lu::LuPlus size=Size::Inline />
                {add_label}
            </button>
            <button
                type="button"
                class=BTN_SECONDARY
                disabled=move || at_floor.get()
                on:click=move |_| remove.run(())
            >
                <Decoration icon=icondata_lu::LuMinus size=Size::Inline />
                {remove_label}
            </button>
        </div>
    }
}
