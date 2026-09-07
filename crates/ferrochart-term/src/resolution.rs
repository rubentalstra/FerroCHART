// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a coded field's value set resolved to, and where the codes came from.
//!
//! Membership and display text are separate questions (`docs/architecture.md`
//! section 7), so a resolved code carries a display text that is absent rather
//! than empty when nothing states one, and the resolution says which source
//! answered.

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::value::Code;

/// One code a clinician may choose, with its display text resolved.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvedCode {
    /// What the composition stores.
    pub code: Code,
    /// The rubric the option is shown under, absent where nothing states one.
    pub display: Option<String>,
    /// The longer rubric, where the source carries one.
    pub description: Option<String>,
}

impl ResolvedCode {
    /// A code with no display text yet.
    #[must_use]
    pub fn new(code: Code) -> Self {
        Self {
            code,
            display: None,
            description: None,
        }
    }

    /// The same code shown under `display`.
    #[must_use]
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = Some(display.into());
        self
    }

    /// The same code with `description` as its longer rubric.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Which source answered for a value set.
///
/// `docs/architecture.md` section 7 measures 97.6% of coded fields as getting
/// their membership from the template, so a resolution that reports
/// [`Origin::Server`] for one of those is a defect rather than a slow path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Origin {
    /// The operational template, with no request at all.
    Template,
    /// The operational template for the membership, and a terminology server
    /// for the display text of the codes the template states no rubric for.
    ///
    /// `docs/architecture.md` section 7: an enumerated external code carries
    /// no rubric in the template, and 21 of the corpus's 228 are outside
    /// openEHR's own terminology, so which codes the field permits is still a
    /// template question and only the text is a server one.
    TemplateDisplayedByServer,
    /// openEHR's own support terminology, embedded by `openehr-term`, with no
    /// request at all.
    OpenehrTerminology,
    /// A FHIR terminology server.
    Server,
    /// The template constrains the field to no set, so there is nothing to
    /// resolve and no request was made.
    ///
    /// `docs/architecture.md` section 7 counts a `C_CODE_PHRASE` stating
    /// neither a terminology nor a code list as 0.5% of coded fields. The
    /// field takes any code, so a picker is the wrong surface for it.
    Unconstrained,
}

/// Every code one coded field permits, in the order the source states them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The codes, with their display text.
    pub codes: Vec<ResolvedCode>,
    /// Which source answered.
    pub origin: Origin,
    /// The language the display text was asked for in.
    pub language: LanguageTag,
    /// How many codes the set holds in total, where the source said.
    ///
    /// A server expansion is paged, so `codes` can be a prefix of the set.
    /// HL7 FHIR R4 4.0.1 `valueset.html` gives `ValueSet.expansion.total` as
    /// "the total number of concepts in the expansion", and a server need not
    /// state it.
    pub total: Option<u32>,
}

impl Resolution {
    /// A resolution of `codes` from `origin`, complete rather than paged.
    #[must_use]
    pub fn complete(codes: Vec<ResolvedCode>, origin: Origin, language: LanguageTag) -> Self {
        let total = u32::try_from(codes.len()).ok();
        Self {
            codes,
            origin,
            language,
            total,
        }
    }

    /// Whether the set holds more codes than this resolution carries.
    #[must_use]
    pub fn is_paged(&self) -> bool {
        self.total
            .is_some_and(|total| usize::try_from(total).is_ok_and(|total| total > self.codes.len()))
    }
}
