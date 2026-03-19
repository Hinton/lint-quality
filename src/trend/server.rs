use axum::extract::State;
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use rust_embed::Embed;
use std::sync::Arc;

use crate::report::Report;

#[derive(Embed)]
#[folder = "web/dist/"]
struct Assets;

#[derive(Clone)]
struct AppState {
    /// Reports JSON, injected into index.html as a global variable
    reports_json: Arc<String>,
}

pub async fn serve(reports: Vec<Report>, port: u16, no_open: bool) -> anyhow::Result<()> {
    let reports_json = serde_json::to_string(&reports)?;
    let state = AppState {
        reports_json: Arc::new(reports_json),
    };

    let app = Router::new()
        .fallback(static_handler)
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let url = format!("http://{}", addr);
    eprintln!("serving dashboard at {}", url);

    if !no_open {
        let _ = open::that(&url);
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Serve embedded static assets, injecting report data into index.html.
async fn static_handler(uri: Uri, State(state): State<AppState>) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if path == "index.html" {
        return serve_index(&state.reports_json);
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for unknown routes
            serve_index(&state.reports_json)
        }
    }
}

/// Inject the reports JSON into index.html as `window.__REPORTS__`.
fn serve_index(reports_json: &str) -> Response {
    match Assets::get("index.html") {
        Some(content) => {
            let html = String::from_utf8_lossy(&content.data);
            let injected = html.replace(
                "</head>",
                &format!(
                    "<script>window.__REPORTS__={}</script></head>",
                    reports_json
                ),
            );
            Html(injected).into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl+c");
    eprintln!("\nshutting down...");
}
