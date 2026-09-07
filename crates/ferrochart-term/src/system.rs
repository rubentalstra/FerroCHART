// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Which FHIR code system an openEHR `terminology_id` names.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 5.2.3 types
//! `CODE_PHRASE.terminology_id` as a `TERMINOLOGY_ID`, whose lexical form
//! openEHR BASE Release-1.2.0 `base_types.html` section 5 gives as
//! `name [ '(' version ')' ]`. FHIR names a code system by a canonical URI,
//! published in HL7 FHIR R4 4.0.1 `terminologies-systems.html` section 4.3.0.
//!
//! No specification maps one onto the other: our own design. The table below
//! carries the names the operational templates in `corpus/templates` actually
//! state, each paired with the URI that page publishes, and a deployer states
//! the rest.

use std::collections::BTreeMap;

use ferrochart_form::ids::TerminologyName;
use openehr_base::v1_2::base_types::identification::terminology_id::TerminologyId;

/// The terminology name an archetype's own codes carry.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3 reserves `local`
/// for codes the archetype itself defines, and those never reach a server.
pub const LOCAL: &str = "local";

/// The terminology name openEHR's own support terminology carries.
///
/// openEHR TERM Release-3.0.0 publishes the groups behind it, and
/// `openehr-term` embeds them, so these codes never reach a server either.
pub const OPENEHR: &str = "openehr";

/// The openEHR terminology names this table knows a FHIR system URI for.
///
/// Every URI is the one HL7 FHIR R4 4.0.1 `terminologies-systems.html`
/// section 4.3.0 publishes for that code system. ICD-10 is deliberately
/// absent: that page defers it to `icd.html`, where the URI depends on the
/// national modification, so a client cannot pick one.
const DEFAULTS: &[(&str, &str)] = &[
    ("SNOMED-CT", "http://snomed.info/sct"),
    ("SNOMED_CT", "http://snomed.info/sct"),
    ("LOINC", "http://loinc.org"),
    ("UCUM", "http://unitsofmeasure.org"),
    ("ISO_639-1", "urn:ietf:bcp:47"),
    ("ISO_3166-1", "urn:iso:std:iso:3166"),
    ("IANA_media-types", "urn:ietf:bcp:13"),
];

/// The openEHR terminology name to FHIR system URI table one client uses.
#[derive(Debug, Clone)]
pub struct SystemMap {
    by_name: BTreeMap<String, String>,
}

impl Default for SystemMap {
    fn default() -> Self {
        Self {
            by_name: DEFAULTS
                .iter()
                .map(|&(name, uri)| (name.to_owned(), uri.to_owned()))
                .collect(),
        }
    }
}

impl SystemMap {
    /// The table as published, with no deployer additions.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that `terminology` is the FHIR code system at `uri`, replacing
    /// any URI already recorded for that name.
    pub fn insert(&mut self, terminology: &TerminologyName, uri: impl Into<String>) {
        self.by_name
            .insert(terminology.as_str().to_owned(), uri.into());
    }

    /// The FHIR system URI and version `terminology` names.
    ///
    /// The version is the part inside a trailing `(...)`, which openEHR BASE
    /// Release-1.2.0 `base_types.html` section 5 defines as
    /// `TERMINOLOGY_ID.version_id`, and is `None` where the template states
    /// none. `LOINC(2.65)` therefore resolves the same system as `LOINC` and
    /// carries `2.65` beside it.
    #[must_use]
    pub fn resolve(&self, terminology: &TerminologyName) -> Option<System> {
        let id = TerminologyId {
            value: terminology.as_str().to_owned(),
        };
        let version = match id.version_id() {
            "" => None,
            version => Some(version.to_owned()),
        };
        // An identifier that is already an absolute URI is the system itself,
        // which is what a template states when it names a FHIR code system
        // directly rather than by an openEHR name.
        let uri = match self.by_name.get(id.name()) {
            Some(uri) => uri.clone(),
            None if is_absolute_uri(id.name()) => id.name().to_owned(),
            None => return None,
        };
        Some(System { uri, version })
    }

    /// Whether `terminology` resolves without a server at all.
    ///
    /// The archetype's own codes and openEHR's support terminology both carry
    /// their rubrics inside this process (`docs/architecture.md` section 7).
    #[must_use]
    pub fn is_local(terminology: &TerminologyName) -> bool {
        matches!(terminology.as_str(), LOCAL | OPENEHR)
    }
}

/// One FHIR code system, as a request names it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct System {
    /// The canonical system URI.
    pub uri: String,
    /// The system version, where the openEHR identifier states one.
    pub version: Option<String>,
}

/// Whether `value` is an absolute URI with a scheme this client would send.
///
/// A FHIR system is a `uri`, and the two forms a terminology server answers to
/// are an absolute URL and a URN.
fn is_absolute_uri(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://") || value.starts_with("urn:")
}

#[cfg(test)]
mod tests {
    use super::{SystemMap, is_absolute_uri};
    use ferrochart_form::ids::TerminologyName;

    #[test]
    fn a_versioned_openehr_identifier_resolves_the_unversioned_system() {
        // `LOINC(2.65)` is what the vendored templates state, and openEHR
        // BASE Release-1.2.0 `base_types.html` section 5 splits it into a name
        // and a version rather than making it another terminology.
        let map = SystemMap::new();
        let system = map
            .resolve(&TerminologyName::new("LOINC(2.65)"))
            .expect("LOINC is in the published table");
        assert_eq!(system.uri, "http://loinc.org");
        assert_eq!(system.version.as_deref(), Some("2.65"));
    }

    #[test]
    fn a_terminology_the_table_does_not_carry_resolves_to_nothing() {
        // ICD-10 has no single FHIR URI, so guessing one would send a request
        // about the wrong code system.
        assert!(
            SystemMap::new()
                .resolve(&TerminologyName::new("ICD10"))
                .is_none()
        );
    }

    #[test]
    fn a_deployer_can_state_a_system_the_table_does_not_carry() {
        let mut map = SystemMap::new();
        let name = TerminologyName::new("ICD10");
        map.insert(&name, "http://hl7.org/fhir/sid/icd-10");
        assert_eq!(
            map.resolve(&name).map(|system| system.uri),
            Some("http://hl7.org/fhir/sid/icd-10".to_owned())
        );
    }

    #[test]
    fn an_identifier_that_is_already_a_uri_is_the_system() {
        let system = SystemMap::new()
            .resolve(&TerminologyName::new("http://snomed.info/sct"))
            .expect("an absolute URI names itself");
        assert_eq!(system.uri, "http://snomed.info/sct");
        assert_eq!(system.version, None);
    }

    #[test]
    fn only_a_url_or_a_urn_reads_as_an_absolute_uri() {
        assert!(is_absolute_uri("urn:ietf:bcp:47"));
        assert!(!is_absolute_uri("terminology:SNOMEDCT?subset=x"));
    }
}
