// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The Reference Model fields a form never shows, and where each one comes
//! from.
//!
//! The field derivation table (`docs/architecture.md` section 5) has a row per
//! data type and no row for a single envelope class, because a form has no
//! widget for `ENTRY.encoding`. A COMPOSITION will not validate without them,
//! so every one is named here with its source, and the ones FerroCHART has to
//! invent are marked as invented rather than passed off as derived.
//!
//! All citations are openEHR RM Release-1.1.0.

use openehr_base::v1_3::base_types::identification::terminology_id::TerminologyId;
use openehr_rm::v1_2::data_types::text::code_phrase::CodePhrase;

/// The openEHR terminology's own identifier.
///
/// `support.html` section 5.4.4: `Terminology_id_openehr: String = "openehr"`.
pub const OPENEHR: &str = "openehr";

/// The `composition category` group's `event` code.
///
/// `ehr.html` section 5.4.1: "433|event| - valid at the time of recording".
pub const CATEGORY_EVENT: &str = "433";

/// The Reference Model release this build writes into `ARCHETYPED.rm_version`.
///
/// `common.html` section 3.2.3 requires only a non-empty release string
/// ("Expressed in terms of the release version string, e.g. 1.0 , 1.2.4") and
/// never says which release the value describes. No specification governs the
/// choice: our own design. FerroCHART pins RM Release-1.1.0, so that is what
/// it writes.
pub const RM_VERSION: &str = "1.1.0";

/// The `character sets` code for UTF-8, which every `ENTRY` needs.
///
/// `ehr.html` section 8.3.1 types `ENTRY.encoding` as a `CODE_PHRASE` at
/// 1..1, and no template constrains it, so it is configuration.
pub const UTF8: &str = "UTF-8";

/// The identifier of the code set a `CODE_PHRASE` for a language belongs to.
///
/// `support.html` section 5.4.5: `Code_set_id_languages: String = "languages"`.
pub const ISO_639_1: &str = "ISO_639-1";

/// The identifier of the code set a territory belongs to.
///
/// `ehr.html` section 5.4.1: territory is "Coded from openEHR countries code
/// set, which is an expression of the ISO 3166 standard".
pub const ISO_3166_1: &str = "ISO_3166-1";

/// The identifier of the character-set code set.
const IANA_CHARACTER_SETS: &str = "IANA_character-sets";

/// Everything the Reference Model requires and the form does not carry.
///
/// Each field says where it comes from, because the distinction matters: a
/// derived value is a fact about the template, and an invented one is a
/// decision FerroCHART made that a reader is entitled to question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// `COMPOSITION.language`, 1..1. Configuration or the session locale, and
    /// no CKM template constrains it.
    pub language: String,
    /// `COMPOSITION.territory`, 1..1. Configuration.
    pub territory: String,
    /// `COMPOSITION.category`, 1..1, as a code in the openEHR
    /// `composition category` group.
    ///
    /// The group has four members in TERM Release-3.0.0, not the three the
    /// Reference Model prose names: `431` persistent, `451` episodic, `433`
    /// event and `815` report. `Category_validity` tests membership of the
    /// group, and `ehr.html` section 5.4.1 adds "or any other code defined in
    /// the openEHR terminology group", so a fixed list of three would be
    /// wrong.
    pub category: String,
    /// The rubric of `category` in `language`.
    pub category_rubric: String,
    /// `COMPOSITION.composer`, 1..1. The session's clinician.
    ///
    /// `ehr.html` section 5.2.2: "This attribute is mandatory, since all
    /// content must be been created by some person or agent." Nothing in the
    /// Reference Model or ITS-REST authorises a server to invent it, so
    /// FerroCHART always sends one.
    pub composer: Composer,
    /// `ENTRY.subject`, 1..1 on every entry.
    ///
    /// `PARTY_SELF` where the record's own subject is who the data is about,
    /// which is the ordinary case.
    pub subject: Subject,
    /// `ENTRY.encoding`, 1..1 on every entry.
    pub encoding: String,
    /// `EVENT_CONTEXT.start_time` and `HISTORY.origin` and `EVENT.time`, as an
    /// ISO 8601 date and time. The session supplies it; no template
    /// constrains any of the three.
    pub now: String,
    /// `EVENT_CONTEXT.setting`, 1..1 whenever a context is written, as a code
    /// in the openEHR `setting` group.
    pub setting: Option<Setting>,
}

/// Who composed the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Composer {
    /// `PARTY_SELF`: the subject of the record composed it, which is what
    /// patient-entered data uses (`ehr.html` section 5.2.2).
    SelfParty,
    /// `PARTY_IDENTIFIED`. `common.html` section 4.3.3 requires at least one
    /// of a name, an identifier and an external reference, so the name is not
    /// optional here even though the class makes it so.
    Identified {
        /// The composer's name, as a demographic identifier would spell it.
        name: String,
    },
}

/// Who the entry is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// `PARTY_SELF`: the subject of the record.
    SelfParty,
}

/// The setting a context was recorded in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    /// The code, from the openEHR `setting` group.
    pub code: String,
    /// Its rubric in the composition's language.
    pub rubric: String,
}

impl Envelope {
    /// A `CODE_PHRASE` in the openEHR terminology.
    #[must_use]
    pub fn openehr_code(code: &str) -> CodePhrase {
        code_phrase(OPENEHR, code)
    }

    /// The `CODE_PHRASE` for this envelope's language.
    #[must_use]
    pub fn language_code(&self) -> CodePhrase {
        code_phrase(ISO_639_1, &self.language)
    }

    /// The `CODE_PHRASE` for this envelope's territory.
    #[must_use]
    pub fn territory_code(&self) -> CodePhrase {
        code_phrase(ISO_3166_1, &self.territory)
    }

    /// The `CODE_PHRASE` for this envelope's character encoding.
    #[must_use]
    pub fn encoding_code(&self) -> CodePhrase {
        code_phrase(IANA_CHARACTER_SETS, &self.encoding)
    }
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

/// The `instruction states` code for an action that has happened.
///
/// openEHR RM Release-1.1.0 `ehr.html` section 8.3.6 makes
/// `ISM_TRANSITION.current_state` 1..1 and codes it from that group. No
/// specification says what a form that states no transition means, so
/// recording the action as having happened is our own design.
pub const ACTIVE: &str = "532";
