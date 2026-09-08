// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The HTTP surface. It serves form definitions, validates entered values
//! against the operational template, builds COMPOSITIONs and commits them.
//!
//! The runtime surface is the configuration, the listener, the health probe
//! and an orderly shutdown. [`mod@api`] is the form surface over it, and no
//! specification governs a route of it: our own design.
//!
//! [`mod@commit`] is not a route: it is the gate of `docs/architecture.md`
//! section 9. It is the only path in this workspace from a clinician's entries
//! to a CDR write, and it validates the COMPOSITION against its operational
//! template before it makes any request.

pub mod api;
pub mod commit;
mod config;
mod health;
pub mod overlays;
pub mod store;
pub mod ui;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::error::CdrError;
use tokio::net::TcpListener;

pub use crate::api::ServerState;
pub use crate::config::{Config, ConfigError};
pub use crate::health::Health;
use crate::overlays::{OverlayStore, OverlayStoreError};
use crate::store::{StoreError, TemplateStore};

/// Why the server never started, or stopped.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServeError {
    /// A template the operator installed would not compile.
    #[error("the template directory {} cannot be served", directory.display())]
    Templates {
        /// The directory as configured.
        directory: PathBuf,
        /// Which template, and why.
        #[source]
        source: Box<StoreError>,
    },

    /// A layout the operator installed would not read.
    #[error("the overlay directory {} cannot be served", directory.display())]
    Overlays {
        /// The directory as configured.
        directory: PathBuf,
        /// Which overlay, and why.
        #[source]
        source: Box<OverlayStoreError>,
    },

    /// The configured CDR base is not a URL.
    #[error("FERROCHART_CDR_URL is not a URL: {url:?}")]
    CdrUrl {
        /// What was configured.
        url: String,
        /// Why it did not parse.
        #[source]
        source: url::ParseError,
    },

    /// The CDR client could not be built.
    #[error("no client can be built for the configured CDR")]
    Cdr {
        /// Why it could not be built.
        #[source]
        source: Box<CdrError>,
    },

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

/// Build the router over `state`, serving the renderer this binary carries.
///
/// Kept separate from [`serve`] so a test can drive the routes over a listener
/// of its own.
pub fn router(state: ServerState) -> Router {
    router_with_bundle(state, ui::BUNDLE)
}

/// Build the router over `state`, serving `bundle` as the renderer.
///
/// The renderer is mounted when the deployment asked for it and the bundle is
/// not empty; otherwise `/ui` is the `404` every unknown path answers. Taking
/// the bundle as an argument is what lets a test drive the renderer routes
/// without a `trunk build` behind them.
pub fn router_with_bundle(state: ServerState, bundle: &'static [ui::Asset]) -> Router {
    let renderer = state.serves_ui() && !bundle.is_empty();
    let app = Router::new()
        .route("/health", get(health::probe))
        .merge(api::routes(state));
    // NOTE: axum applies a layer only to the routes already added, so merging
    // the renderer last keeps its assets outside whatever wraps the API
    // (<https://docs.rs/axum/0.8/axum/struct.Router.html#method.layer>).
    if renderer {
        app.merge(ui::router(bundle))
    } else {
        app
    }
}

/// Compile the configured templates and connect to the configured CDR.
///
/// # Errors
/// Returns [`ServeError::Templates`] when a template the operator installed
/// will not compile, [`ServeError::Overlays`] when a layout they installed
/// will not read, [`ServeError::CdrUrl`] when the configured CDR base is not a
/// URL, and [`ServeError::Cdr`] when no client can be built for it.
pub fn state(config: &Config) -> Result<ServerState, ServeError> {
    let templates = match config.templates {
        None => TemplateStore::new(),
        Some(ref directory) => {
            TemplateStore::load(directory).map_err(|source| ServeError::Templates {
                directory: directory.clone(),
                source: Box::new(source),
            })?
        }
    };
    let overlays = match config.overlays {
        None => OverlayStore::new(),
        Some(ref directory) => {
            OverlayStore::load(directory).map_err(|source| ServeError::Overlays {
                directory: directory.clone(),
                source: Box::new(source),
            })?
        }
    };
    let base = url::Url::parse(&config.cdr_url).map_err(|source| ServeError::CdrUrl {
        url: config.cdr_url.clone(),
        source,
    })?;
    let cdr = CdrClient::new(&base).map_err(|source| ServeError::Cdr {
        source: Box::new(source),
    })?;
    Ok(ServerState::new(Arc::new(templates), cdr)
        .with_overlays(Arc::new(overlays))
        .with_ui(config.ui))
}

/// Bind the configured address and serve until the process is asked to stop.
///
/// Shutdown is orderly on SIGTERM and on Ctrl-C: the container image runs this
/// binary as PID 1 with no shell to forward a signal, so the process handles
/// its own.
///
/// # Errors
///
/// Returns whatever [`state`] refuses at startup, [`ServeError::Bind`] when
/// the address cannot be bound, and [`ServeError::Serve`] when the running
/// server fails.
pub async fn serve(config: &Config) -> Result<(), ServeError> {
    let state = state(config)?;
    let held = state.templates().len();
    let laid_out = state.overlays().len();

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
        templates = held,
        layouts = laid_out,
        "FerroCHART is listening"
    );

    axum::serve(listener, router(state))
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
