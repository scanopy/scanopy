//! The built UI, optionally compiled into the server binary.
//!
//! There are three ways the server can serve the web app. [`resolve`] picks
//! one at startup, in this order:
//!
//! 1. `SCANOPY_WEB_EXTERNAL_PATH` is set and holds an `index.html` — serve
//!    that directory from disk. This is what the Docker image does
//!    (`/app/static`), and it stays the override for anyone who wants to swap
//!    the UI without a rebuild. A path without a readable `index.html` (a
//!    failed or wiped UI build) falls through to (2) or (3) with a warning
//!    rather than stopping the server.
//! 2. The binary was built with `--features embed-ui` — serve the copy
//!    compiled in. This is what makes the standalone release artifact a
//!    single file with nothing beside it.
//! 3. Neither — API-only. Intended for a debug build behind `vite dev`; in a
//!    release build it almost always means a from-source build that left out
//!    `embed-ui`, so it warns.
//!
//! Responses here deliberately set only `Content-Type`. Cache-control and the
//! security headers come from the layers wrapping the whole router, so
//! embedded and on-disk serving stay identical in everything but the source
//! of the bytes.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(feature = "embed-ui")]
mod embedded {
    use axum::http::{Uri, header};
    use axum::response::{IntoResponse, Response};
    use rust_embed::Embed;

    /// The SvelteKit build output. Compiled in only under `embed-ui` — the
    /// folder exists only after a UI build, so an unconditional embed would
    /// break `cargo build` for anyone who hasn't run one.
    #[derive(Embed)]
    #[folder = "../ui/build"]
    struct WebAssets;

    const INDEX: &str = "index.html";

    pub fn index_html() -> Option<String> {
        let file = WebAssets::get(INDEX)?;
        String::from_utf8(file.data.into_owned()).ok()
    }

    /// Serve an embedded file, falling back to `index.html` for anything not
    /// found — the SPA owns client-side routing, which is the same behavior
    /// `ServeDir(...).fallback(ServeFile::new(index.html))` gives on the
    /// external-path branch.
    pub async fn handler(uri: Uri) -> Response {
        let path = uri.path().trim_start_matches('/');

        // A bare `/` or a directory path resolves to that directory's
        // index.html, matching `append_index_html_on_directories`.
        let candidate = if path.is_empty() || path.ends_with('/') {
            format!("{path}{INDEX}")
        } else {
            path.to_string()
        };

        serve(&candidate)
            .or_else(|| serve(INDEX))
            .unwrap_or_else(|| {
                // Only reachable if the build had no index.html, which the
                // startup check rules out before this handler is mounted.
                (axum::http::StatusCode::NOT_FOUND, "Not found").into_response()
            })
    }

    fn serve(path: &str) -> Option<Response> {
        let file = WebAssets::get(path)?;
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Some(
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                file.data.into_owned(),
            )
                .into_response(),
        )
    }
}

/// Whether this binary carries a compiled-in UI.
pub fn is_embedded() -> bool {
    cfg!(feature = "embed-ui")
}

/// The compiled-in `index.html`, or `None` when no UI is embedded.
pub fn index_html() -> Option<String> {
    #[cfg(feature = "embed-ui")]
    {
        embedded::index_html()
    }
    #[cfg(not(feature = "embed-ui"))]
    {
        None
    }
}

/// Router fallback that serves the compiled-in UI.
///
/// Only mount this when [`is_embedded`] is true; without the feature it
/// answers 404 for everything.
pub async fn fallback_handler(uri: axum::http::Uri) -> axum::response::Response {
    #[cfg(feature = "embed-ui")]
    {
        embedded::handler(uri).await
    }
    #[cfg(not(feature = "embed-ui"))]
    {
        let _ = uri;
        use axum::response::IntoResponse;
        (axum::http::StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

/// The source the server serves the web app from, as chosen by [`resolve`].
#[derive(Debug)]
pub enum WebUi {
    /// Served from `SCANOPY_WEB_EXTERNAL_PATH`. `index_html` is read during
    /// resolution, so the share routes never go back to disk for it.
    External {
        path: PathBuf,
        index_html: String,
    },
    Embedded,
    Disabled,
}

impl WebUi {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, WebUi::Disabled)
    }

    /// The document the share routes serve under their per-share CSP.
    pub fn share_index_html(&self) -> Option<String> {
        match self {
            WebUi::External { index_html, .. } => Some(index_html.clone()),
            WebUi::Embedded => index_html(),
            WebUi::Disabled => None,
        }
    }
}

/// A web UI configuration that is almost certainly not what the operator
/// meant. `Display` is the startup log line, and names the fix.
#[derive(Debug, PartialEq, Eq)]
pub enum WebUiWarning {
    /// `SCANOPY_WEB_EXTERNAL_PATH` is set but its `index.html` can't be read.
    ExternalMissingIndex {
        path: PathBuf,
        reason: io::ErrorKind,
        fell_back_to_embedded: bool,
    },
    /// A release build with no compiled-in UI and no external path.
    NoUiInReleaseBuild,
}

impl fmt::Display for WebUiWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebUiWarning::ExternalMissingIndex {
                path,
                reason,
                fell_back_to_embedded: true,
            } => write!(
                f,
                "SCANOPY_WEB_EXTERNAL_PATH is set, but {}/index.html could not be read ({reason}). \
                 Serving the UI compiled into the binary instead. Fix the path, or unset it.",
                path.display()
            ),
            WebUiWarning::ExternalMissingIndex {
                path,
                reason,
                fell_back_to_embedded: false,
            } => write!(
                f,
                "SCANOPY_WEB_EXTERNAL_PATH is set, but {}/index.html could not be read ({reason}), \
                 and this binary has no UI compiled in, so the web UI is disabled (API-only). \
                 Fix the path, or run the scanopy-server release binary, which serves the UI itself.",
                path.display()
            ),
            WebUiWarning::NoUiInReleaseBuild => f.write_str(
                "The web UI is disabled (API-only): this binary was built without the embed-ui \
                 feature and SCANOPY_WEB_EXTERNAL_PATH is unset. Run the scanopy-server release \
                 binary, rebuild with `--features embed-ui`, or point SCANOPY_WEB_EXTERNAL_PATH at \
                 a UI build. Ignore this if another server hosts the UI.",
            ),
        }
    }
}

/// Pick where the web app comes from. See the module docs for the order.
///
/// `embedded` and `dev_build` are parameters rather than read from `cfg!` so
/// every branch is testable from one build.
pub fn resolve(
    external_path: Option<&Path>,
    embedded: bool,
    dev_build: bool,
) -> (WebUi, Option<WebUiWarning>) {
    let fallback = if embedded {
        WebUi::Embedded
    } else {
        WebUi::Disabled
    };

    match external_path {
        Some(path) => match std::fs::read_to_string(path.join("index.html")) {
            Ok(index_html) => (
                WebUi::External {
                    path: path.to_path_buf(),
                    index_html,
                },
                None,
            ),
            Err(e) => (
                fallback,
                Some(WebUiWarning::ExternalMissingIndex {
                    path: path.to_path_buf(),
                    reason: e.kind(),
                    fell_back_to_embedded: embedded,
                }),
            ),
        },
        None if embedded || dev_build => (fallback, None),
        None => (fallback, Some(WebUiWarning::NoUiInReleaseBuild)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_path_with_index_serves_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<html>disk</html>").unwrap();

        let (ui, warning) = resolve(Some(dir.path()), true, false);

        assert!(warning.is_none());
        assert_eq!(ui.share_index_html().as_deref(), Some("<html>disk</html>"));
        assert!(matches!(ui, WebUi::External { .. }));
    }

    #[test]
    fn external_path_without_index_falls_back_to_embedded() {
        let dir = tempfile::tempdir().unwrap();

        let (ui, warning) = resolve(Some(dir.path()), true, false);

        assert!(matches!(ui, WebUi::Embedded));
        assert!(matches!(
            warning,
            Some(WebUiWarning::ExternalMissingIndex {
                reason: io::ErrorKind::NotFound,
                fell_back_to_embedded: true,
                ..
            })
        ));
    }

    #[test]
    fn external_path_without_index_or_embed_runs_api_only_instead_of_failing() {
        let missing = tempfile::tempdir().unwrap().path().join("build");

        let (ui, warning) = resolve(Some(&missing), false, false);

        assert!(!ui.is_enabled());
        assert!(matches!(
            warning,
            Some(WebUiWarning::ExternalMissingIndex {
                fell_back_to_embedded: false,
                ..
            })
        ));
    }

    #[test]
    fn api_only_warns_only_in_release_builds() {
        let (release_ui, release_warning) = resolve(None, false, false);
        let (dev_ui, dev_warning) = resolve(None, false, true);

        assert!(!release_ui.is_enabled());
        assert_eq!(release_warning, Some(WebUiWarning::NoUiInReleaseBuild));
        assert!(!dev_ui.is_enabled());
        assert!(dev_warning.is_none());
    }

    #[test]
    fn embedded_ui_without_external_path_is_silent() {
        let (ui, warning) = resolve(None, true, false);

        assert!(matches!(ui, WebUi::Embedded));
        assert!(warning.is_none());
    }
}
