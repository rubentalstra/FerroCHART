// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Writes the renderer bundle table that `src/ui.rs` includes, one
//! `include_bytes!` per file of the renderer's `dist/`.
//!
//! It runs only with the `ui` feature on, so an ordinary `cargo build` never
//! needs a bundle. No specification governs any of this: our own design.

use std::path::{Path, PathBuf};

/// The environment variable naming the directory the renderer bundle was
/// built into, for a build that stages it somewhere other than the default.
const BUNDLE_ENV: &str = "FERROCHART_UI_BUNDLE";

/// The directory `trunk build` writes, relative to this manifest.
///
/// The server lives under `crates/` and the renderer under `app/`, so the
/// path climbs two levels rather than one.
const DEFAULT_BUNDLE: &str = "../../app/ferrochart-renderer/dist";

/// The size above which `clippy::large_include_file` fires, its own default
/// (<https://rust-lang.github.io/rust-clippy/master/index.html#large_include_file>).
const LARGE_INCLUDE_FILE: u64 = 1_000_000;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_UI").is_some() {
        ui_bundle();
    }
}

/// Writes the renderer bundle table into `OUT_DIR/ui_bundle.rs`.
///
/// A directory named by `FERROCHART_UI_BUNDLE` has to exist, so a release
/// lane that means to ship the renderer fails loud when the bundle is
/// missing. The default directory may be absent, because a fresh clone has no
/// `dist/` and `cargo build --all-features` still has to succeed there; the
/// table is then empty and the server mounts no `/ui` route.
fn ui_bundle() {
    println!("cargo::rerun-if-env-changed={BUNDLE_ENV}");
    let named = std::env::var_os(BUNDLE_ENV).map(PathBuf::from);
    let required = named.is_some();
    let dist = named.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_BUNDLE));
    println!("cargo::rerun-if-changed={}", dist.display());
    let mut files = Vec::new();
    let source = match collect(&dist, "", &mut files) {
        Ok(()) => table(&files),
        Err(error) if required => refusal(&format!(
            "{BUNDLE_ENV} names {}, which does not read: {error}. Build the bundle with `trunk build --release --locked` in app/ferrochart-renderer.",
            dist.display()
        )),
        Err(error) => {
            println!(
                "cargo::warning=the ui feature is on and {} does not read ({error}); this binary carries no renderer and serves no /ui route",
                dist.display()
            );
            table(&[])
        }
    };
    let Some(out_dir) = std::env::var_os("OUT_DIR") else {
        println!("cargo::warning=OUT_DIR is unset, so the renderer bundle table cannot be written");
        return;
    };
    let out = Path::new(&out_dir).join("ui_bundle.rs");
    if let Err(error) = std::fs::write(&out, source) {
        println!("cargo::warning=cannot write {}: {error}", out.display());
    }
}

/// Collects every file under `root`, depth first and in name order, as its
/// path under `/ui/` and the file to read it from.
///
/// The order is the directory listing sorted by name, so the table a build
/// writes depends only on the bundle's contents.
fn collect(root: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(root)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            println!(
                "cargo::warning=the bundle file {} has no UTF-8 name and is left out",
                entry.path().display()
            );
            continue;
        };
        let path = entry.path();
        let relative = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };
        if entry.file_type()?.is_dir() {
            collect(&path, &relative, out)?;
        } else {
            println!("cargo::rerun-if-changed={}", path.display());
            out.push((relative, path));
        }
    }
    Ok(())
}

/// Renders the `BUNDLE` table over `files`.
///
/// The `include_bytes!` of a multi-megabyte WebAssembly module trips
/// `clippy::large_include_file`, so the expectation is written only when a
/// file is actually large enough to fire it.
#[expect(
    clippy::format_collect,
    reason = "a build script writes this table once, and the line per file reads better than a folded writer"
)]
fn table(files: &[(String, PathBuf)]) -> String {
    let large = files
        .iter()
        .any(|(_, path)| std::fs::metadata(path).is_ok_and(|meta| meta.len() > LARGE_INCLUDE_FILE));
    let expectation = if large {
        "#[expect(clippy::large_include_file, reason = \"the renderer's WebAssembly module is one file and the binary carries it whole\")]\n"
    } else {
        ""
    };
    let entries: String = files
        .iter()
        .map(|(relative, path)| {
            let file = path.display().to_string();
            format!("    Asset {{ path: {relative:?}, bytes: include_bytes!({file:?}) }},\n")
        })
        .collect();
    format!(
        "/// The bundle compiled into this binary: the files the renderer's\n\
         /// `dist/` held when this crate was built.\n\
         {expectation}pub const BUNDLE: &[Asset] = &[\n{entries}];\n"
    )
}

/// Renders a bundle table that refuses to compile, with `message` as the
/// failure.
fn refusal(message: &str) -> String {
    format!(
        "compile_error!({message:?});\n/// The bundle this build could not read.\npub const BUNDLE: &[Asset] = &[];\n"
    )
}
