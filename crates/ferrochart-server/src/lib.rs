// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The HTTP surface. It serves form definitions, validates entered values
//! against the operational template, builds COMPOSITIONs and commits them.
//!
//! The runtime surface is the configuration, the listener, the health probe
//! and an orderly shutdown. The routes that serve a form definition arrive
//! with the renderer.
//!
//! [`mod@commit`] is here already, and it is not a route: it is the gate of
//! `docs/architecture.md` section 9. It is the only path in this workspace
//! from a clinician's entries to a CDR write, and it validates the
//! COMPOSITION against its operational template before it makes any request.

pub mod commit;
mod config;
mod health;

use std::io;

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;

pub use crate::config::{Config, ConfigError};
pub use crate::health::Health;

/// Why the server stopped.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The listener could not be bound.
    #[error("cannot bind {addr}")]
    Bind {
        /// The address that was refused.
        addr: std::net::SocketAddr,
        /// The operating system's reason.
        #[source]
        source: io::Error,
    },

    /// The server was serving and stopped with an error.
    #[error("the server stopped")]
    Serve {
        /// The failure underneath.
        #[source]
        source: io::Error,
    },
}

/// Build the router.
///
/// Kept separate from [`serve`] so a test can drive the routes without binding
/// a port.
pub fn router() -> Router {
    Router::new().route("/health", get(health::probe))
}

/// Bind the configured address and serve until the process is asked to stop.
///
/// Shutdown is orderly on SIGTERM and on Ctrl-C: the container image runs this
/// binary as PID 1 with no shell to forward a signal, so the process handles
/// its own.
///
/// # Errors
///
/// Returns [`ServeError::Bind`] when the address cannot be bound, and
/// [`ServeError::Serve`] when the running server fails.
pub async fn serve(config: &Config) -> Result<(), ServeError> {
    let listener = TcpListener::bind(config.listen)
        .await
        .map_err(|source| ServeError::Bind {
            addr: config.listen,
            source,
        })?;

    tracing::info!(
        listen = %config.listen,
        cdr = %config.cdr_url,
        terminology = %config.term_url,
        "FerroCHART is listening"
    );

    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|source| ServeError::Serve { source })
}

/// Resolve when the process is asked to stop.
async fn shutdown() {
    let ctrl_c = async {
        // NOTE: a failed signal registration means the process cannot observe
        // the signal at all, so waiting forever is the honest outcome.
        match tokio::signal::ctrl_c().await {
            Ok(()) => tracing::info!("interrupt received, shutting down"),
            Err(error) => {
                tracing::error!(%error, "cannot listen for an interrupt");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
                tracing::info!("SIGTERM received, shutting down");
            }
            Err(error) => {
                tracing::error!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
