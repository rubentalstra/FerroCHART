// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Turning the envelope's coded fields into Reference Model values.
//!
//! The envelope itself is `ferrochart_form::envelope`, because a browser has
//! to state a session's own facts. This module is the machinery that reads
//! one, which needs the Reference Model types the browser never links.
//!
//! All citations are openEHR RM Release-1.1.0.

use ferrochart_form::envelope::{Envelope, IANA_CHARACTER_SETS, ISO_639_1, ISO_3166_1, OPENEHR};
use openehr_base::v1_3::base_types::identification::terminology_id::TerminologyId;
use openehr_rm::v1_2::data_types::text::code_phrase::CodePhrase;

/// A `CODE_PHRASE` in the openEHR terminology.
#[must_use]
pub fn openehr_code(code: &str) -> CodePhrase {
    code_phrase(OPENEHR, code)
}

/// The `CODE_PHRASE` for an envelope's language.
#[must_use]
pub fn language_code(envelope: &Envelope) -> CodePhrase {
    code_phrase(ISO_639_1, &envelope.language)
}

/// The `CODE_PHRASE` for an envelope's territory.
#[must_use]
pub fn territory_code(envelope: &Envelope) -> CodePhrase {
    code_phrase(ISO_3166_1, &envelope.territory)
}

/// The `CODE_PHRASE` for an envelope's character encoding.
#[must_use]
pub fn encoding_code(envelope: &Envelope) -> CodePhrase {
    code_phrase(IANA_CHARACTER_SETS, &envelope.encoding)
}

/// A `CODE_PHRASE` naming `code` in `terminology`.
///
/// `preferred_term` is left absent: `data_types.html` section 5.2.3 makes it
/// optional, and a value FerroCHART did not get from a terminology service
/// would be a claim it cannot support.
#[must_use]
pub fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
    CodePhrase {
        terminology_id: TerminologyId {
            value: terminology.to_owned(),
        },
        code_string: code.to_owned(),
        preferred_term: None,
    }
}
