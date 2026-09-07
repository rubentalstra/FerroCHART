// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One entry point that decides where a coded field's codes come from.
//!
//! `docs/architecture.md` section 7: "One code path serves every kind. The
//! compiler resolves what the template carries, then asks the terminology
//! client for what is missing." This is that client. The default path is
//! local, and a request happens only where the template names a target or
//! states a code it carries no rubric for.

use fhir_types::codec::Json;

use ferrochart_compile::model::terminology::Terminology;
use ferrochart_form::ids::{LanguageTag, TemplateId, TerminologyName, local_terminology};
use ferrochart_form::value::{CodedOption, EnumeratedSet, ExpansionSource, ValueSet};

use crate::cache::{CacheKey, ExpansionCache};
use crate::client::TermClient;
use crate::error::TermError;
use crate::expand::{ExpandOptions, expand_request};
use crate::local;
use crate::resolution::{Origin, Resolution, ResolvedCode};
use crate::system::{OPENEHR, SystemMap};
use crate::target::ExpansionTarget;

/// Resolves the codes behind a coded field, locally where it can.
///
/// A resolver with no client answers everything the template and openEHR's own
/// terminology carry, and reports [`TermError::NoServer`] for the rest, so a
/// deployment with no terminology server still renders 97.6% of coded fields
/// (`docs/architecture.md` section 7) and never shows an empty picker for the
/// remainder.
#[derive(Debug, Default)]
pub struct Resolver {
    client: Option<TermClient>,
    cache: ExpansionCache,
}

impl Resolver {
    /// A resolver that reaches no server.
    #[must_use]
    pub fn local() -> Self {
        Self::default()
    }

    /// A resolver that asks `client` for what the template does not carry.
    #[must_use]
    pub fn with_server(client: TermClient) -> Self {
        Self {
            client: Some(client),
            cache: ExpansionCache::new(),
        }
    }

    /// The cache this resolver files server expansions in.
    #[must_use]
    pub fn cache(&self) -> &ExpansionCache {
        &self.cache
    }

    /// The server this resolver asks, where one is configured.
    #[must_use]
    pub fn client(&self) -> Option<&TermClient> {
        self.client.as_ref()
    }

    /// Resolves `set` for a field of `template`, with display text in
    /// `language`.
    ///
    /// # Errors
    /// [`TermError::NoServer`] when the binding names a target and no client
    /// is configured, [`TermError::AmbiguousBinding`] when the template binds
    /// a value set to several terminologies and pins none,
    /// [`TermError::UnresolvableBinding`] when the target is a form no FHIR
    /// request can name, and one variant per status family a server answers
    /// with. None of them is an empty picker.
    pub async fn resolve(
        &self,
        template: &TemplateId,
        set: &ValueSet,
        language: &LanguageTag,
        options: &ExpandOptions,
    ) -> Result<Resolution, TermError> {
        match *set {
            ValueSet::Enumerated(ref enumerated) => self.enumerated(enumerated, language).await,
            ValueSet::OpenTerminology { ref terminology } => {
                let system = self.system(terminology)?;
                let target = ExpansionTarget::Canonical(system);
                self.expand(template, &target, language, options).await
            }
            ValueSet::Expansion(ref source) => {
                let target = target_of(source)?;
                self.expand(template, &target, language, options).await
            }
            ValueSet::Unconstrained => Ok(Resolution {
                codes: Vec::new(),
                origin: Origin::Unconstrained,
                language: language.clone(),
                total: None,
            }),
            _ => Err(TermError::UnsupportedValueSet),
        }
    }

    /// Resolves `set` for a field of `template`, reading `terminology` first.
    ///
    /// An `ac`-code the archetype enumerates in its own terminology resolves
    /// out of it with no request, which is the 70.6% path of
    /// `docs/architecture.md` section 7. Everything else falls through to
    /// [`Resolver::resolve`].
    ///
    /// # Errors
    /// Whatever [`Resolver::resolve`] returns for a set the archetype does not
    /// enumerate.
    pub async fn resolve_in_template(
        &self,
        template: &TemplateId,
        terminology: &Terminology,
        set: &ValueSet,
        language: &LanguageTag,
        options: &ExpandOptions,
    ) -> Result<Resolution, TermError> {
        if let ValueSet::Expansion(ExpansionSource::ConstraintCode { ref code, .. }) = *set
            && let Some(resolution) = local::archetype_value_set(terminology, code, language)
        {
            return Ok(resolution);
        }
        self.resolve(template, set, language, options).await
    }

    /// The codes a set the template enumerates permits, with their rubrics.
    async fn enumerated(
        &self,
        set: &EnumeratedSet,
        language: &LanguageTag,
    ) -> Result<Resolution, TermError> {
        let mut codes: Vec<ResolvedCode> = set
            .options
            .iter()
            .map(|option| stated(option, language))
            .collect();
        if set.terminology == local_terminology() {
            return Ok(Resolution::complete(
                codes,
                Origin::Template,
                language.clone(),
            ));
        }
        if set.terminology.as_str() == OPENEHR {
            for resolved in &mut codes {
                if resolved.display.is_none() {
                    resolved.display = local::openehr_display(&resolved.code.code, language);
                }
            }
            return Ok(Resolution::complete(
                codes,
                Origin::OpenehrTerminology,
                language.clone(),
            ));
        }
        // Membership is settled by the template whatever happens next, so a
        // set whose rubrics are all present, and a deployment with no
        // terminology server, both come back as a template resolution. A
        // picker of bare codes is poor and it is honest.
        let missing = codes.iter().any(|resolved| resolved.display.is_none());
        let Some(client) = self.client.as_ref().filter(|_| missing) else {
            return Ok(Resolution::complete(
                codes,
                Origin::Template,
                language.clone(),
            ));
        };
        for resolved in &mut codes {
            if resolved.display.is_some() {
                continue;
            }
            let concept = client
                .lookup(&set.terminology, &resolved.code.code, language)
                .await?;
            resolved.display = Some(concept.display);
        }
        Ok(Resolution::complete(
            codes,
            Origin::TemplateDisplayedByServer,
            language.clone(),
        ))
    }

    /// One server expansion, served from the cache where it has been asked
    /// before.
    async fn expand(
        &self,
        template: &TemplateId,
        target: &ExpansionTarget,
        language: &LanguageTag,
        options: &ExpandOptions,
    ) -> Result<Resolution, TermError> {
        let described = target.describe();
        let Some(client) = self.client.as_ref() else {
            return Err(TermError::NoServer { target: described });
        };
        let parameters = expand_request(target, language, options).to_parameters();
        let request = parameters
            .to_json()
            .map_err(|source| TermError::Request { source })
            .and_then(|object| {
                serde_json::to_string(&object).map_err(|source| TermError::NotJson { source })
            })?;
        let key = CacheKey {
            template: template.clone(),
            language: language.clone(),
            request,
        };
        if let Some(cached) = self.cache.get(&key)? {
            return Ok(cached);
        }
        let resolution = client
            .expand_with(&parameters, &described, language)
            .await?;
        self.cache.put(key, resolution.clone())?;
        Ok(resolution)
    }

    /// The FHIR system URI `terminology` names.
    fn system(&self, terminology: &TerminologyName) -> Result<String, TermError> {
        let resolved = match self.client.as_ref() {
            Some(client) => client.systems().resolve(terminology),
            None => SystemMap::new().resolve(terminology),
        };
        resolved
            .map(|system| system.uri)
            .ok_or_else(|| TermError::UnknownSystem {
                terminology: terminology.clone(),
            })
    }
}

/// One option as the template states it, in `language`.
fn stated(option: &CodedOption, language: &LanguageTag) -> ResolvedCode {
    ResolvedCode {
        code: option.code.clone(),
        display: option.label.get(language).map(str::to_owned),
        description: option.description.get(language).map(str::to_owned),
    }
}

/// The target a binding that names one resolves to.
///
/// A value set bound to several terminologies with no pin is refused rather
/// than guessed at: openEHR AM Release-2.3.0 `ADL2.html` section 8.2 warns
/// that a binding implies no coverage, so the terminologies are alternatives
/// the author chose between rather than one set spelled twice.
fn target_of(source: &ExpansionSource) -> Result<ExpansionTarget, TermError> {
    match *source {
        ExpansionSource::ReferenceSet { ref uri } => ExpansionTarget::from_uri(uri),
        ExpansionSource::ConstraintCode {
            ref code,
            ref bindings,
            ref pinned_terminology,
        } => {
            if let Some(pinned) = pinned_terminology.as_ref()
                && let Some(uri) = bindings.get(pinned)
            {
                return ExpansionTarget::from_uri(uri);
            }
            let mut targets = bindings.values();
            match (targets.next(), targets.next()) {
                (Some(only), None) => ExpansionTarget::from_uri(only),
                (Some(_), Some(_)) => Err(TermError::AmbiguousBinding {
                    code: code.clone(),
                    terminologies: bindings.keys().cloned().collect(),
                }),
                _ => Err(TermError::UnresolvableBinding {
                    uri: code.as_str().to_owned(),
                }),
            }
        }
        _ => Err(TermError::UnsupportedValueSet),
    }
}

#[cfg(test)]
mod tests {
    use super::target_of;
    use crate::error::TermError;
    use crate::target::ExpansionTarget;
    use ferrochart_form::ids::{LocalCode, TerminologyName};
    use ferrochart_form::value::ExpansionSource;
    use std::collections::BTreeMap;

    fn bindings(entries: &[(&str, &str)]) -> BTreeMap<TerminologyName, String> {
        entries
            .iter()
            .map(|&(name, uri)| (TerminologyName::new(name), uri.to_owned()))
            .collect()
    }

    #[test]
    fn a_pinned_terminology_decides_which_binding_is_expanded() {
        let source = ExpansionSource::ConstraintCode {
            code: LocalCode::new("ac0.1"),
            bindings: bindings(&[
                ("LOINC", "https://example.org/ValueSet/loinc"),
                ("SNOMED-CT", "https://example.org/ValueSet/snomed"),
            ]),
            pinned_terminology: Some(TerminologyName::new("SNOMED-CT")),
        };
        assert_eq!(
            target_of(&source).expect("the pin decides"),
            ExpansionTarget::Canonical("https://example.org/ValueSet/snomed".to_owned())
        );
    }

    #[test]
    fn several_bindings_with_no_pin_are_refused_rather_than_guessed_at() {
        let source = ExpansionSource::ConstraintCode {
            code: LocalCode::new("ac0.1"),
            bindings: bindings(&[
                ("LOINC", "https://example.org/ValueSet/loinc"),
                ("SNOMED-CT", "https://example.org/ValueSet/snomed"),
            ]),
            pinned_terminology: None,
        };
        match target_of(&source).expect_err("the template pins none") {
            TermError::AmbiguousBinding {
                code,
                terminologies,
            } => {
                assert_eq!(code.as_str(), "ac0.1");
                assert_eq!(terminologies.len(), 2);
            }
            other => panic!("the failure is {other:?}"),
        }
    }

    #[test]
    fn an_ac_code_with_no_binding_at_all_is_unresolvable() {
        let source = ExpansionSource::ConstraintCode {
            code: LocalCode::new("ac0.1"),
            bindings: BTreeMap::new(),
            pinned_terminology: None,
        };
        assert!(matches!(
            target_of(&source),
            Err(TermError::UnresolvableBinding { .. })
        ));
    }
}
