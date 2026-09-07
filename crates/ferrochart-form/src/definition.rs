// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The published form definition.

use serde::{Deserialize, Serialize};

use crate::field::FormField;
use crate::group::{FormGroup, UndeterminedContent};
use crate::ids::{LanguageTag, TemplateId};

/// The version of the form definition format this crate defines.
///
/// A consumer reads it from [`FormDefinition::format_version`] and refuses a
/// document it does not know. The number changes only when a document that
/// parsed under the old version would parse differently under the new one.
pub const FORMAT_VERSION: u32 = 1;

/// A form, derived from one operational template.
///
/// This is what the compiler emits and what a renderer reads. It is a
/// projection of the operational template, so a form never admits what the
/// template refuses. It carries no layout: field order is template order,
/// grouping is the Reference Model tree, and every decision beyond that lives
/// in the layout overlay, which no specification governs either.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormDefinition {
    /// The version of the format this document is written in.
    pub format_version: u32,
    /// The identifier the operational template states for itself.
    pub template_id: TemplateId,
    /// The language the template is authored in, which is the one a renderer
    /// falls back to.
    pub default_language: LanguageTag,
    /// Every language any label or help text of this form is stated in, in tag
    /// order.
    pub languages: Vec<LanguageTag>,
    /// The form's content, rooted in the group the template root became.
    pub root: FormGroup,
}

impl FormDefinition {
    /// Whether this document is written in the format version this crate
    /// defines.
    #[must_use]
    pub const fn is_current_format(&self) -> bool {
        self.format_version == FORMAT_VERSION
    }

    /// Every field of the form, in template order.
    pub fn fields(&self) -> impl Iterator<Item = &FormField> {
        self.root.walk_fields()
    }

    /// Every group of the form, the root first, in template order.
    pub fn groups(&self) -> impl Iterator<Item = &FormGroup> {
        self.root.walk_groups()
    }

    /// Every piece of content the template left undetermined.
    pub fn undetermined(&self) -> impl Iterator<Item = &UndeterminedContent> {
        self.root.walk_undetermined()
    }
}
