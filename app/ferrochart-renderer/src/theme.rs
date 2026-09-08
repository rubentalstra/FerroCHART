// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The theme: a class on the document element, persisted, applied after
//! hydration so the first render is deterministic.
//!
//! No specification governs this: our own design. Both themes are the same
//! semantic token names redefined under `.dark` in `style/tailwind.css`, so
//! nothing here knows a colour.

use leptos::logging;
use leptos::prelude::{Effect, Get, RwSignal, document};

use crate::storage;

/// The key the reader's choice is remembered under.
const STORAGE_KEY: &str = "ferrochart.renderer.theme";
/// The class the stylesheet's `dark` variant matches.
const DARK_CLASS: &str = "dark";
/// The class that pins the light theme against a dark system preference.
const LIGHT_CLASS: &str = "light";

/// Which theme the document is showing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Theme {
    /// The light ground.
    Light,
    /// The dark ground.
    Dark,
}

impl Theme {
    /// The stored form, and the value [`Theme::from_key`] reads back.
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The theme's name, as a control says it aloud.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The other theme.
    pub(crate) const fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    /// Reads a stored key back, or `None` for anything this version does not
    /// recognise, so an old or hand-edited value falls back rather than
    /// pinning the reader to a theme that no longer exists.
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// The theme the operating system asks for.
    pub(crate) fn preferred() -> Self {
        let matches = leptos::prelude::window()
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
            .is_some_and(|query| query.matches());
        if matches { Self::Dark } else { Self::Light }
    }

    /// The reader's stored choice, falling back to the system preference.
    pub(crate) fn initial() -> Self {
        storage::read(STORAGE_KEY)
            .as_deref()
            .and_then(Self::from_key)
            .unwrap_or_else(Self::preferred)
    }
}

/// Holds the theme, writes the class onto `<html>`, and remembers the choice.
///
/// The signal is created with the stored value already in it, so the first
/// paint after mount is the right one and nothing corrects itself afterwards.
pub(crate) fn install() -> RwSignal<Theme> {
    let theme = RwSignal::new(Theme::initial());
    Effect::new(move |_| {
        let chosen = theme.get();
        apply(chosen);
        storage::write(STORAGE_KEY, chosen.key());
    });
    theme
}

/// Puts the theme's class on the document element.
///
/// Both classes are written explicitly. An absent class is not the light
/// theme: it is "no choice yet", which the stylesheet resolves from the
/// system preference, so pinning light against a dark system needs its own
/// class.
fn apply(theme: Theme) {
    let Some(root) = document().document_element() else {
        logging::warn!("the document has no root element, so the theme is not applied");
        return;
    };
    let classes = root.class_list();
    let (add, remove) = match theme {
        Theme::Dark => (DARK_CLASS, LIGHT_CLASS),
        Theme::Light => (LIGHT_CLASS, DARK_CLASS),
    };
    if classes.remove_1(remove).is_err() || classes.add_1(add).is_err() {
        logging::warn!("the browser refused the theme class");
    }
}

#[cfg(test)]
mod tests {
    use super::Theme;

    #[test]
    fn every_theme_survives_a_round_trip_through_its_stored_key() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(Theme::from_key(theme.key()), Some(theme), "{theme:?}");
        }
    }

    #[test]
    fn a_stored_value_this_version_does_not_know_is_treated_as_absent() {
        assert_eq!(Theme::from_key("solarized"), None);
        assert_eq!(Theme::from_key(""), None);
    }

    #[test]
    fn toggling_twice_returns_the_theme_it_started_from() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(theme.toggled().toggled(), theme);
            assert_ne!(theme.toggled(), theme);
        }
    }

    #[test]
    fn the_two_themes_do_not_share_a_label_or_a_key() {
        assert_ne!(Theme::Light.key(), Theme::Dark.key());
        assert_ne!(Theme::Light.label(), Theme::Dark.label());
    }
}
