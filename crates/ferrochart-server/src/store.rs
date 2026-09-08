// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The operational templates one server holds, compiled once at startup.
//!
//! No specification governs this: our own design. openEHR ITS-REST
//! Release-1.1.0 defines how a CDR serves a template and says nothing about
//! where a form server keeps one, so the directory and its variable are
//! FerroCHART's own.
//!
//! Compiling at startup is what makes the request path cheap: a validator
//! flattens its operational template when it is built, so a server judges
//! every submission against a value it already holds.
//!
//! A template that will not compile fails the startup, naming the file and
//! the cause. A server that skipped it would serve fewer forms than its
//! operator installed and say nothing, which is the silent-wrongness class
//! `.claude/rules/reliability.md` refuses.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ferrochart_compile::derive::error::DeriveError;
use ferrochart_compile::error::ReadError;
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::ids::TemplateId;
use ferrochart_validate::error::ValidateError;
use ferrochart_validate::template::TemplateValidator;

/// The file extension of an operational template on disk.
///
/// The validator reads the canonical ADL 1.4 operational template XML, which
/// openEHR ITS-REST Release-1.1.0 `definition.html` serves from
/// `GET /v1/definition/template/adl1.4/{template_id}`, so that is the one
/// document this directory holds.
const EXTENSION: &str = "opt";

/// One compiled template: the form a renderer reads, and the gate a
/// submission is judged against.
#[derive(Debug)]
pub struct HeldTemplate {
    /// The form the template derives to.
    pub definition: FormDefinition,
    /// The gate for the same template, flattened once.
    pub validator: TemplateValidator,
    /// The file the template was read from.
    pub path: PathBuf,
}

/// Every template one server serves, keyed by the identifier each states for
/// itself.
#[derive(Debug, Default)]
pub struct TemplateStore {
    held: BTreeMap<TemplateId, HeldTemplate>,
}

/// Why a template directory could not be served.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The directory could not be listed.
    #[error("the template directory {} cannot be read", directory.display())]
    Directory {
        /// The directory as configured.
        directory: PathBuf,
        /// The operating system's reason.
        #[source]
        source: io::Error,
    },

    /// A template file could not be read.
    #[error("the operational template {} cannot be read", path.display())]
    Unreadable {
        /// The file.
        path: PathBuf,
        /// The operating system's reason.
        #[source]
        source: io::Error,
    },

    /// A template did not parse.
    #[error("the operational template {} does not parse", path.display())]
    Parse {
        /// The file.
        path: PathBuf,
        /// What the reader refused.
        #[source]
        source: Box<ReadError>,
    },

    /// A template parsed and derived no form.
    #[error("the operational template {} derives no form", path.display())]
    Derive {
        /// The file.
        path: PathBuf,
        /// What the derivation refused.
        #[source]
        source: Box<DeriveError>,
    },

    /// A template parsed and could not be flattened into a validator.
    #[error("the operational template {} cannot be flattened for validation", path.display())]
    Flatten {
        /// The file.
        path: PathBuf,
        /// What the validator refused.
        #[source]
        source: Box<ValidateError>,
    },

    /// Two files state the same template identifier.
    ///
    /// Serving either one would make which form a request gets depend on the
    /// order the filesystem listed them in.
    #[error(
        "the operational templates {} and {} both state the identifier `{template_id}`",
        first.display(),
        second.display()
    )]
    Duplicate {
        /// The identifier both files claim.
        template_id: TemplateId,
        /// The file that claimed it first.
        first: PathBuf,
        /// The file that claimed it again.
        second: PathBuf,
    },
}

impl TemplateStore {
    /// A store holding no template.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compiles every `.opt` file in `directory`, in file-name order.
    ///
    /// # Errors
    /// [`StoreError::Directory`] when the directory cannot be listed, one of
    /// [`StoreError::Unreadable`], [`StoreError::Parse`],
    /// [`StoreError::Derive`] or [`StoreError::Flatten`] when a template will
    /// not compile, and [`StoreError::Duplicate`] when two files state the
    /// same identifier.
    pub fn load(directory: &Path) -> Result<Self, StoreError> {
        let mut store = Self::new();
        for path in templates_in(directory)? {
            store.add(&path)?;
        }
        Ok(store)
    }

    /// Compiles one operational template into the store.
    fn add(&mut self, path: &Path) -> Result<(), StoreError> {
        let xml = fs::read_to_string(path).map_err(|source| StoreError::Unreadable {
            path: path.to_path_buf(),
            source,
        })?;
        let template =
            ferrochart_compile::adl14::from_xml(&xml).map_err(|source| StoreError::Parse {
                path: path.to_path_buf(),
                source: Box::new(source),
            })?;
        let definition =
            ferrochart_compile::derive::form(&template).map_err(|source| StoreError::Derive {
                path: path.to_path_buf(),
                source: Box::new(source),
            })?;
        let validator =
            TemplateValidator::from_opt14_xml(&xml).map_err(|source| StoreError::Flatten {
                path: path.to_path_buf(),
                source: Box::new(source),
            })?;

        let template_id = definition.template_id.clone();
        match self.held.entry(template_id.clone()) {
            Entry::Occupied(held) => Err(StoreError::Duplicate {
                template_id,
                first: held.get().path.clone(),
                second: path.to_path_buf(),
            }),
            Entry::Vacant(slot) => {
                slot.insert(HeldTemplate {
                    definition,
                    validator,
                    path: path.to_path_buf(),
                });
                Ok(())
            }
        }
    }

    /// The template `template_id` names, where the store holds it.
    #[must_use]
    pub fn get(&self, template_id: &str) -> Option<&HeldTemplate> {
        self.held.get(&TemplateId::new(template_id))
    }

    /// Every identifier the store holds, in identifier order.
    pub fn ids(&self) -> impl Iterator<Item = &TemplateId> {
        self.held.keys()
    }

    /// How many templates the store holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether the store holds no template at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}

/// Every `.opt` file directly in `directory`, sorted by path.
///
/// The sort makes the load order independent of the filesystem, so a
/// duplicate identifier names the same two files on every machine.
fn templates_in(directory: &Path) -> Result<Vec<PathBuf>, StoreError> {
    let listing = fs::read_dir(directory).map_err(|source| StoreError::Directory {
        directory: directory.to_path_buf(),
        source,
    })?;
    let mut found = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|source| StoreError::Directory {
            directory: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|suffix| suffix == EXTENSION) {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}
