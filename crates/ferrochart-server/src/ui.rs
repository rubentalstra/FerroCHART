// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The renderer bundle this binary carries, served under `/ui`.
//!
//! No specification governs a form server's user interface: our own design.
//! The bundle is a table of files compiled into the binary, so a request path
//! never reaches the filesystem and there is nothing to traverse out of.
//!
//! The table is empty unless the `ui` feature was on and the renderer's
//! `dist/` existed when this crate was built, and the server mounts no route
//! over an empty table.

use axum::Router;
use axum::http::StatusCode;
use axum::http::Uri;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;

/// One file of the renderer bundle, as the binary carries it.
#[derive(Debug, Clone, Copy)]
pub struct Asset {
    /// The path under `/ui/`, `/`-separated and without a leading slash.
    pub path: &'static str,
    /// The bytes the build wrote.
    pub bytes: &'static [u8],
}

/// The document a client-side route falls back to.
pub const INDEX: &str = "index.html";

/// The path the renderer is mounted at, with its trailing slash.
///
/// It is the `public_url` of `app/ferrochart-renderer/Trunk.toml`, so the
/// asset URLs the bundle's own `index.html` carries resolve here.
pub const MOUNT: &str = "/ui/";

/// The media type served for an extension [`MEDIA_TYPES`] does not name.
const OCTET_STREAM: &str = "application/octet-stream";

/// The media type of each extension the bundle carries.
///
/// `application/wasm` is the registered type
/// (<https://www.iana.org/assignments/media-types/application/wasm>) and a
/// browser refuses to stream-compile a module served as anything else;
/// `text/javascript` is the type RFC 9239 section 6 settles on.
const MEDIA_TYPES: [(&str, &str); 16] = [
    ("css", "text/css; charset=utf-8"),
    ("html", "text/html; charset=utf-8"),
    ("ico", "image/vnd.microsoft.icon"),
    ("jpeg", "image/jpeg"),
    ("jpg", "image/jpeg"),
    ("js", "text/javascript; charset=utf-8"),
    ("json", "application/json"),
    ("map", "application/json"),
    ("png", "image/png"),
    ("svg", "image/svg+xml"),
    ("ttf", "font/ttf"),
    ("txt", "text/plain; charset=utf-8"),
    ("wasm", "application/wasm"),
    ("webp", "image/webp"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
];

/// A year of caching, for a file whose name carries its own content hash.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// No reuse without revalidation, for a file whose name is stable.
const REVALIDATE: &str = "no-cache";

/// Builds the renderer's routes over `bundle`: the document at [`MOUNT`],
/// every asset under it, and `/ui` itself redirecting onto the mount.
///
/// The caller merges these after any middleware layer, so an asset request is
/// outside whatever the API routes are wrapped in.
pub fn router<S>(bundle: &'static [Asset]) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/ui",
            // The query travels with the redirect. `Redirect` takes a whole
            // location, so a bare path silently drops it and a link carrying
            // one lands somewhere else (RFC 9110 section 15.4).
            get(|uri: Uri| async move {
                match uri.query() {
                    Some(query) => Redirect::temporary(&format!("{MOUNT}?{query}")),
                    None => Redirect::temporary(MOUNT),
                }
            }),
        )
        .route("/ui/", get(move || async move { document(bundle) }))
        .route(
            "/ui/{*path}",
            get(
                move |axum::extract::Path(path): axum::extract::Path<String>| async move {
                    asset(bundle, &path)
                },
            ),
        )
}

/// The response for `path` under the mount.
///
/// A path the bundle holds is that file. A path naming a file type
/// [`MEDIA_TYPES`] knows is a request for an asset this build does not carry,
/// and answers `404`, so a stale script tag fails as a missing script rather
/// than as a page of HTML parsed as JavaScript. Anything else is a
/// client-side route and answers the document, which is how a deep link
/// works; that is why the test is the extension rather than the presence of a
/// dot, because a template identifier ends in `.v0`.
fn asset(bundle: &'static [Asset], path: &str) -> Response {
    match bundle.iter().find(|asset| asset.path == path) {
        Some(found) => served(found),
        None if known_extension(path) => StatusCode::NOT_FOUND.into_response(),
        None => document(bundle),
    }
}

/// The response for the single-page document, or `404` when the bundle holds
/// none.
fn document(bundle: &'static [Asset]) -> Response {
    bundle
        .iter()
        .find(|asset| asset.path == INDEX)
        .map_or_else(|| StatusCode::NOT_FOUND.into_response(), served)
}

/// `asset`, with its media type, its cache policy, and no content sniffing.
fn served(asset: &Asset) -> Response {
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, media_type(asset.path)),
            (CACHE_CONTROL, cache_control(asset.path)),
            (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        asset.bytes,
    )
        .into_response()
}

/// The extension of the file name in `path`, lowercased by comparison.
fn extension(path: &str) -> &str {
    let name = path.rsplit_once('/').map_or(path, |(_, last)| last);
    name.rsplit_once('.').map_or("", |(_, last)| last)
}

/// Whether `path` names a file type [`MEDIA_TYPES`] knows.
fn known_extension(path: &str) -> bool {
    let extension = extension(path);
    MEDIA_TYPES
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(extension))
}

/// The media type of the file at `path`.
fn media_type(path: &str) -> &'static str {
    let extension = extension(path);
    MEDIA_TYPES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(extension))
        .map_or(OCTET_STREAM, |(_, media)| *media)
}

/// The cache policy of the file at `path`.
///
/// Only a name carrying its own content hash is cached immutably, so the
/// header states what the build guarantees.
fn cache_control(path: &str) -> &'static str {
    if hashed(path) { IMMUTABLE } else { REVALIDATE }
}

/// Whether the file name of `path` carries a content hash.
///
/// Trunk's `filehash = true` writes the hash into the file name as its own
/// run of lowercase hexadecimal characters
/// (<https://trunkrs.dev/configuration/>), which is what makes an immutable
/// cache header true rather than a promise.
fn hashed(path: &str) -> bool {
    let name = path.rsplit_once('/').map_or(path, |(_, last)| last);
    name.split(['-', '_', '.'])
        .any(|token| token.len() >= 8 && token.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')))
}

// The build script writes the table from the renderer's `dist/`, one
// `include_bytes!` per file, so the bundle rides inside the binary.
#[cfg(feature = "ui")]
include!(concat!(env!("OUT_DIR"), "/ui_bundle.rs"));

/// The bundle compiled into this binary, empty because the `ui` feature is
/// off.
#[cfg(not(feature = "ui"))]
pub const BUNDLE: &[Asset] = &[];

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;

    use axum::Router;
    use axum::http::StatusCode;
    use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION, X_CONTENT_TYPE_OPTIONS};
    use ferrochart_cdr::client::CdrClient;
    use tokio::task::JoinHandle;

    use super::{
        Asset, IMMUTABLE, MOUNT, OCTET_STREAM, REVALIDATE, cache_control, hashed, media_type,
    };
    use crate::api::ServerState;
    use crate::store::TemplateStore;

    /// A bundle shaped like one Trunk writes, with no build behind it.
    const BUNDLE: &[Asset] = &[
        Asset {
            path: "index.html",
            bytes: b"<!doctype html><title>renderer</title>",
        },
        Asset {
            path: "ferrochart-renderer-0123456789abcdef.js",
            bytes: b"export default 0;",
        },
        Asset {
            path: "ferrochart-renderer-0123456789abcdef_bg.wasm",
            bytes: b"\0asm",
        },
    ];

    /// A bundle a build wrote without the document.
    const NO_INDEX: &[Asset] = &[Asset {
        path: "ferrochart-renderer-0123456789abcdef.js",
        bytes: b"export default 0;",
    }];

    /// A server bound to an ephemeral port, aborted when the value drops.
    struct Serving {
        base: String,
        task: JoinHandle<()>,
    }

    impl Drop for Serving {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    /// Serves `router` on an ephemeral loopback port.
    async fn serving(router: Router) -> Serving {
        let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .expect("an ephemeral port binds");
        let address = listener.local_addr().expect("the listener names its port");
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("the server runs");
        });
        Serving {
            base: format!("http://{address}"),
            task,
        }
    }

    /// A client that reports a redirect rather than following it.
    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("the client builds")
    }

    /// The whole application over `bundle`, with the renderer switched by
    /// `ui`.
    fn application(bundle: &'static [Asset], ui: bool) -> Router {
        let cdr = CdrClient::new(&url::Url::parse("http://127.0.0.1:9").expect("the base parses"))
            .expect("the client builds");
        let state = ServerState::new(Arc::new(TemplateStore::new()), cdr).with_ui(ui);
        crate::router_with_bundle(state, bundle)
    }

    #[test]
    fn the_mount_is_where_the_bundle_expects_it() {
        let trunk = include_str!("../../../app/ferrochart-renderer/Trunk.toml");
        assert!(
            trunk.contains(&format!("public_url = \"{MOUNT}\"")),
            "Trunk.toml has to publish under {MOUNT}"
        );
    }

    #[tokio::test]
    async fn ui_redirects_onto_the_mount_and_keeps_the_query() {
        let server = serving(super::router::<()>(BUNDLE)).await;
        let response = client()
            .get(format!("{}/ui?template=x", server.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/ui/?template=x")
        );
    }

    #[tokio::test]
    async fn the_mount_serves_the_document() {
        let server = serving(super::router::<()>(BUNDLE)).await;
        let response = client()
            .get(format!("{}/ui/", server.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some(REVALIDATE)
        );
        assert!(
            response
                .text()
                .await
                .expect("the body reads")
                .contains("renderer")
        );
    }

    #[tokio::test]
    async fn a_content_hashed_asset_is_immutable_and_a_stable_one_revalidates() {
        let server = serving(super::router::<()>(BUNDLE)).await;
        let hashed = client()
            .get(format!(
                "{}/ui/ferrochart-renderer-0123456789abcdef.js",
                server.base
            ))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(hashed.status(), StatusCode::OK);
        assert_eq!(
            hashed
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some(IMMUTABLE)
        );
        let stable = client()
            .get(format!("{}/ui/index.html", server.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(
            stable
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some(REVALIDATE)
        );
    }

    #[tokio::test]
    async fn the_module_is_served_as_wasm_and_never_sniffed() {
        let server = serving(super::router::<()>(BUNDLE)).await;
        let response = client()
            .get(format!(
                "{}/ui/ferrochart-renderer-0123456789abcdef_bg.wasm",
                server.base
            ))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/wasm")
        );
        assert_eq!(
            response
                .headers()
                .get(X_CONTENT_TYPE_OPTIONS)
                .and_then(|value| value.to_str().ok()),
            Some("nosniff")
        );
    }

    #[tokio::test]
    async fn a_missing_asset_is_not_found_and_a_client_side_route_is_the_document() {
        let server = serving(super::router::<()>(BUNDLE)).await;
        let missing = client()
            .get(format!("{}/ui/gone-0123456789abcdef.js", server.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        // A template identifier ends in `.v0`, which no media type names, so
        // the deep link is a route rather than a missing file.
        let route = client()
            .get(format!(
                "{}/ui/forms/openEHR-Suspected%20Covid-19%20assessment.v0",
                server.base
            ))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(route.status(), StatusCode::OK);
        assert_eq!(
            route
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
    }

    #[tokio::test]
    async fn a_bundle_without_the_document_answers_not_found() {
        let server = serving(super::router::<()>(NO_INDEX)).await;
        for path in ["/ui/", "/ui/templates"] {
            let response = client()
                .get(format!("{}{path}", server.base))
                .send()
                .await
                .expect("the request is answered");
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn the_switch_off_drops_the_routes() {
        let on = serving(application(BUNDLE, true)).await;
        let response = client()
            .get(format!("{}/ui/", on.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(response.status(), StatusCode::OK);

        let off = serving(application(BUNDLE, false)).await;
        for path in ["/ui", "/ui/", "/ui/index.html"] {
            let response = client()
                .get(format!("{}{path}", off.base))
                .send()
                .await
                .expect("the request is answered");
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
        let health = client()
            .get(format!("{}/health", off.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(health.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn an_empty_bundle_mounts_nothing() {
        let server = serving(application(&[], true)).await;
        let response = client()
            .get(format!("{}/ui/", server.base))
            .send()
            .await
            .expect("the request is answered");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn every_asset_carries_the_media_type_its_extension_names() {
        assert_eq!(media_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            media_type("ferrochart-renderer-0123456789abcdef_bg.wasm"),
            "application/wasm"
        );
        assert_eq!(
            media_type("ferrochart-renderer-0123456789abcdef.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            media_type("tailwind-0123456789abcdef.css"),
            "text/css; charset=utf-8"
        );
        assert_eq!(media_type("robots.txt"), "text/plain; charset=utf-8");
        assert_eq!(media_type("noextension"), OCTET_STREAM);
        assert_eq!(media_type("something.unknown"), OCTET_STREAM);
    }

    #[test]
    fn only_a_content_hashed_name_is_cached_immutably() {
        assert!(hashed("ferrochart-renderer-0123456789abcdef_bg.wasm"));
        assert!(hashed("tailwind-0123456789abcdef.css"));
        assert!(!hashed("index.html"));
        assert!(!hashed("robots.txt"));
        assert!(!hashed("snippets/logo.svg"));
        assert_eq!(cache_control("index.html"), REVALIDATE);
        assert_eq!(
            cache_control("ferrochart-renderer-0123456789abcdef.js"),
            IMMUTABLE
        );
    }
}
