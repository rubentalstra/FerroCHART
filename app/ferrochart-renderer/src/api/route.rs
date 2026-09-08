// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The addresses of the form surface, built rather than written out.
//!
//! No specification governs these paths: they are `ferrochart-server`'s own
//! design, and this module is the one place the renderer spells them. Every
//! caller-supplied part goes through [`crate::url`], so an identifier cannot
//! change the shape of the address it sits in.
//!
//! The functions are pure, so a native test covers them where the transport
//! itself cannot be covered without a browser.

use crate::url;

/// The prefix every route of the form surface hangs off.
const BASE: &str = "/api";

/// The templates this server holds.
pub(crate) fn templates() -> String {
    format!("{BASE}/templates")
}

/// The form definition one template compiles to.
pub(crate) fn definition(template_id: &str) -> String {
    format!("{BASE}/templates/{}/definition", url::segment(template_id))
}

/// The judgement of entered values against one template.
#[allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]
pub(crate) fn validation(template_id: &str) -> String {
    format!("{BASE}/templates/{}/validation", url::segment(template_id))
}

/// The compositions of one EHR under one template.
#[allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]
pub(crate) fn compositions(ehr_id: &str, template_id: &str) -> String {
    format!(
        "{BASE}/ehrs/{}/templates/{}/compositions",
        url::segment(ehr_id),
        url::segment(template_id)
    )
}

/// The entered values one stored composition reads back into.
#[allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]
pub(crate) fn values(ehr_id: &str, uid: &str, template_id: &str) -> String {
    format!(
        "{BASE}/ehrs/{}/compositions/{}/values?template={}",
        url::segment(ehr_id),
        url::segment(uid),
        url::query_value(template_id)
    )
}

#[cfg(test)]
mod tests {
    use super::{compositions, definition, templates, validation, values};

    #[test]
    fn every_route_hangs_off_the_one_prefix() {
        for address in [
            templates(),
            definition("t"),
            validation("t"),
            compositions("e", "t"),
            values("e", "u", "t"),
        ] {
            assert!(address.starts_with("/api/"), "{address}");
        }
    }

    #[test]
    fn a_template_identifier_carrying_a_space_stays_inside_its_segment() {
        assert_eq!(
            definition("IDCR - Adverse Reaction List.v1"),
            "/api/templates/IDCR%20-%20Adverse%20Reaction%20List.v1/definition"
        );
    }

    #[test]
    fn a_definition_and_a_judgement_address_the_same_template() {
        assert_eq!(
            definition("a/b").trim_end_matches("definition"),
            validation("a/b").trim_end_matches("validation")
        );
    }

    #[test]
    fn a_version_uid_reaches_the_read_back_route_unaltered() {
        let address = values("e", "f0a1::ferro.example::1", "t");
        assert!(
            address.contains("/compositions/f0a1::ferro.example::1/values"),
            "{address}"
        );
    }

    #[test]
    fn the_template_a_read_back_names_travels_as_a_query_value() {
        assert_eq!(
            values("e", "u", "vital signs&x"),
            "/api/ehrs/e/compositions/u/values?template=vital%20signs%26x"
        );
    }

    #[test]
    fn the_two_identifiers_of_a_commit_keep_their_order() {
        assert_eq!(
            compositions("ehr-1", "tmpl-1"),
            "/api/ehrs/ehr-1/templates/tmpl-1/compositions"
        );
    }
}
