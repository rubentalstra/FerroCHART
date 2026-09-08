// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The layout a person authored, as the form reads it.
//!
//! No specification governs any of this: our own design. openEHR publishes no
//! form artefact, so field order, labels, help text, defaults and conditional
//! visibility are all authored rather than derived
//! (`ferrochart_form::layout`).
//!
//! The layout decorates the definition and never replaces it. Nothing here
//! can make a control admit a value the template refuses: an authored label
//! changes the words above a field, and an authored rule changes whether the
//! field is on screen, and neither one touches what the control accepts.
//!
//! A form whose server serves no layout gets [`Laid::none`], which decides
//! nothing, so every screen below draws the same way it did before there was
//! an overlay.

use std::sync::Arc;

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::key::NodeKey;
use ferrochart_form::layout::{Answers, FormLayout, Layout, Visibility};
use ferrochart_form::text::Localized;
use ferrochart_form::values::Entered;
use leptos::prelude::*;

use crate::state::{FormState, Path};

/// The layout the screen is drawing under.
///
/// Cheap to clone, because every control reads it and a redraw clones it per
/// node. It travels through Leptos context rather than through a prop on
/// every component, so a control deep in the tree reads it without each
/// group above it having to carry it.
#[derive(Clone, Debug)]
pub(crate) struct Laid(Option<Arc<FormLayout>>);

impl Laid {
    /// The layout the server served.
    pub(crate) fn of(layout: FormLayout) -> Self {
        Self(Some(Arc::new(layout)))
    }

    /// No layout, which is what a template nobody laid out has.
    pub(crate) fn none() -> Self {
        Self(None)
    }

    /// The layout in context, or one that decides nothing.
    ///
    /// A component drawn outside a form (the style guide draws several) has
    /// no layout in context and reads one that decides nothing, rather than
    /// refusing to draw.
    pub(crate) fn from_context() -> Self {
        use_context::<Self>().unwrap_or_else(Self::none)
    }

    /// What the person authored for `key`, where they authored anything.
    pub(crate) fn at(&self, key: &NodeKey) -> Option<&Layout> {
        self.0.as_ref().and_then(|layout| layout.of(key))
    }

    /// The label to draw over `key`, where a person wrote one.
    ///
    /// The archetype rubric is what the caller falls back to, so a layout
    /// that states the label in no language at all is the same as no layout.
    pub(crate) fn label(&self, key: &NodeKey, language: &LanguageTag) -> Option<String> {
        self.at(key)
            .and_then(|layout| said(&layout.label, language))
    }

    /// The help text to draw under `key`, where a person wrote one.
    pub(crate) fn help(&self, key: &NodeKey, language: &LanguageTag) -> Option<String> {
        self.at(key).and_then(|layout| said(&layout.help, language))
    }

    /// Whether the item at `key` is shown, given what is entered under `path`.
    ///
    /// Reactive: the read goes through the form's own signal, so ticking the
    /// box a rule names redraws the items that rule governs.
    pub(crate) fn shows(&self, key: &NodeKey, state: FormState, path: &Path) -> bool {
        let Some(layout) = self.at(key) else {
            return true;
        };
        layout.visibility.shows(&Reader { state, path })
    }

    /// Whether a person authored a visibility rule over `key`.
    ///
    /// The gate is only wrapped around an item that has one, so a form nobody
    /// laid out costs no extra node per field.
    pub(crate) fn rules(&self, key: &NodeKey) -> bool {
        self.at(key)
            .is_some_and(|layout| layout.visibility != Visibility::Always)
    }

    /// Where the item at `key` sits among its siblings, where a person said.
    pub(crate) fn order(&self, key: &NodeKey) -> Option<u32> {
        self.at(key).and_then(|layout| layout.order)
    }
}

/// The text `localized` states in `language`, where it states one that is not
/// empty.
fn said(localized: &Localized, language: &LanguageTag) -> Option<String> {
    crate::label::text(localized, language).map(str::to_owned)
}

/// How a visibility rule reads the form it is judged against.
struct Reader<'p> {
    /// The values the form holds.
    state: FormState,
    /// Where the item being judged sits.
    path: &'p Path,
}

impl Answers for Reader<'_> {
    fn entered(&self, node: &NodeKey) -> Option<Entered> {
        self.state.nearest(node, self.path)
    }
}
