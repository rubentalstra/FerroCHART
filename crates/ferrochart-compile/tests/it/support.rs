// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Shared helpers for the integration tests.

use std::fs;
use std::path::{Path, PathBuf};

/// The committed CKM operational template pack.
pub(crate) fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/templates/ckm")
}

/// Every committed `.opt` in the pack, in a stable order.
pub(crate) fn templates() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(corpus_dir()) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "opt"))
        .collect();
    found.sort();
    found
}
