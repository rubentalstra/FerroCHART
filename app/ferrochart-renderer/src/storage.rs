// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Local storage, reduced to two calls that cannot fail loudly.
//!
//! No specification governs this: our own design. A browser refuses
//! `localStorage` in a private context and when a site's data is blocked, and
//! a preference is not worth a broken screen, so a refusal is logged and the
//! caller reads the default.

use leptos::logging;

/// Reads a key, or `None` when storage is unavailable or the key is unset.
pub(crate) fn read(key: &str) -> Option<String> {
    let storage = leptos::prelude::window().local_storage().ok()??;
    // NOTE: a refused read and an unset key are the same answer to the
    // caller, which is why this one is an Option rather than a Result.
    storage.get_item(key).ok()?
}

/// Writes a key, logging a refusal rather than failing the screen.
pub(crate) fn write(key: &str, value: &str) {
    let Ok(Some(storage)) = leptos::prelude::window().local_storage() else {
        logging::warn!("local storage is unavailable, so {key} is not remembered");
        return;
    };
    if storage.set_item(key, value).is_err() {
        logging::warn!("local storage refused {key}");
    }
}
