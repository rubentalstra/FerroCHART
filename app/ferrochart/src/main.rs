// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The FerroCHART binary.

// A binary writes to stdio; a library does not.
#![expect(
    clippy::print_stderr,
    reason = "a configuration failure has to reach a terminal before tracing is up"
)]

use std::process::ExitCode;

use ferrochart_server::{Config, serve};

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            // Before the server is up there is no request to answer with, and
            // an operator reading `docker logs` needs the variable's name.
            eprintln!("ferrochart: {error}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("ferrochart: cannot start the async runtime: {error}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(serve(&config)) {
        Ok(()) => {
            tracing::info!("stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!(%error, "the server stopped with an error");
            ExitCode::FAILURE
        }
    }
}
