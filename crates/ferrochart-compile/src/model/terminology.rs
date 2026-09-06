// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The terminology one archetype contributes to a template.
//!
//! Membership and display text are separate questions. A local `at`-code
//! carries its own rubric here (openEHR AM Release-2.3.0 `AOM1.4.html`
//! section 7.3.2, where `ARCHETYPE_TERM.text` and `.description` are
//! mandatory); an enumerated external code carries none, and its display text
//! comes from a terminology server.

use std::collections::BTreeMap;

use crate::model::ids::{LanguageTag, LocalCode, TerminologyName};

/// The rubric one local code carries in one language.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3.2 (`ARCHETYPE_TERM`) and
/// `AOM2.html` section 7.3.4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermDefinition {
    text: String,
    description: Option<String>,
    other_items: BTreeMap<String, String>,
}

impl TermDefinition {
    /// A rubric with its mandatory text and its optional description.
    #[must_use]
    pub fn new(
        text: impl Into<String>,
        description: Option<String>,
        other_items: BTreeMap<String, String>,
    ) -> Self {
        Self {
            text: text.into(),
            description,
            other_items,
        }
    }

    /// The short rubric.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The long rubric, where the archetype states one.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Every other item the term carries.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3.2 defines
    /// `ARCHETYPE_TERM.other_items` as a free hash and reserves no keys for
    /// form use, so nothing here is interpreted.
    #[must_use]
    pub fn other_items(&self) -> &BTreeMap<String, String> {
        &self.other_items
    }
}

/// A code in some terminology other than the archetype's own.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExternalTerm {
    terminology: TerminologyName,
    value: String,
}

impl ExternalTerm {
    /// A binding target: the terminology, and the code or URI the template
    /// states, verbatim.
    #[must_use]
    pub fn new(terminology: TerminologyName, value: impl Into<String>) -> Self {
        Self {
            terminology,
            value: value.into(),
        }
    }

    /// The terminology the binding names.
    #[must_use]
    pub fn terminology(&self) -> &TerminologyName {
        &self.terminology
    }

    /// The code or URI the binding states, verbatim.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// One archetype's terminology, as both readers fill it.
///
/// Every map is ordered, so a walk over this type feeds a deterministic
/// result (`.claude/rules/reliability.md`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Terminology {
    definitions: BTreeMap<LanguageTag, BTreeMap<LocalCode, TermDefinition>>,
    bindings: BTreeMap<LocalCode, BTreeMap<TerminologyName, ExternalTerm>>,
    constraint_bindings: BTreeMap<LocalCode, BTreeMap<TerminologyName, ExternalTerm>>,
    value_sets: BTreeMap<LocalCode, Vec<LocalCode>>,
}

impl Terminology {
    /// An empty terminology.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the rubric `code` carries in `language`, replacing any rubric
    /// already recorded for that pair.
    pub fn insert_definition(
        &mut self,
        language: LanguageTag,
        code: LocalCode,
        term: TermDefinition,
    ) {
        self.definitions
            .entry(language)
            .or_default()
            .insert(code, term);
    }

    /// Records that `code` binds to `target` in `target`'s terminology.
    pub fn insert_binding(&mut self, code: LocalCode, target: ExternalTerm) {
        self.bindings
            .entry(code)
            .or_default()
            .insert(target.terminology().clone(), target);
    }

    /// Records that the value set `code` names resolves through `target`.
    pub fn insert_constraint_binding(&mut self, code: LocalCode, target: ExternalTerm) {
        self.constraint_bindings
            .entry(code)
            .or_default()
            .insert(target.terminology().clone(), target);
    }

    /// Records the members of the value set `code` names.
    pub fn insert_value_set(&mut self, code: LocalCode, members: Vec<LocalCode>) {
        self.value_sets.insert(code, members);
    }

    /// The rubric `code` carries in `language`.
    #[must_use]
    pub fn rubric(&self, language: &LanguageTag, code: &LocalCode) -> Option<&TermDefinition> {
        self.definitions.get(language)?.get(code)
    }

    /// Every language this terminology states rubrics in.
    pub fn languages(&self) -> impl Iterator<Item = &LanguageTag> {
        self.definitions.keys()
    }

    /// Every external code `code` binds to, keyed by terminology.
    #[must_use]
    pub fn bindings(&self, code: &LocalCode) -> Option<&BTreeMap<TerminologyName, ExternalTerm>> {
        self.bindings.get(code)
    }

    /// Every target the value set `code` names resolves through.
    #[must_use]
    pub fn constraint_bindings(
        &self,
        code: &LocalCode,
    ) -> Option<&BTreeMap<TerminologyName, ExternalTerm>> {
        self.constraint_bindings.get(code)
    }

    /// The members of the value set `code` names, where the archetype
    /// enumerates them.
    #[must_use]
    pub fn value_set(&self, code: &LocalCode) -> Option<&[LocalCode]> {
        self.value_sets.get(code).map(Vec::as_slice)
    }

    /// Whether the terminology states nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
            && self.bindings.is_empty()
            && self.constraint_bindings.is_empty()
            && self.value_sets.is_empty()
    }
}
