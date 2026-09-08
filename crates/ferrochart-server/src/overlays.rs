// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The layouts an operator installed, one per template.
//!
//! No specification governs this: our own design. A form definition is a
//! projection of the operational template and carries no layout
//! (`docs/architecture.md` section 4), so what a person authored travels as
//! its own document beside it. A template with no layout serves none, and a
//! renderer that fetches none draws the form in template order.
//!
//! The overlay on disk is `ferrochart-overlay`'s own JSON, which is what the
//! authoring surface will write. What a renderer reads is the published
//! `FormLayout` of `ferrochart-form`, so the browser links no engine crate
//! (`scripts/checks/crate-closure.sh`). This module is where the one becomes
//! the other.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_form::ids::TemplateId;
use ferrochart_form::layout::{FormLayout, LayoutItem};
use ferrochart_overlay::store::Overlay;

/// Why the layouts an operator installed could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OverlayStoreError {
    /// The directory could not be listed.
    #[error("the overlay directory `{path}` could not be read")]
    Directory {
        /// The directory.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },
    /// A file could not be read.
    #[error("the overlay `{path}` could not be read")]
    Unreadable {
        /// The file.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },
    /// A file is not an overlay.
    #[error("the overlay `{path}` could not be parsed")]
    Parse {
        /// The file.
        path: PathBuf,
        /// What the reader said.
        #[source]
        source: Box<ferrochart_overlay::error::OverlayError>,
    },
    /// Two files carry a layout for one template.
    #[error("`{first}` and `{second}` both lay out `{template_id}`")]
    Duplicate {
        /// The template both name.
        template_id: TemplateId,
        /// The file read first.
        first: PathBuf,
        /// The file that collided with it.
        second: PathBuf,
    },
}

/// The layouts the server holds, keyed by the template each was authored
/// against.
#[derive(Debug, Default)]
pub struct OverlayStore {
    held: BTreeMap<TemplateId, FormLayout>,
}

impl OverlayStore {
    /// A store holding nothing, which is what a deployment with no layouts
    /// has.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads every `.json` file in `directory`, in file-name order.
    ///
    /// # Errors
    /// [`OverlayStoreError::Directory`] when the directory cannot be listed,
    /// [`OverlayStoreError::Unreadable`] or [`OverlayStoreError::Parse`] when
    /// a file is not a readable overlay, and
    /// [`OverlayStoreError::Duplicate`] when two files lay out one template.
    ///
    /// A file that will not read fails the startup rather than being skipped:
    /// a layout an operator installed and the server quietly ignored is a form
    /// drawn in template order with no way to tell why.
    pub fn load(directory: &Path) -> Result<Self, OverlayStoreError> {
        let mut store = Self::new();
        let mut paths: Vec<PathBuf> = fs::read_dir(directory)
            .map_err(|source| OverlayStoreError::Directory {
                path: directory.to_path_buf(),
                source,
            })?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
            .collect();
        paths.sort();
        let mut first_of: BTreeMap<TemplateId, PathBuf> = BTreeMap::new();
        for path in paths {
            let text =
                fs::read_to_string(&path).map_err(|source| OverlayStoreError::Unreadable {
                    path: path.clone(),
                    source,
                })?;
            let overlay = Overlay::from_json(&text).map_err(|source| OverlayStoreError::Parse {
                path: path.clone(),
                source: Box::new(source),
            })?;
            let template_id = overlay.template().id.clone();
            if let Some(first) = first_of.get(&template_id) {
                return Err(OverlayStoreError::Duplicate {
                    template_id,
                    first: first.clone(),
                    second: path,
                });
            }
            first_of.insert(template_id.clone(), path);
            store.held.insert(template_id, published(&overlay));
        }
        Ok(store)
    }

    /// The layout authored against `template_id`, where the store holds one.
    #[must_use]
    pub fn get(&self, template_id: &str) -> Option<&FormLayout> {
        self.held.get(&TemplateId::new(template_id))
    }

    /// How many layouts the store holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether the store holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}

/// The published document an overlay becomes.
///
/// The authoring bookkeeping stays behind: an entry's recorded Reference Model
/// class and its positional anchor are what a replay compares against a
/// revised template, and a renderer has no use for either.
fn published(overlay: &Overlay) -> FormLayout {
    FormLayout {
        format_version: ferrochart_form::layout::LAYOUT_FORMAT_VERSION,
        template_id: overlay.template().id.clone(),
        sections: overlay.sections().to_vec(),
        items: overlay
            .entries()
            .iter()
            .map(|entry| LayoutItem {
                key: entry.key.clone(),
                layout: entry.layout.clone(),
            })
            .collect(),
    }
}
