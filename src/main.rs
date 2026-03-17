mod codec;
mod mutations;
mod tree;

use axum::{Json, Router, routing::get};
use base64::Engine;
use serde::Serialize;
use tower_http::services::ServeDir;

use mutations::{is_degenerate, Mutation};
use tree::ImageProgram;

// ── API types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GenerateResponse {
    program_text: String,
    original: ImagePayload,
    mutations: Vec<MutatedPayload>,
}

#[derive(Serialize)]
struct ImagePayload {
    rgba_b64: String,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
struct MutatedPayload {
    label: String,
    program_text: String,
    image: ImagePayload,
    /// Set when the rendered result looks degenerate (single flat colour).
    warning: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn generate() -> Json<GenerateResponse> {
    // Load and decode the source JXL file.  Look first in assets/, then in
    // the user's Downloads folder so the project works straight away with
    // the sample file.
    let jxl_paths = [
        "assets/input.jxl",
        &format!("{}/Downloads/art.jxl", std::env::var("HOME").unwrap_or_default()),
        &format!("{}/Downloads/art(1).jxl", std::env::var("HOME").unwrap_or_default()),
    ];

    let original_payload = jxl_paths
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .and_then(|bytes| codec::decode_jxl_to_rgba(&bytes).ok())
        .map(|(w, h, rgba)| to_payload(&rgba, w, h))
        .unwrap_or_else(|| {
            // Fallback: render the example tree so the server still works
            // even without a JXL file present.
            let prog = ImageProgram::example();
            let rgba = prog.render_rgba();
            to_payload(&rgba, prog.width, prog.height)
        });

    let example = ImageProgram::example();
    let mutations: Vec<MutatedPayload> = Mutation::showcase()
        .iter()
        .map(|m| {
            let mutated = m.apply(&example);
            let rgba = mutated.render_rgba();
            let warning = if is_degenerate(&rgba) {
                Some("Degenerate render — this program may produce a flat image in jxl-art too, or our simplified renderer doesn't capture it correctly.".to_string())
            } else {
                None
            };
            MutatedPayload {
                label: m.label(),
                program_text: mutated.to_text(),
                image: to_payload(&rgba, mutated.width, mutated.height),
                warning,
            }
        })
        .collect();

    Json(GenerateResponse {
        program_text: example.to_text(),
        original: original_payload,
        mutations,
    })
}

fn to_payload(rgba: &[u8], width: u32, height: u32) -> ImagePayload {
    ImagePayload {
        rgba_b64: base64::engine::general_purpose::STANDARD.encode(rgba),
        width,
        height,
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/generate", get(generate))
        .fallback_service(ServeDir::new("static"));

    let addr = "0.0.0.0:3000";
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
