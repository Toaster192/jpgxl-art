mod mutations;
mod tree;

use axum::{Json, Router, body::Body, body::Bytes, extract::Query, http::header, response::Response, routing::get, routing::post};
use base64::Engine;
use futures::stream::{FuturesOrdered, FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tower_http::services::ServeDir;

use mutations::{is_degenerate, random_compounds, random_program, Mutation};
use tree::ImageProgram;

// ── API types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ImagePayload {
    rgba_b64: String,
    width: u32,
    height: u32,
    /// Byte size of the JXL file produced by jxl_from_tree for this program.
    jxl_size: u64,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamItem {
    Original {
        program_text: String,
        image: ImagePayload,
        mutation_count: usize,
    },
    Mutation {
        label: String,
        program_text: String,
        image: ImagePayload,
        compound: bool,
        warning: Option<String>,
    },
    BatchImage {
        index: usize,
        total: usize,
        program_text: String,
        image: ImagePayload,
    },
    Done,
}

#[derive(Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RenderMode {
    Single,
    #[default]
    Mutations,
    Compound20,
}

#[derive(Deserialize)]
struct RenderRequest {
    program_text: String,
    #[serde(default)]
    preview: bool,
    #[serde(default)]
    mode: RenderMode,
}

#[derive(Deserialize, Default)]
struct PreviewQuery {
    #[serde(default)]
    preview: bool,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn generate(Query(q): Query<PreviewQuery>) -> Response {
    stream_response(ImageProgram::example_jxlart(), q.preview, vec![])
}

async fn randomize(Query(q): Query<PreviewQuery>) -> Response {
    stream_response(random_program(), q.preview, Mutation::showcase())
}

async fn random_batch(Query(q): Query<PreviewQuery>) -> Response {
    const COUNT: usize = 20;
    let preview = q.preview;
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mut unordered: FuturesUnordered<_> = (0..COUNT)
            .map(|i| {
                tokio::task::spawn_blocking(move || {
                    let prog = random_program();
                    let program_text = prog.to_text();
                    let (rgba, w, h) = if preview {
                        prog.render_display_preview()
                    } else {
                        prog.render_display()
                    };
                    let image = to_payload(&rgba, w, h, &program_text);
                    (i, program_text, image)
                })
            })
            .collect();

        while let Some(result) = unordered.next().await {
            if let Ok((index, program_text, image)) = result {
                let item = StreamItem::BatchImage { index, total: COUNT, program_text, image };
                if let Ok(line) = serde_json::to_string(&item) {
                    let _ = tx.send(Bytes::from(line + "\n")).await;
                }
            }
        }

        let done = serde_json::to_string(&StreamItem::Done).unwrap_or_else(|_| "{\"type\":\"done\"}".into());
        let _ = tx.send(Bytes::from(done + "\n")).await;
    });

    let stream = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|bytes| (Ok::<_, Infallible>(bytes), rx))
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(stream))
        .unwrap()
}

async fn render(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let prog = ImageProgram::from_text(&req.program_text)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    let mutations = match req.mode {
        RenderMode::Single     => vec![],
        RenderMode::Mutations  => Mutation::showcase(),
        RenderMode::Compound20 => random_compounds(20),
    };
    Ok(stream_response(prog, req.preview, mutations))
}

async fn download_png(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let prog = ImageProgram::from_text(&req.program_text)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    let (rgba, w, h) = prog.render_display();
    let png = encode_png(&rgba, w, h)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"artxl.png\"")
        .body(Body::from(png))
        .unwrap())
}

async fn download_jxl(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    ImageProgram::from_text(&req.program_text)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    let jxl = encode_jxl_from_tree(&req.program_text)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/jxl")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"artxl.jxl\"")
        .body(Body::from(jxl))
        .unwrap())
}

// ── Streaming response ────────────────────────────────────────────────────────

fn stream_response(prog: ImageProgram, preview: bool, mutations: Vec<Mutation>) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mutation_count = mutations.len();

        let prog_orig = prog.clone();
        let orig_handle = tokio::task::spawn_blocking(move || {
            let program_text = prog_orig.to_text();
            let (rgba, w, h) = if preview {
                prog_orig.render_display_preview()
            } else {
                prog_orig.render_display()
            };
            let image = to_payload(&rgba, w, h, &program_text);
            (program_text, image)
        });

        let mut ordered: FuturesOrdered<_> = mutations
            .into_iter()
            .map(|m| {
                let label    = m.label();
                let compound = m.is_compound();
                let p = prog.clone();
                tokio::task::spawn_blocking(move || {
                    let mutated = m.apply(&p);
                    let text = mutated.to_text();
                    let (rgba, w, h) = if preview {
                        mutated.render_display_preview()
                    } else {
                        mutated.render_display()
                    };
                    let warning = if is_degenerate(&rgba) {
                        Some("Degenerate render — this program may produce a flat image in jxl-art too, or our simplified renderer doesn't capture it correctly.".to_string())
                    } else {
                        None
                    };
                    let image = to_payload(&rgba, w, h, &text);
                    (label, text, image, compound, warning)
                })
            })
            .collect();

        if let Ok((program_text, image)) = orig_handle.await {
            let item = StreamItem::Original { program_text, image, mutation_count };
            if let Ok(line) = serde_json::to_string(&item) {
                let _ = tx.send(Bytes::from(line + "\n")).await;
            }
        }

        while let Some(result) = ordered.next().await {
            if let Ok((label, program_text, image, compound, warning)) = result {
                let item = StreamItem::Mutation { label, program_text, image, compound, warning };
                if let Ok(line) = serde_json::to_string(&item) {
                    let _ = tx.send(Bytes::from(line + "\n")).await;
                }
            }
        }

        let done = serde_json::to_string(&StreamItem::Done).unwrap_or_else(|_| "{\"type\":\"done\"}".into());
        let _ = tx.send(Bytes::from(done + "\n")).await;
    });

    let stream = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|bytes| (Ok::<_, Infallible>(bytes), rx))
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(stream))
        .unwrap()
}

// ── Payload builder ───────────────────────────────────────────────────────────

fn to_payload(rgba: &[u8], width: u32, height: u32, program_text: &str) -> ImagePayload {
    let jxl_size = encode_jxl_from_tree(program_text).map(|v| v.len() as u64).unwrap_or(0);
    ImagePayload {
        rgba_b64: base64::engine::general_purpose::STANDARD.encode(rgba),
        width,
        height,
        jxl_size,
    }
}

// ── Encoding helpers ──────────────────────────────────────────────────────────

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Rgba};
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba.to_vec())
        .ok_or("failed to create image buffer")?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

fn encode_jxl_from_tree(program_text: &str) -> Result<Vec<u8>, String> {
    use std::process::Command;
    use rand::Rng;

    if !std::path::Path::new("./jxl_from_tree").exists() {
        return Err(
            "jxl_from_tree binary not found. Run 'make setup' to build it.".to_string()
        );
    }

    let id: u64 = rand::thread_rng().gen();
    let tmp = std::env::temp_dir();
    let input_path  = tmp.join(format!("artxl_{}.xl",  id));
    let output_path = tmp.join(format!("artxl_{}.jxl", id));

    std::fs::write(&input_path, program_text)
        .map_err(|e| format!("write temp input: {}", e))?;

    let status = Command::new("./jxl_from_tree")
        .arg(&input_path)
        .arg(&output_path)
        .status()
        .map_err(|e| format!("launch jxl_from_tree: {}", e))?;

    let _ = std::fs::remove_file(&input_path);

    if !status.success() {
        let _ = std::fs::remove_file(&output_path);
        return Err(format!("jxl_from_tree exited with {}", status));
    }

    let bytes = std::fs::read(&output_path)
        .map_err(|e| format!("read jxl output: {}", e))?;
    let _ = std::fs::remove_file(&output_path);

    Ok(bytes)
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/generate", get(generate))
        .route("/api/random", get(randomize))
        .route("/api/random/batch", get(random_batch))
        .route("/api/render", post(render))
        .route("/api/download/png", post(download_png))
        .route("/api/download/jxl", post(download_jxl))
        .fallback_service(ServeDir::new("static"));

    let addr = "0.0.0.0:3000";
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
