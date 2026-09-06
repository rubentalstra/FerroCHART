// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The health probe.
//!
//! No specification governs this: our own design. It answers without
//! authentication, because the thing that probes it is an orchestrator with no
//! credentials, and it reports only that this process is up. It deliberately
//! does not report on the CDR or the terminology server: a probe that fails
//! when an upstream is down takes a healthy process out of rotation for
//! someone else's outage.

use axum::Json;
use axum::http::StatusCode;

/// What the probe answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// The running version.
    pub version: &'static str,
}

impl Health {
    /// The health of this process.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// `GET /health`.
pub(crate) async fn probe() -> (StatusCode, Json<serde_json::Value>) {
    let health = Health::current();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok", "version": health.version })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_probe_answers_ok_with_the_running_version() {
        let (status, Json(body)) = probe().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn the_version_is_the_crate_version() {
        assert_eq!(Health::current().version, env!("CARGO_PKG_VERSION"));
    }
}
