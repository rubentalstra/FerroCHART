// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `CodeSystem/$lookup`, which supplies the display text of an enumerated
//! external code.
//!
//! All citations are HL7 FHIR R4 4.0.1 `codesystem-operation-lookup.html`
//! section 4.8.21.1 unless another page is named.
//!
//! `docs/architecture.md` section 7 measures 228 enumerated external codes in
//! the corpus and finds none of them carries a rubric in the template, so this
//! is the call that fills in what the template cannot. Membership still comes
//! from the template: nothing here changes which codes a field permits.

use fhir_types::codec::{Json, Path};
use fhir_types::r4::operations::code_system_lookup::{
    CodeSystemLookupRequest, CodeSystemLookupResponse,
};
use fhir_types::r4::parameters::Parameters;
use fhir_types::r4::primitives::{Code, String as FhirString, Uri};

use ferrochart_form::ids::{LanguageTag, TerminologyName};

use crate::client::TermClient;
use crate::error::TermError;
use crate::system::System;

/// The path the operation is invoked at.
const ENDPOINT: &[&str] = &["CodeSystem", "$lookup"];

/// The operation, for an error that names it.
const OPERATION: &str = "CodeSystem/$lookup";

/// The one property this client asks for.
///
/// The specification declares `property` as "a property that the client wishes
/// to be returned in the output. If no properties are specified, the server
/// chooses what to return", and names `display` among the properties defined
/// for all code systems. FerroCHART wants the display text and nothing else,
/// and asking for it keeps the answer inside the out parameters R4 declares.
const DISPLAY: &str = "display";

/// What a code system says about one code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    /// The display name of the code system, which the specification declares
    /// as a `1..1` out parameter.
    pub system_name: String,
    /// The version the answer is based on, where the server states one.
    pub system_version: Option<String>,
    /// The preferred display for the concept, a `1..1` out parameter.
    pub display: String,
}

impl TermClient {
    /// Looks `code` up in the code system `terminology` names, for display in
    /// `language`.
    ///
    /// # Errors
    /// [`TermError::UnknownSystem`] when no FHIR system URI is configured for
    /// `terminology`, [`TermError::Refused`] when the server does not know the
    /// code (the specification answers that with 400 and an
    /// `OperationOutcome`), [`TermError::OutParameters`] when the answer
    /// carries a parameter R4 does not declare, and one variant per status
    /// family the specification publishes.
    pub async fn lookup(
        &self,
        terminology: &TerminologyName,
        code: &str,
        language: &LanguageTag,
    ) -> Result<Concept, TermError> {
        let system =
            self.systems()
                .resolve(terminology)
                .ok_or_else(|| TermError::UnknownSystem {
                    terminology: terminology.clone(),
                })?;
        let described = format!("{code} in {}", system.uri);
        let parameters = lookup_request(&system, code, language).to_parameters();
        let object = self.invoke(ENDPOINT, &parameters, &described).await?;
        concept_of(&object)
    }
}

/// The `in` parameters one lookup sends.
pub(crate) fn lookup_request(
    system: &System,
    code: &str,
    language: &LanguageTag,
) -> CodeSystemLookupRequest {
    CodeSystemLookupRequest {
        code: Some(Code::from(code)),
        system: Some(Uri::from(system.uri.as_str())),
        version: system.version.as_deref().map(FhirString::from),
        display_language: Some(Code::from(language.as_str())),
        property: vec![Code::from(DISPLAY)],
        ..CodeSystemLookupRequest::default()
    }
}

/// The concept the `Parameters` in `object` describe.
fn concept_of(object: &fhir_types::codec::Object) -> Result<Concept, TermError> {
    let mut path = Path::lenient("Parameters");
    let parameters =
        Parameters::from_json(object, &mut path).map_err(|source| TermError::Body {
            expected: "a Parameters resource",
            source,
        })?;
    let answer = CodeSystemLookupResponse::from_parameters(&parameters).map_err(|source| {
        TermError::OutParameters {
            operation: OPERATION,
            source,
        }
    })?;
    Ok(Concept {
        system_name: answer.name.value.unwrap_or_default(),
        system_version: answer.version.and_then(|version| version.value),
        display: answer.display.value.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{concept_of, lookup_request};
    use crate::client::object_of;
    use crate::system::System;
    use ferrochart_form::ids::LanguageTag;

    fn loinc() -> System {
        System {
            uri: "http://loinc.org".to_owned(),
            version: Some("2.65".to_owned()),
        }
    }

    #[test]
    fn a_lookup_names_the_system_its_version_and_the_display_language() {
        let request = lookup_request(&loinc(), "1963-8", &LanguageTag::new("en"));
        let names: Vec<String> = request
            .to_parameters()
            .parameter
            .iter()
            .filter_map(|parameter| parameter.name.value.clone())
            .collect();
        assert_eq!(
            names,
            ["code", "system", "version", "displayLanguage", "property"]
        );
    }

    #[test]
    fn the_declared_out_parameters_become_a_concept() {
        let body = r#"{"resourceType":"Parameters","parameter":[
            {"name":"name","valueString":"LOINC"},
            {"name":"version","valueString":"2.65"},
            {"name":"display","valueString":"Bicarbonate"}]}"#;
        let concept = concept_of(&object_of(body).expect("the body is JSON"))
            .expect("the answer fits the operation");
        assert_eq!(concept.display, "Bicarbonate");
        assert_eq!(concept.system_version.as_deref(), Some("2.65"));
    }

    #[test]
    fn an_out_parameter_r4_does_not_declare_is_reported_rather_than_dropped() {
        // R4 declares `name`, `version`, `display`, `designation` and
        // `property` and nothing else, so a server answering with more is
        // answering a question this client did not ask.
        let body = r#"{"resourceType":"Parameters","parameter":[
            {"name":"name","valueString":"LOINC"},
            {"name":"display","valueString":"Bicarbonate"},
            {"name":"ferroExtra","valueString":"undeclared"}]}"#;
        assert!(concept_of(&object_of(body).expect("the body is JSON")).is_err());
    }
}
