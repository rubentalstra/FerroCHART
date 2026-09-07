// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Shared helpers for the mock-server cases.
//!
//! Every code, URL and rubric below is invented for the test. No patient data.

use ferrochart_form::ids::{LanguageTag, TemplateId, TerminologyName};
use ferrochart_form::text::Localized;
use ferrochart_form::value::{Code, CodedOption};
use ferrochart_term::client::TermClient;
use url::Url;
use wiremock::MockServer;

/// The template the resolved fields belong to.
pub(crate) fn template() -> TemplateId {
    TemplateId::new("openEHR-EHR-COMPOSITION.ferro_synthetic.v1.0.0")
}

/// The language every case asks for unless it says otherwise.
pub(crate) fn english() -> LanguageTag {
    LanguageTag::new("en")
}

/// A client pointed at `server`.
pub(crate) fn client(server: &MockServer) -> TermClient {
    let base = Url::parse(&server.uri()).expect("the mock server has a URL");
    TermClient::new(&base).expect("the client builds")
}

/// One option of an enumerated set, with the rubric the template states.
pub(crate) fn option(terminology: &str, code: &str, label: Option<&str>) -> CodedOption {
    CodedOption {
        code: Code::new(TerminologyName::new(terminology), code),
        label: label.map_or_else(Localized::empty, |text| {
            Localized::in_language(english(), text)
        }),
        description: Localized::empty(),
    }
}
