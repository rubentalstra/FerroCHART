// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Text a person reads, in every language the template states it in.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::LanguageTag;

/// One piece of display text, per language.
///
/// The map is ordered, so the serialisation of a form definition is
/// byte-deterministic (`.claude/rules/reliability.md`). A language the
/// template states no text in is absent rather than empty, so a renderer can
/// tell "not translated" from "translated to nothing".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Localized {
    /// The text, keyed by the language tag the template states it under.
    pub by_language: BTreeMap<LanguageTag, String>,
}

impl Localized {
    /// An empty piece of text, stated in no language.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// The text stated in exactly one language.
    #[must_use]
    pub fn in_language(language: LanguageTag, text: impl Into<String>) -> Self {
        let mut by_language = BTreeMap::new();
        by_language.insert(language, text.into());
        Self { by_language }
    }

    /// Records `text` as the text of `language`, replacing any already there.
    pub fn insert(&mut self, language: LanguageTag, text: impl Into<String>) {
        self.by_language.insert(language, text.into());
    }

    /// The text stated in `language`, where the template states one.
    #[must_use]
    pub fn get(&self, language: &LanguageTag) -> Option<&str> {
        self.by_language.get(language).map(String::as_str)
    }

    /// Whether the template states this text in no language at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_language.is_empty()
    }

    /// Every language this text is stated in, in tag order.
    pub fn languages(&self) -> impl Iterator<Item = &LanguageTag> {
        self.by_language.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::Localized;
    use crate::ids::LanguageTag;

    #[test]
    fn the_languages_come_back_in_tag_order_whatever_order_they_went_in() {
        let mut text = Localized::empty();
        text.insert(LanguageTag::new("sv"), "Sittande");
        text.insert(LanguageTag::new("en"), "Sitting");
        text.insert(LanguageTag::new("nl"), "Zittend");
        let order: Vec<&str> = text.languages().map(LanguageTag::as_str).collect();
        assert_eq!(order, ["en", "nl", "sv"]);
    }

    #[test]
    fn an_untranslated_language_is_absent_rather_than_empty() {
        let text = Localized::in_language(LanguageTag::new("en"), "Sitting");
        assert_eq!(text.get(&LanguageTag::new("en")), Some("Sitting"));
        assert_eq!(text.get(&LanguageTag::new("de")), None);
    }
}
