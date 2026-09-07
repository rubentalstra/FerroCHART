// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `ValueSet/$expand`, which fills a picker the template does not enumerate.
//!
//! All citations are HL7 FHIR R4 4.0.1 `valueset-operation-expand.html`
//! section 4.9.15.1 unless another page is named. The request is formatted
//! against the generated `ValueSetExpandRequest` contract rather than against
//! a hand-built parameter list, and the response is the `ValueSet` resource
//! itself, because `operations.html` section 3.2.0.6.2 says an operation whose
//! single output parameter is named `return` and is a resource answers with
//! "the resource itself".

use fhir_types::codec::{Json, Path};
use fhir_types::r4::operations::value_set_expand::ValueSetExpandRequest;
use fhir_types::r4::parameters::Parameters;
use fhir_types::r4::primitives::{Code, Integer, String as FhirString, Uri};
use fhir_types::r4::value_set::{ValueSet, ValueSetExpansionContains};

use ferrochart_form::ids::{LanguageTag, TerminologyName};
use ferrochart_form::value;

use crate::client::TermClient;
use crate::error::TermError;
use crate::resolution::{Origin, Resolution, ResolvedCode};
use crate::target::ExpansionTarget;

/// The path the operation is invoked at, as a type-level operation.
const ENDPOINT: &[&str] = &["ValueSet", "$expand"];

/// How much of an expansion one page carries by default.
///
/// No specification governs the number: our own design. `count` is declared
/// `0..1` and left to the client, and a picker that a person scrolls does not
/// need a whole terminology in one answer.
const PAGE: i32 = 200;

/// What an expansion asks for beyond the value set itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandOptions {
    /// The text filter, which the specification describes as applied "to
    /// restrict the codes that are returned".
    pub filter: Option<String>,
    /// How many codes one page carries.
    pub count: i32,
    /// Where the page starts.
    pub offset: i32,
    /// Whether to leave out the concepts a server marks as not selectable.
    pub active_only: Option<bool>,
}

impl Default for ExpandOptions {
    fn default() -> Self {
        Self {
            filter: None,
            count: PAGE,
            offset: 0,
            active_only: None,
        }
    }
}

impl ExpandOptions {
    /// A first page with no filter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The same options restricted to codes matching `filter`.
    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// The same options starting at `offset`.
    #[must_use]
    pub fn at_offset(mut self, offset: i32) -> Self {
        self.offset = offset;
        self
    }

    /// The same options carrying `count` codes per page.
    #[must_use]
    pub fn with_count(mut self, count: i32) -> Self {
        self.count = count;
        self
    }
}

impl TermClient {
    /// Expands `target`, with display text in `language`.
    ///
    /// # Errors
    /// [`TermError::UnknownValueSet`] when the server does not hold the value
    /// set, [`TermError::Refused`] when it refuses the expansion (an
    /// `OperationOutcome` with code `too-costly` among the reasons the
    /// specification names), and one variant per status family the
    /// specification publishes. Nothing is flattened into an empty picker.
    pub async fn expand(
        &self,
        target: &ExpansionTarget,
        language: &LanguageTag,
        options: &ExpandOptions,
    ) -> Result<Resolution, TermError> {
        let parameters = expand_request(target, language, options).to_parameters();
        self.expand_with(&parameters, &target.describe(), language)
            .await
    }

    /// Sends `parameters` as an expansion already formatted, and reads the
    /// `ValueSet` the server answers with.
    pub(crate) async fn expand_with(
        &self,
        parameters: &Parameters,
        described: &str,
        language: &LanguageTag,
    ) -> Result<Resolution, TermError> {
        let object = self.invoke(ENDPOINT, parameters, described).await?;
        let mut path = Path::lenient("ValueSet");
        let value_set =
            ValueSet::from_json(&object, &mut path).map_err(|source| TermError::Body {
                expected: "a ValueSet",
                source,
            })?;
        Ok(resolution_of(&value_set, language))
    }
}

/// The `in` parameters one expansion sends.
///
/// A named target goes as `url` and a supplied one as `valueSet`, which the
/// specification declares `0..1` each.
pub(crate) fn expand_request(
    target: &ExpansionTarget,
    language: &LanguageTag,
    options: &ExpandOptions,
) -> ValueSetExpandRequest {
    let inline = match *target {
        ExpansionTarget::Inline(ref value_set) => Some((**value_set).clone()),
        _ => None,
    };
    ValueSetExpandRequest {
        url: target.canonical_url().map(Uri::from),
        value_set: inline,
        filter: options.filter.as_deref().map(FhirString::from),
        count: Some(Integer::from(options.count)),
        offset: Some(Integer::from(options.offset)),
        display_language: Some(Code::from(language.as_str())),
        active_only: options.active_only.map(Into::into),
        ..ValueSetExpandRequest::default()
    }
}

/// The codes an expanded `ValueSet` carries.
///
/// HL7 FHIR R4 4.0.1 `valueset.html` gives `ValueSet.expansion.contains` as
/// the codes of the expansion and nests it, so a grouper's children are read
/// too. Every entry the server sent is kept, because dropping one would change
/// the set the server answered with.
fn resolution_of(value_set: &ValueSet, language: &LanguageTag) -> Resolution {
    let Some(ref expansion) = value_set.expansion else {
        return Resolution {
            codes: Vec::new(),
            origin: Origin::Server,
            language: language.clone(),
            total: Some(0),
        };
    };
    let mut codes = Vec::new();
    for concept in &expansion.contains {
        push_contains(concept, &mut codes);
    }
    let total = expansion
        .total
        .as_ref()
        .and_then(|total| total.value)
        .and_then(|total| u32::try_from(total).ok());
    Resolution {
        codes,
        origin: Origin::Server,
        language: language.clone(),
        total,
    }
}

/// Adds one expansion entry and everything nested under it.
fn push_contains(concept: &ValueSetExpansionContains, out: &mut Vec<ResolvedCode>) {
    if let (Some(system), Some(code)) = (
        concept.system.as_ref().and_then(|uri| uri.value.as_deref()),
        concept.code.as_ref().and_then(|code| code.value.as_deref()),
    ) {
        let mut resolved = ResolvedCode::new(value::Code::new(TerminologyName::new(system), code));
        resolved.display = concept
            .display
            .as_ref()
            .and_then(|display| display.value.clone());
        out.push(resolved);
    }
    for nested in &concept.contains {
        push_contains(nested, out);
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpandOptions, expand_request};
    use crate::target::ExpansionTarget;
    use ferrochart_form::ids::LanguageTag;

    #[test]
    fn an_expansion_sends_only_what_it_was_given() {
        let target = ExpansionTarget::Canonical("https://example.org/ValueSet/ferro".to_owned());
        let request = expand_request(&target, &LanguageTag::new("nl"), &ExpandOptions::new());
        let names: Vec<String> = request
            .to_parameters()
            .parameter
            .iter()
            .filter_map(|parameter| parameter.name.value.clone())
            .collect();
        // The order is the operation's own: the generated contract writes the
        // parameters as HL7 FHIR R4 4.0.1 declares them, so `offset` precedes
        // `count`.
        assert_eq!(names, ["url", "offset", "count", "displayLanguage"]);
    }

    #[test]
    fn a_filter_reaches_the_wire_as_the_filter_parameter() {
        let target = ExpansionTarget::Canonical("https://example.org/ValueSet/ferro".to_owned());
        let options = ExpandOptions::new().with_filter("abdo").at_offset(20);
        let request = expand_request(&target, &LanguageTag::new("en"), &options);
        assert_eq!(
            request.filter.and_then(|text| text.value).as_deref(),
            Some("abdo")
        );
        assert_eq!(request.offset.and_then(|offset| offset.value), Some(20));
    }

    #[test]
    fn a_snomed_expression_is_sent_as_a_url_and_never_parsed() {
        let target = ExpansionTarget::SnomedExpression("<<73211009".to_owned());
        let request = expand_request(&target, &LanguageTag::new("en"), &ExpandOptions::new());
        let url = request.url.and_then(|uri| uri.value).unwrap_or_default();
        assert!(
            url.starts_with("http://snomed.info/sct?fhir_vs=ecl/"),
            "{url}"
        );
    }
}
