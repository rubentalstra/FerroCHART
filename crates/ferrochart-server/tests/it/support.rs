// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One committed template, compiled and ready to commit through the gate.
//!
//! Synthetic content invented for these tests. No patient data: every entry
//! comes from `crate::filler`, which picks a code the template enumerates or a
//! number inside a range the template states.

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use ferrochart_compose::envelope::{CATEGORY_EVENT, Composer, Envelope, Setting, Subject, UTF8};
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::values::FormValues;
use ferrochart_server::Config;
use ferrochart_validate::template::TemplateValidator;
use tokio::task::JoinHandle;

use crate::filler;

/// The committed template these tests commit.
///
/// It is rooted at COMPOSITION, which is what lets a CDR judge the document
/// FerroCHART sends against the template FerroCHART derived the form from.
/// The pack's other 113 templates are rooted at an ENTRY or a SECTION, and
/// what a CDR does with a COMPOSITION built around one of those is a separate
/// question from whether this gate works.
pub(crate) const TEMPLATE: &str = "openehr-suspected-covid-19-assessment-v0.opt";

/// The identifier that template states for itself.
pub(crate) const TEMPLATE_ID: &str = "openEHR-Suspected Covid-19 assessment.v0";

/// The template's canonical XML.
pub(crate) fn template_xml() -> String {
    fs::read_to_string(path()).expect("the committed template reads")
}

/// Where the template lives.
fn path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/templates/ckm")
        .join(TEMPLATE)
}

/// The form, the gate and a full set of entries.
pub(crate) fn case() -> (FormDefinition, TemplateValidator, FormValues) {
    let xml = template_xml();
    let template = ferrochart_compile::adl14::from_xml(&xml).expect("the template reads");
    let definition = ferrochart_compile::derive::form(&template).expect("the template derives");
    let validator = TemplateValidator::from_opt14_xml(&xml).expect("the template flattens");
    let values = filler::fill(&definition);
    (definition, validator, values)
}

/// A session's worth of the values a form never carries.
pub(crate) fn envelope() -> Envelope {
    Envelope {
        language: "en".to_owned(),
        territory: "NL".to_owned(),
        category: CATEGORY_EVENT.to_owned(),
        category_rubric: "event".to_owned(),
        composer: Composer::Identified {
            name: "Ferro test suite".to_owned(),
        },
        subject: Subject::SelfParty,
        encoding: UTF8.to_owned(),
        now: "2026-09-07T12:00:00Z".to_owned(),
        setting: Some(Setting {
            code: "238".to_owned(),
            rubric: "other care".to_owned(),
        }),
        // A template rooted below COMPOSITION is wrapped in this one.
        composition_archetype: Some("openEHR-EHR-COMPOSITION.encounter.v1".to_owned()),
    }
}

/// A directory holding the committed template, named for its caller.
///
/// Each test gets its own so a run in parallel cannot see another's files.
pub(crate) fn template_dir(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("the previous run's directory clears");
    }
    fs::create_dir_all(&directory).expect("the template directory is created");
    write_template(&directory, TEMPLATE);
    directory
}

/// Writes the committed template into `directory` under `name`.
pub(crate) fn write_template(directory: &Path, name: &str) {
    fs::write(directory.join(name), template_xml()).expect("the template is written");
}

/// A server bound to an ephemeral port, with its own routes and its own CDR.
///
/// The task is aborted when the value drops, so a test that fails does not
/// leave a listener behind.
#[derive(Debug)]
pub(crate) struct Running {
    /// The base URL, with no trailing slash.
    pub(crate) base: String,
    /// The task serving the routes.
    task: JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Serves the real router over `templates`, committing to `cdr`.
pub(crate) async fn serve(templates: PathBuf, cdr: &str) -> Running {
    let config = Config {
        listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        cdr_url: cdr.to_owned(),
        term_url: "http://127.0.0.1:9/r4".to_owned(),
        templates: Some(templates),
    };
    let state = ferrochart_server::state(&config).expect("the templates compile");
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .expect("an ephemeral port binds");
    let address = listener.local_addr().expect("the listener names its port");
    let task = tokio::spawn(async move {
        axum::serve(listener, ferrochart_server::router(state))
            .await
            .expect("the test server serves");
    });
    Running {
        base: format!("http://{address}"),
        task,
    }
}
