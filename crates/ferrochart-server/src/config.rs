// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The runtime configuration, read from the environment in one place.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

/// The prefix every FerroCHART environment variable carries.
const PREFIX: &str = "FERROCHART_";

/// The address the server binds when `FERROCHART_LISTEN` is unset.
///
/// Loopback on purpose. A container publishes a port by widening this to every
/// interface in its own image, so the default never exposes a host that did
/// not ask for it.
const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

/// What the server needs before it can answer a request.
///
/// The two upstream endpoints are required. A form builder with no CDR to
/// commit to and no terminology server to expand against cannot do its work,
/// so it refuses to start rather than failing on the first clinician's save.
#[derive(Debug, Clone)]
pub struct Config {
    /// The address to bind.
    pub listen: SocketAddr,
    /// The openEHR CDR's ITS-REST base URL.
    pub cdr_url: String,
    /// The FHIR terminology server's base URL.
    pub term_url: String,
    /// The directory of `.opt` operational templates the server compiles at
    /// startup, where the operator named one.
    ///
    /// Absent means the server holds no template and serves no form, which is
    /// the honest reading of an operator who installed none. A directory that
    /// IS named has to exist and every template in it has to compile, or the
    /// server refuses to start.
    pub templates: Option<PathBuf>,
}

/// Why the configuration could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required variable is absent.
    #[error("{PREFIX}{name} is not set: {hint}")]
    Missing {
        /// The variable's name without its prefix.
        name: &'static str,
        /// What to set it to.
        hint: &'static str,
    },

    /// A variable is present and unusable.
    #[error("{PREFIX}{name} is not valid: {value:?}")]
    Invalid {
        /// The variable's name without its prefix.
        name: &'static str,
        /// What was read.
        value: String,
        /// The parse failure underneath.
        #[source]
        source: std::net::AddrParseError,
    },

    /// A variable holds bytes that are not UTF-8.
    #[error("{PREFIX}{name} is not valid UTF-8")]
    NotUnicode {
        /// The variable's name without its prefix.
        name: &'static str,
    },
}

/// Read one prefixed variable, distinguishing absent from unreadable.
fn var(name: &'static str) -> Result<Option<String>, ConfigError> {
    match env::var(format!("{PREFIX}{name}")) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::NotUnicode { name }),
    }
}

/// Read one prefixed variable that has no default.
fn required(name: &'static str, hint: &'static str) -> Result<String, ConfigError> {
    match var(name)? {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing { name, hint }),
    }
}

impl Config {
    /// Read the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a required variable is absent or empty,
    /// when `FERROCHART_LISTEN` does not parse as a socket address, or when a
    /// variable holds bytes that are not UTF-8.
    pub fn from_env() -> Result<Self, ConfigError> {
        let listen = match var("LISTEN")? {
            Some(value) => value,
            None => DEFAULT_LISTEN.to_owned(),
        };
        let listen = listen.parse().map_err(|source| ConfigError::Invalid {
            name: "LISTEN",
            value: listen,
            source,
        })?;

        let templates = var("TEMPLATES")?
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from);

        Ok(Self {
            listen,
            cdr_url: required("CDR_URL", "the openEHR CDR's ITS-REST base URL")?,
            term_url: required("TERM_URL", "the FHIR terminology server's base URL")?,
            templates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_listen_address_is_loopback() {
        let parsed: SocketAddr = DEFAULT_LISTEN.parse().expect("the default parses");
        assert!(parsed.ip().is_loopback());
    }

    #[test]
    fn a_missing_endpoint_names_itself_and_what_to_set() {
        let err = ConfigError::Missing {
            name: "CDR_URL",
            hint: "the openEHR CDR's ITS-REST base URL",
        };
        let message = err.to_string();
        assert!(message.contains("FERROCHART_CDR_URL"), "{message}");
        assert!(message.contains("ITS-REST base URL"), "{message}");
    }

    #[test]
    fn an_unparseable_listen_address_carries_its_cause() {
        let source = "not-an-address".parse::<SocketAddr>().unwrap_err();
        let err = ConfigError::Invalid {
            name: "LISTEN",
            value: "not-an-address".to_owned(),
            source,
        };
        assert!(std::error::Error::source(&err).is_some());
    }
}
