// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Which value set a request asks a terminology server about.
//!
//! A binding in an operational template names a target; a FHIR request names a
//! value set. This module is the one place the first becomes the second.

use fhir_types::r4::value_set::ValueSet;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::error::TermError;

/// The canonical system URI of SNOMED CT.
///
/// HL7 FHIR R4 4.0.1 `terminologies-systems.html` section 4.3.0.
const SNOMED_CT: &str = "http://snomed.info/sct";

/// The prefix that turns a SNOMED CT URI into the implicit value set of an
/// expression constraint.
///
/// HL7 FHIR R4 4.0.1 `snomedct.html` section 4.3.1.0.9 spells the whole form
/// as `?fhir_vs=ecl/[ecl]`, so everything up to and including the slash is
/// structure and only what follows it is the expression.
const IMPLICIT_ECL: &str = "?fhir_vs=ecl/";

/// What a URI-encoded expression escapes.
///
/// RFC 3986 section 2.3 leaves only the unreserved characters unescaped, and
/// `NON_ALPHANUMERIC` escapes those four as well, so they are removed from the
/// set (<https://datatracker.ietf.org/doc/html/rfc3986#section-2.3>).
const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// The value set one `$expand` or `$validate-code` call is about.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExpansionTarget {
    /// A value set the server already knows, by its canonical URL.
    ///
    /// Sent as the `url` in parameter, which HL7 FHIR R4 4.0.1
    /// `valueset-operation-expand.html` section 4.9.15.1 declares `0..1`.
    Canonical(String),
    /// A SNOMED CT expression constraint, sent as the implicit value set it
    /// denotes.
    ///
    /// FerroCHART never reads the expression (`docs/architecture.md`
    /// section 7.3). HL7 FHIR R4 4.0.1 `snomedct.html` section 4.3.1.0.9
    /// defines `?fhir_vs=ecl/[ecl]` as "all concept ids that match the
    /// supplied (URI-encoded) expression constraint", and section 4.3.1.0.8.3
    /// defines the equivalent `constraint` filter whose result "is the result
    /// of executing the given SNOMED CT Expression Constraint". Executing it
    /// is the server's work either way.
    SnomedExpression(String),
    /// A value set supplied with the request rather than named.
    ///
    /// Sent as the `valueSet` in parameter, which section 4.9.15.1 declares
    /// `0..1`. This is how a value set the server does not hold is expanded
    /// without storing it anywhere (`docs/architecture.md` section 7.2).
    Inline(Box<ValueSet>),
}

impl ExpansionTarget {
    /// The target `uri` names, where a FHIR request can be built from it.
    ///
    /// An absolute URL or URN is a canonical value set URL, which covers a
    /// `C_CODE_REFERENCE.referenceSetUri` and an ADL 2 constraint binding that
    /// states one. Everything else is refused rather than guessed at, because
    /// reading it would be inventing a grammar.
    ///
    /// # Errors
    /// [`TermError::UnresolvableBinding`] when `uri` is not a form a FHIR
    /// request can name a value set with.
    pub fn from_uri(uri: &str) -> Result<Self, TermError> {
        // TODO(#115): read the `terminology:<name>?subset=<...>` form the
        // vendored templates carry, once something defines what it means.
        let trimmed = uri.trim();
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("urn:")
        {
            return Ok(Self::Canonical(trimmed.to_owned()));
        }
        Err(TermError::UnresolvableBinding {
            uri: uri.to_owned(),
        })
    }

    /// The canonical URL this target sends as the `url` in parameter, where it
    /// has one.
    ///
    /// A value set supplied with the request has none, and goes as the
    /// `valueSet` in parameter instead.
    #[must_use]
    pub fn canonical_url(&self) -> Option<String> {
        match *self {
            Self::Canonical(ref url) => Some(url.clone()),
            Self::SnomedExpression(ref expression) => Some(implicit_expression_url(expression)),
            Self::Inline(_) => None,
        }
    }

    /// What this target is called in an error message.
    #[must_use]
    pub fn describe(&self) -> String {
        match *self {
            Self::Canonical(ref url) => url.clone(),
            Self::SnomedExpression(ref expression) => {
                format!("the SNOMED CT expression {expression}")
            }
            Self::Inline(ref value_set) => value_set
                .url
                .as_ref()
                .and_then(|url| url.value.clone())
                .unwrap_or_else(|| "a value set supplied with the request".to_owned()),
        }
    }
}

/// The implicit value set URL of a SNOMED CT expression constraint.
///
/// The `?fhir_vs=ecl/` prefix is structure and stays literal; only the
/// expression after it is URI-encoded, which is what HL7 FHIR R4 4.0.1
/// `snomedct.html` section 4.3.1.0.9 asks for.
fn implicit_expression_url(expression: &str) -> String {
    let encoded = utf8_percent_encode(expression, UNRESERVED);
    format!("{SNOMED_CT}{IMPLICIT_ECL}{encoded}")
}

#[cfg(test)]
mod tests {
    use super::{ExpansionTarget, implicit_expression_url};
    use crate::error::TermError;

    #[test]
    fn an_absolute_reference_set_uri_is_a_canonical_value_set_url() {
        let target =
            ExpansionTarget::from_uri("https://vsac.nlm.nih.gov/valueset/2.16.840.1/expansion")
                .expect("an absolute URL names a value set");
        assert_eq!(
            target.canonical_url(),
            Some("https://vsac.nlm.nih.gov/valueset/2.16.840.1/expansion".to_owned())
        );
    }

    #[test]
    fn the_terminology_scheme_the_templates_carry_is_refused_rather_than_guessed_at() {
        // This exact shape is in `corpus/templates`, and no openEHR
        // specification defines it, so the client says so instead of inventing
        // a grammar for it.
        let uri = "terminology:SNOMEDCT?subset=https://example.org/vs&language=en-GB";
        match ExpansionTarget::from_uri(uri).expect_err("no specification defines this form") {
            TermError::UnresolvableBinding { uri: reported } => assert_eq!(reported, uri),
            other => panic!("the failure is {other:?}"),
        }
    }

    #[test]
    fn the_ecl_prefix_stays_literal_and_only_the_expression_is_encoded() {
        // Section 4.3.1.0.9 spells the form as `?fhir_vs=ecl/[ecl]`, so a
        // client that escaped the slash would name a value set no server
        // recognises.
        let url = implicit_expression_url("<<73211009 |Diabetes mellitus|");
        assert_eq!(
            url,
            "http://snomed.info/sct?fhir_vs=ecl/%3C%3C73211009%20%7CDiabetes%20mellitus%7C"
        );
    }

    #[test]
    fn an_expression_with_no_reserved_character_survives_unchanged() {
        assert_eq!(
            implicit_expression_url("73211009"),
            "http://snomed.info/sct?fhir_vs=ecl/73211009"
        );
    }
}
