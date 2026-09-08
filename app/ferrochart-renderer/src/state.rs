// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the form holds while a clinician fills it in.
//!
//! No specification governs this: our own design. It wraps
//! [`ferrochart_form::values::FormValues`], which is the type the server takes
//! and the composition builder consumes, so the browser collects the contract
//! rather than a shape of its own that something has to translate.
//!
//! Every read and write is addressed the way the builder addresses one: a
//! field key, the occurrence of each repeating group above it, and the
//! occurrence of the field itself (`docs/architecture.md` section 11). A
//! control never invents an address; it is handed the one it renders under.

use std::sync::Arc;

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LocalCode;
use ferrochart_form::key::NodeKey;
use ferrochart_form::values::{Datum, Entered, FormValues};
use leptos::prelude::*;

/// Where one control sits: the occurrence of each repeating group above it,
/// outermost first.
///
/// Cheap to clone, because every control in a repeated subtree carries one and
/// a redraw clones it per node. [`std::sync::Arc`] rather than
/// [`std::rc::Rc`], because a control's address rides inside a Leptos
/// `Callback`, which requires its closure to be `Send + Sync`.
pub(crate) type Path = Arc<[usize]>;

/// The empty path, which is where a control under no repeating group sits.
pub(crate) fn root_path() -> Path {
    Arc::from(&[][..])
}

/// The path one step deeper, at `index` of a repeating group.
pub(crate) fn descend(path: &Path, index: usize) -> Path {
    let mut grown = path.to_vec();
    grown.push(index);
    Arc::from(grown)
}

/// The values a form holds, and how many occurrences of each repeating group
/// it is showing.
///
/// The two are separate because they answer different questions. The values
/// say what was entered; the shown count says what the clinician can see and
/// type into, which includes an occurrence they have added and not yet filled.
/// A repeat that vanished the moment it was added would be unusable.
#[derive(Clone, Copy)]
pub(crate) struct FormState {
    values: RwSignal<FormValues>,
    shown: RwSignal<Vec<(NodeKey, Vec<usize>, usize)>>,
}

impl FormState {
    /// A form showing the minimum occurrences its definition requires, with
    /// nothing entered.
    pub(crate) fn new(definition: &FormDefinition) -> Self {
        let state = Self {
            values: RwSignal::new(FormValues::new()),
            shown: RwSignal::new(Vec::new()),
        };
        let mut initial = Vec::new();
        seed(&definition.root, &[], &mut initial);
        state.shown.set(initial);
        state
    }

    /// What was entered at one address.
    pub(crate) fn get(&self, key: &NodeKey, path: &Path, occurrence: usize) -> Option<Entered> {
        self.values.read().get_in(key, path, occurrence).cloned()
    }

    /// Records a value at one address.
    pub(crate) fn set(&self, key: &NodeKey, path: &Path, occurrence: usize, datum: Datum) {
        self.values.update(|values| {
            values.set_in(
                key.clone(),
                path.to_vec(),
                occurrence,
                Entered::Value(datum),
            );
        });
    }

    /// Records a null flavour at one address, in place of a value.
    ///
    /// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
    /// `ELEMENT` the invariant `Inv_null_flavour_indicated`, so a null flavour
    /// and a value are alternatives and never both. Writing one here replaces
    /// the other, which is that invariant held by construction.
    pub(crate) fn set_null(
        &self,
        key: &NodeKey,
        path: &Path,
        occurrence: usize,
        code: LocalCode,
        reason: Option<String>,
    ) {
        self.values.update(|values| {
            values.set_in(
                key.clone(),
                path.to_vec(),
                occurrence,
                Entered::Null { code, reason },
            );
        });
    }

    /// Forgets whatever was entered at one address.
    ///
    /// What was there is dropped deliberately: this is the clinician saying
    /// the field is empty, so the previous value is not something a caller
    /// has any use for.
    pub(crate) fn clear(&self, key: &NodeKey, path: &Path, occurrence: usize) {
        self.values.update(|values| {
            drop(values.remove_in(key, path, occurrence));
        });
    }

    /// How many occurrences of `group` the form is showing under `path`.
    pub(crate) fn shown(&self, group: &NodeKey, path: &Path) -> usize {
        self.shown
            .read()
            .iter()
            .find(|(key, at, _)| key == group && at.as_slice() == path.as_ref())
            .map_or(1, |&(_, _, count)| count)
    }

    /// Whether one more occurrence of `key` may be shown.
    pub(crate) fn can_add(&self, key: &NodeKey, path: &Path, maximum: Option<u32>) -> bool {
        maximum.is_none_or(|most| self.shown(key, path) < most as usize)
    }

    /// Whether the last occurrence of `key` may be taken away.
    pub(crate) fn can_remove(&self, key: &NodeKey, path: &Path, minimum: u32) -> bool {
        self.shown(key, path) > minimum.max(1) as usize
    }

    /// Shows one more occurrence of `key`, up to what the template admits.
    ///
    /// Returns whether one was added, so a control can disable itself at the
    /// ceiling rather than offering an action that does nothing.
    pub(crate) fn add_occurrence(&self, key: &NodeKey, path: &Path, maximum: Option<u32>) -> bool {
        if !self.can_add(key, path, maximum) {
            return false;
        }
        let showing = self.shown(key, path);
        self.set_shown(key, path, showing.saturating_add(1));
        true
    }

    /// Hides the last occurrence of `group` and forgets what it held, down to
    /// what the template requires.
    ///
    /// Returns whether one was removed. The values go with it: an occurrence a
    /// clinician removed is one they said is not there, and leaving its
    /// entries behind would commit a document they cannot see.
    pub(crate) fn remove_group_occurrence(
        &self,
        group: &NodeKey,
        path: &Path,
        minimum: u32,
    ) -> bool {
        if !self.can_remove(group, path, minimum) {
            return false;
        }
        let last = self.shown(group, path).saturating_sub(1);
        let inside = descend(path, last);
        self.values
            .update(|values| values.remove_under(group, &inside));
        self.shown.update(|shown| {
            shown.retain(|(key, at, _)| {
                !(key.steps.starts_with(&group.steps) && at.starts_with(&inside))
            });
        });
        self.set_shown(group, path, last);
        true
    }

    /// Hides the last repeat of a FIELD and forgets what it held.
    ///
    /// A field's repeats are addressed by the occurrence rather than by the
    /// occurrence path, so what goes with the repeat is the one value at that
    /// occurrence and never a subtree.
    pub(crate) fn remove_field_occurrence(&self, key: &NodeKey, path: &Path, minimum: u32) -> bool {
        if !self.can_remove(key, path, minimum) {
            return false;
        }
        let last = self.shown(key, path).saturating_sub(1);
        self.values.update(|values| {
            // The removed repeat's value goes with it, deliberately: an
            // occurrence a clinician removed is one they said is not there.
            drop(values.remove_in(key, path, last));
        });
        self.set_shown(key, path, last);
        true
    }

    /// Every value the form holds, for the request that submits it.
    pub(crate) fn values(&self) -> FormValues {
        self.values.read().clone()
    }

    /// Records how many occurrences of one group are showing.
    fn set_shown(&self, group: &NodeKey, path: &Path, count: usize) {
        self.shown.update(|shown| {
            if let Some(entry) = shown
                .iter_mut()
                .find(|(key, at, _)| key == group && at.as_slice() == path.as_ref())
            {
                entry.2 = count;
            } else {
                shown.push((group.clone(), path.to_vec(), count));
            }
        });
    }
}

/// Records the occurrences a fresh form shows, one entry per repeating group.
///
/// The minimum the template requires, and never fewer than one: a group a
/// clinician cannot see is a group they cannot fill, and a template that
/// permits none still has to offer the first.
fn seed(group: &FormGroup, path: &[usize], shown: &mut Vec<(NodeKey, Vec<usize>, usize)>) {
    for item in &group.items {
        let FormItem::Group(ref child) = *item else {
            continue;
        };
        if child.occurrences.is_repeatable() {
            let count = (child.occurrences.minimum as usize).max(1);
            shown.push((child.key.clone(), path.to_vec(), count));
            for index in 0..count {
                let mut inside = path.to_vec();
                inside.push(index);
                seed(child, &inside, shown);
            }
        } else {
            seed(child, path, shown);
        }
    }
}
