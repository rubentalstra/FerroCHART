// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The display text a template carries, resolved per language.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3.2 gives every local code
//! an `ARCHETYPE_TERM` with a mandatory `text` and an optional `description`,
//! per language, and `AOM2.html` section 7.3.4 does the same. Which of the two
//! becomes the label and which becomes the help text is not governed by any
//! specification.

use std::collections::BTreeSet;

use ferrochart_form::ids::LanguageTag as FormLanguageTag;
use ferrochart_form::text::Localized;

use crate::model::ids::{ArchetypeId, LocalCode};
use crate::model::node::ConstraintTemplate;

/// The template's terminology, read as display text.
#[derive(Debug)]
pub(crate) struct Terms<'a> {
    template: &'a ConstraintTemplate,
}

impl<'a> Terms<'a> {
    /// Reads `template`'s terminology.
    pub(crate) const fn new(template: &'a ConstraintTemplate) -> Self {
        Self { template }
    }

    /// The label and the help text `code` carries in `scope`.
    ///
    /// The short rubric becomes the label and the long one becomes the help
    /// text. No specification governs the split, and this is the only
    /// distinction the two fields of an `ARCHETYPE_TERM` support.
    pub(crate) fn rubrics(&self, scope: &ArchetypeId, code: &LocalCode) -> (Localized, Localized) {
        let mut label = Localized::empty();
        let mut help = Localized::empty();
        let Some(terminology) = self.template.terminology(scope) else {
            return (label, help);
        };
        for language in terminology.languages() {
            let Some(term) = terminology.rubric(language, code) else {
                continue;
            };
            let tag = FormLanguageTag::new(language.as_str());
            label.insert(tag.clone(), term.text());
            if let Some(description) = term.description() {
                help.insert(tag, description);
            }
        }
        (label, help)
    }

    /// The members of the value set `code` names in `scope`, where the
    /// archetype enumerates them.
    pub(crate) fn value_set(
        &self,
        scope: &ArchetypeId,
        code: &LocalCode,
    ) -> Option<&'a [LocalCode]> {
        self.template.terminology(scope)?.value_set(code)
    }

    /// Every language any rubric of the template is stated in, in tag order.
    pub(crate) fn languages(&self) -> Vec<FormLanguageTag> {
        let mut tags: BTreeSet<FormLanguageTag> = BTreeSet::new();
        for terminology in self.template.terminologies().values() {
            for language in terminology.languages() {
                tags.insert(FormLanguageTag::new(language.as_str()));
            }
        }
        tags.into_iter().collect()
    }
}
