// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `ValueSet/$validate-code`, which confirms the code a clinician chose.
//!
//! All citations are HL7 FHIR R4 4.0.1
//! `valueset-operation-validate-code.html` section 4.9.15.2 unless another
//! page is named.
//!
//! A refusal is an answer rather than a failure: the operation declares
//! `result` as a `1..1` boolean and `message` as "error details, if result =
//! false", so a code the value set does not admit comes back as a 200 with
//! `result` false. A transport failure and a code that is not a member are
//! therefore never the same outcome.

use fhir_types::codec::{Json, Object, Path};
use fhir_types::r4::operations::value_set_validate_code::{
    ValueSetValidateCodeRequest, ValueSetValidateCodeResponse,
};
use fhir_types::r4::parameters::Parameters;
use fhir_types::r4::primitives::{Code, String as FhirString, Uri};

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::value;

use crate::client::TermClient;
use crate::error::TermError;
use crate::system::SystemMap;
use crate::target::ExpansionTarget;

/// The path the operation is invoked at, as a type-level operation.
const ENDPOINT: &[&str] = &["ValueSet", "$validate-code"];

/// The operation, for an error that names it.
const OPERATION: &str = "ValueSet/$validate-code";

/// What a server said about one chosen code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validation {
    /// Whether the concept details supplied are valid, the `1..1` out
    /// parameter.
    pub valid: bool,
    /// The error details the server gave, where `valid` is false.
    pub message: Option<String>,
    /// A display for the concept the server would show, where it offered one.
    pub display: Option<String>,
}

impl TermClient {
    /// Asks whether `code` is a member of `target`, for display in
    /// `language`.
    ///
    /// # Errors
    /// [`TermError::UnknownSystem`] when no FHIR system URI is configured for
    /// the code's terminology, [`TermError::UnknownValueSet`] when the server
    /// does not hold the value set, and one variant per status family the
    /// specification publishes. A code the value set refuses is a
    /// [`Validation`] with `valid` false, never an error.
    pub async fn validate_code(
        &self,
        target: &ExpansionTarget,
        code: &value::Code,
        language: &LanguageTag,
    ) -> Result<Validation, TermError> {
        let system = if SystemMap::is_local(&code.terminology) {
            None
        } else {
            Some(self.systems().resolve(&code.terminology).ok_or_else(|| {
                TermError::UnknownSystem {
                    terminology: code.terminology.clone(),
                }
            })?)
        };
        let described = target.describe();
        let request = validate_request(target, code, system.as_ref(), language);
        let object = self
            .invoke(ENDPOINT, &request.to_parameters(), &described)
            .await?;
        validation_of(&object)
    }
}

/// The `in` parameters one validation sends.
///
/// A named target goes as `url` and a supplied one as `valueSet`, which the
/// specification declares `0..1` each.
pub(crate) fn validate_request(
    target: &ExpansionTarget,
    code: &value::Code,
    system: Option<&crate::system::System>,
    language: &LanguageTag,
) -> ValueSetValidateCodeRequest {
    let inline = match *target {
        ExpansionTarget::Inline(ref value_set) => Some((**value_set).clone()),
        _ => None,
    };
    ValueSetValidateCodeRequest {
        url: target.canonical_url().map(Uri::from),
        value_set: inline,
        code: Some(Code::from(code.code.as_str())),
        system: system.map(|system| Uri::from(system.uri.as_str())),
        system_version: system
            .and_then(|system| system.version.as_deref())
            .map(FhirString::from),
        display_language: Some(Code::from(language.as_str())),
        ..ValueSetValidateCodeRequest::default()
    }
}

/// The verdict the `Parameters` in `object` carry.
fn validation_of(object: &Object) -> Result<Validation, TermError> {
    let mut path = Path::lenient("Parameters");
    let parameters =
        Parameters::from_json(object, &mut path).map_err(|source| TermError::Body {
            expected: "a Parameters resource",
            source,
        })?;
    let answer = ValueSetValidateCodeResponse::from_parameters(&parameters).map_err(|source| {
        TermError::OutParameters {
            operation: OPERATION,
            source,
        }
    })?;
    Ok(Validation {
        valid: answer.result.value.unwrap_or_default(),
        message: answer.message.and_then(|message| message.value),
        display: answer.display.and_then(|display| display.value),
    })
}

#[cfg(test)]
mod tests {
    use super::{validate_request, validation_of};
    use crate::client::object_of;
    use crate::system::System;
    use crate::target::ExpansionTarget;
    use ferrochart_form::ids::{LanguageTag, TerminologyName};
    use ferrochart_form::value::Code;

    #[test]
    fn a_validation_names_the_value_set_the_code_and_the_system() {
        let target = ExpansionTarget::Canonical("https://example.org/ValueSet/ferro".to_owned());
        let code = Code::new(TerminologyName::new("LOINC"), "1963-8");
        let system = System {
            uri: "http://loinc.org".to_owned(),
            version: None,
        };
        let request = validate_request(&target, &code, Some(&system), &LanguageTag::new("en"));
        let names: Vec<String> = request
            .to_parameters()
            .parameter
            .iter()
            .filter_map(|parameter| parameter.name.value.clone())
            .collect();
        assert_eq!(names, ["url", "code", "system", "displayLanguage"]);
    }

    #[test]
    fn a_code_the_value_set_refuses_is_an_answer_and_not_a_failure() {
        // `result` is `1..1` and `message` is "error details, if result =
        // false", so the refusal arrives as a 200 the client reads rather than
        // as an error it maps.
        let body = r#"{"resourceType":"Parameters","parameter":[
            {"name":"result","valueBoolean":false},
            {"name":"message","valueString":"Unknown code 'ferro-9' in the CodeSystem"}]}"#;
        let verdict = validation_of(&object_of(body).expect("the body is JSON"))
            .expect("the answer fits the operation");
        assert!(!verdict.valid);
        assert!(verdict.message.is_some_and(|text| text.contains("ferro-9")));
    }

    #[test]
    fn an_accepted_code_carries_the_display_the_server_would_show() {
        let body = r#"{"resourceType":"Parameters","parameter":[
            {"name":"result","valueBoolean":true},
            {"name":"display","valueString":"Female"}]}"#;
        let verdict = validation_of(&object_of(body).expect("the body is JSON"))
            .expect("the answer fits the operation");
        assert!(verdict.valid);
        assert_eq!(verdict.display.as_deref(), Some("Female"));
    }
}
