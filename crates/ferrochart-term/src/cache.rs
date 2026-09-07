// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The expansion cache, keyed per template and language.
//!
//! No specification governs caching a terminology expansion: our own design.
//! HL7 FHIR R4 4.0.1 publishes no cache-validity mechanism for the operations
//! of `terminology-service.html`, so nothing on the wire tells a client that
//! an expansion has moved, and the rules below are FerroCHART's.

use std::collections::BTreeMap;
use std::sync::Mutex;

use ferrochart_form::ids::{LanguageTag, TemplateId};

use crate::error::TermError;
use crate::resolution::Resolution;

/// What one cached expansion is filed under.
///
/// The key is three parts, and each one is there because leaving it out would
/// serve a wrong answer:
///
/// - **the template**, because two templates may bind the same `ac`-code to
///   different targets, and because a form is recompiled as a whole;
/// - **the language**, because `displayLanguage` changes the display text of
///   every member and the codes are the same set under two names;
/// - **the request**, as the FHIR JSON of the `Parameters` that went on the
///   wire. That is exact by construction: a filter, a page offset, a count and
///   an inline value set all change the request, so they all change the key,
///   and nothing can be served for a question it does not answer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CacheKey {
    /// The template the field belongs to.
    pub template: TemplateId,
    /// The language the display text was asked for in.
    pub language: LanguageTag,
    /// The request, as the FHIR JSON it was sent as.
    pub request: String,
}

/// Expansions already answered for, held for the life of this cache.
///
/// **What invalidates an entry.** Nothing on a timer, and nothing the server
/// says: a FHIR terminology server can change what a canonical URL expands to
/// and R4 gives it no way to tell a client so. An entry therefore lives until
/// the process drops the cache, until [`ExpansionCache::forget_template`] is
/// called for its template, which is what a recompile of that template does,
/// or until [`ExpansionCache::clear`] empties it. A deployment that needs a
/// shorter life gives each request round its own cache rather than expiring
/// entries inside one.
#[derive(Debug, Default)]
pub struct ExpansionCache {
    entries: Mutex<BTreeMap<CacheKey, Resolution>>,
}

impl ExpansionCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The resolution already answered for `key`, where there is one.
    ///
    /// # Errors
    /// [`TermError::CachePoisoned`] when a panic in another task left the
    /// cache locked. A poisoned lock is reported rather than read as a miss,
    /// because a miss would turn a broken process into silent extra traffic.
    pub fn get(&self, key: &CacheKey) -> Result<Option<Resolution>, TermError> {
        let entries = self.entries.lock().map_err(|_| TermError::CachePoisoned)?;
        Ok(entries.get(key).cloned())
    }

    /// Files `resolution` under `key`, replacing anything already there.
    ///
    /// # Errors
    /// [`TermError::CachePoisoned`] when a panic in another task left the
    /// cache locked.
    pub fn put(&self, key: CacheKey, resolution: Resolution) -> Result<(), TermError> {
        let mut entries = self.entries.lock().map_err(|_| TermError::CachePoisoned)?;
        entries.insert(key, resolution);
        Ok(())
    }

    /// Drops every entry filed under `template`.
    ///
    /// This is what a recompile of one template calls, because a revision can
    /// bind the same `ac`-code to another target.
    ///
    /// # Errors
    /// [`TermError::CachePoisoned`] when a panic in another task left the
    /// cache locked.
    pub fn forget_template(&self, template: &TemplateId) -> Result<usize, TermError> {
        let mut entries = self.entries.lock().map_err(|_| TermError::CachePoisoned)?;
        let before = entries.len();
        entries.retain(|key, _| key.template != *template);
        Ok(before.saturating_sub(entries.len()))
    }

    /// Drops every entry.
    ///
    /// # Errors
    /// [`TermError::CachePoisoned`] when a panic in another task left the
    /// cache locked.
    pub fn clear(&self) -> Result<(), TermError> {
        let mut entries = self.entries.lock().map_err(|_| TermError::CachePoisoned)?;
        entries.clear();
        Ok(())
    }

    /// How many expansions the cache holds.
    ///
    /// # Errors
    /// [`TermError::CachePoisoned`] when a panic in another task left the
    /// cache locked.
    pub fn len(&self) -> Result<usize, TermError> {
        let entries = self.entries.lock().map_err(|_| TermError::CachePoisoned)?;
        Ok(entries.len())
    }

    /// Whether the cache holds nothing.
    ///
    /// # Errors
    /// [`TermError::CachePoisoned`] when a panic in another task left the
    /// cache locked.
    pub fn is_empty(&self) -> Result<bool, TermError> {
        Ok(self.len()? == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheKey, ExpansionCache};
    use crate::resolution::{Origin, Resolution};
    use ferrochart_form::ids::{LanguageTag, TemplateId};

    fn key(template: &str, language: &str, request: &str) -> CacheKey {
        CacheKey {
            template: TemplateId::new(template),
            language: LanguageTag::new(language),
            request: request.to_owned(),
        }
    }

    fn resolution(language: &str) -> Resolution {
        Resolution::complete(Vec::new(), Origin::Server, LanguageTag::new(language))
    }

    #[test]
    fn two_languages_of_one_request_are_two_entries() {
        let cache = ExpansionCache::new();
        cache
            .put(key("ferro.v1", "en", "{}"), resolution("en"))
            .expect("the cache accepts an entry");
        cache
            .put(key("ferro.v1", "nl", "{}"), resolution("nl"))
            .expect("the cache accepts an entry");
        assert_eq!(cache.len().expect("the cache answers"), 2);
        assert_eq!(
            cache
                .get(&key("ferro.v1", "nl", "{}"))
                .expect("the cache answers")
                .map(|entry| entry.language),
            Some(LanguageTag::new("nl"))
        );
    }

    #[test]
    fn a_filtered_request_is_not_served_from_an_unfiltered_one() {
        let cache = ExpansionCache::new();
        cache
            .put(
                key("ferro.v1", "en", r#"{"filter":"abdo"}"#),
                resolution("en"),
            )
            .expect("the cache accepts an entry");
        assert_eq!(
            cache
                .get(&key("ferro.v1", "en", "{}"))
                .expect("the cache answers"),
            None
        );
    }

    #[test]
    fn recompiling_one_template_forgets_only_its_own_entries() {
        let cache = ExpansionCache::new();
        cache
            .put(key("ferro.v1", "en", "{}"), resolution("en"))
            .expect("the cache accepts an entry");
        cache
            .put(key("ferro.v2", "en", "{}"), resolution("en"))
            .expect("the cache accepts an entry");
        assert_eq!(
            cache
                .forget_template(&TemplateId::new("ferro.v1"))
                .expect("the cache answers"),
            1
        );
        assert!(
            cache
                .get(&key("ferro.v2", "en", "{}"))
                .expect("the cache answers")
                .is_some()
        );
    }
}
