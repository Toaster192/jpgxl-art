mod codec;
mod gallery;
mod mutations;
mod render;
mod tree;

use axum::{Json, Router, body::Body, body::Bytes, extract::Query, http::header, response::Response, routing::get, routing::post};
use base64::Engine;
use futures::stream::{FuturesOrdered, FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::OnceLock;
use tower_http::services::ServeDir;

use mutations::{is_degenerate, random_compounds, random_program_non_degenerate, Mutation};
use render::{encode_jxl_from_tree, render_roundtrip};
use tree::ImageProgram;

// ── API types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ImagePayload {
    webp_b64: String,
    width: u32,
    height: u32,
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
    GalleryImage {
        index: usize,
        total: usize,
        name: String,
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
    /// 0 = native resolution, any other value = render at that max dimension.
    #[serde(default)]
    size: u32,
    #[serde(default)]
    mode: RenderMode,
}

#[derive(Deserialize, Default)]
struct SizeQuery {
    /// 0 = native resolution, any other value = render at that max dimension.
    #[serde(default)]
    size: u32,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn generate(Query(q): Query<SizeQuery>) -> Response {
    stream_response(ImageProgram::example_jxlart(), q.size, vec![])
}

async fn randomize(Query(q): Query<SizeQuery>) -> Response {
    let prog = tokio::task::spawn_blocking(random_program_non_degenerate)
        .await
        .expect("random_program_non_degenerate panicked");
    stream_response(prog, q.size, Mutation::showcase())
}

async fn random_batch(Query(q): Query<SizeQuery>) -> Response {
    const COUNT: usize = 20;
    let size = q.size;
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mut unordered: FuturesUnordered<_> = (0..COUNT)
            .map(|i| {
                tokio::task::spawn_blocking(move || {
                    let prog = random_program_non_degenerate();
                    let program_text = prog.to_text();
                    let image = render_to_payload(&program_text, size);
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

/// NDJSON lines (including the trailing `{"type":"done"}`) that make up a
/// gallery response. Computed once at startup by `prerender_gallery` so the
/// `/api/gallery` endpoint just replays them.
static GALLERY_LINES: OnceLock<Vec<Bytes>> = OnceLock::new();

async fn gallery_handler() -> Response {
    let lines: &'static Vec<Bytes> = GALLERY_LINES
        .get()
        .expect("gallery cache not initialized");
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        for b in lines {
            // Bytes::clone is an O(1) Arc bump.
            if tx.send(b.clone()).await.is_err() {
                break;
            }
        }
    });

    let stream = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|bytes| (Ok::<_, Infallible>(bytes), rx))
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(stream))
        .unwrap()
}

/// Render every gallery entry once at startup and cache the serialized
/// NDJSON response in `GALLERY_LINES`. Subsequent clicks on the Gallery
/// button are instant — no encode, no decode, no subprocess.
async fn prerender_gallery() {
    let entries = gallery::entries();
    let total = entries.len();
    let start = std::time::Instant::now();
    println!("Pre-rendering gallery ({total} entries)…");

    let mut tasks: FuturesOrdered<_> = entries
        .into_iter()
        .enumerate()
        .map(|(index, e)| {
            tokio::task::spawn_blocking(move || {
                let image = render_to_payload(e.program_text, e.size);
                let item = StreamItem::GalleryImage {
                    index,
                    total,
                    name: e.name.to_string(),
                    program_text: e.program_text.to_string(),
                    image,
                };
                let line = serde_json::to_string(&item)
                    .expect("gallery payload serialization cannot fail");
                Bytes::from(line + "\n")
            })
        })
        .collect();

    let mut lines: Vec<Bytes> = Vec::with_capacity(total + 1);
    while let Some(result) = tasks.next().await {
        lines.push(result.expect("gallery prerender task panicked"));
    }
    let done = serde_json::to_string(&StreamItem::Done)
        .unwrap_or_else(|_| "{\"type\":\"done\"}".into());
    lines.push(Bytes::from(done + "\n"));

    GALLERY_LINES.set(lines).ok();
    println!("Gallery ready in {:.1}s.", start.elapsed().as_secs_f64());
}

async fn render(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    match req.mode {
        RenderMode::Single => Ok(stream_single(req.program_text, req.size)),
        RenderMode::Mutations => {
            let prog = ImageProgram::from_text(&req.program_text)
                .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
            Ok(stream_response(prog, req.size, Mutation::showcase()))
        }
        RenderMode::Compound20 => {
            let prog = ImageProgram::from_text(&req.program_text)
                .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
            Ok(stream_response(prog, req.size, random_compounds(20)))
        }
    }
}

async fn download_png(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let text = req.program_text;
    let png = tokio::task::spawn_blocking(move || {
        let (rgba, w, h) = render_roundtrip(&text, 0)?;
        encode_png(rgba, w, h)
    })
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"artxl.png\"")
        .body(Body::from(png))
        .unwrap())
}

async fn download_jxl(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let text = req.program_text;
    let jxl = tokio::task::spawn_blocking(move || encode_jxl_from_tree(&text))
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/jxl")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"artxl.jxl\"")
        .body(Body::from(jxl))
        .unwrap())
}

#[derive(Serialize)]
struct JxlSizeResponse {
    size: u64,
}

async fn jxl_size(Json(req): Json<RenderRequest>) -> Json<JxlSizeResponse> {
    let text = req.program_text;
    let size = tokio::task::spawn_blocking(move || {
        encode_jxl_from_tree(&text).map(|v| v.len() as u64).unwrap_or(0)
    })
    .await
    .unwrap_or(0);
    Json(JxlSizeResponse { size })
}

// ── Streaming response ────────────────────────────────────────────────────────

/// Render `program_text` via the roundtrip pipeline and wrap as a payload;
/// on failure emit the striped "unsupported" placeholder.
fn render_to_payload(program_text: &str, size: u32) -> ImagePayload {
    match render_roundtrip(program_text, size) {
        Ok((rgba, w, h)) => to_payload(&rgba, w, h),
        Err(_) => unsupported_placeholder(),
    }
}

/// Render a single program (no mutations) — for `RenderMode::Single`. We
/// bypass our strict Rust parser on purpose, so users can render any jxl-art
/// text `jxl_from_tree` accepts, even if it uses syntax our tree data types
/// don't model.
fn stream_single(program_text: String, size: u32) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let program_text_for_payload = program_text.clone();
        let handle = tokio::task::spawn_blocking(move || {
            render_to_payload(&program_text, size)
        });
        if let Ok(image) = handle.await {
            let item = StreamItem::Original {
                program_text: program_text_for_payload,
                image,
                mutation_count: 0,
            };
            if let Ok(line) = serde_json::to_string(&item) {
                let _ = tx.send(Bytes::from(line + "\n")).await;
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

fn stream_response(prog: ImageProgram, size: u32, mutations: Vec<Mutation>) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mutation_count = mutations.len();

        let prog_orig = prog.clone();
        let orig_handle = tokio::task::spawn_blocking(move || {
            let program_text = prog_orig.to_text();
            let image = render_to_payload(&program_text, size);
            (program_text, image)
        });

        let mut ordered: FuturesOrdered<_> = mutations
            .into_iter()
            .map(|m| {
                let compound = m.is_compound();
                let p = prog.clone();
                tokio::task::spawn_blocking(move || {
                    const MAX_RETRIES: usize = 5;
                    let mut current = m;
                    let mut retries = 0;

                    let (label, text, image) = loop {
                        let mutated = current.apply(&p);
                        let text = mutated.to_text();
                        match render_roundtrip(&text, size) {
                            Ok((rgba, w, h)) => {
                                // For compound mutations, retry with a fresh random compound
                                // if the result is degenerate (flat / solid colour).
                                if compound && is_degenerate(&rgba) && retries < MAX_RETRIES {
                                    retries += 1;
                                    current = random_compounds(1).pop().unwrap();
                                    continue;
                                }
                                break (current.label(), text, to_payload(&rgba, w, h));
                            }
                            Err(_) => {
                                // Unrenderable mutation (shouldn't normally happen, but
                                // fall back to the placeholder so the stream keeps flowing).
                                break (current.label(), text, unsupported_placeholder());
                            }
                        }
                    };

                    (label, text, image, compound, None::<String>)
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

fn to_payload(rgba: &[u8], width: u32, height: u32) -> ImagePayload {
    let webp = encode_preview_webp(rgba, width, height);
    ImagePayload {
        webp_b64: base64::engine::general_purpose::STANDARD.encode(&webp),
        width,
        height,
    }
}

/// Dark-grey diagonally-striped 256×256 placeholder used when the roundtrip
/// render fails (e.g. `jxl_from_tree` rejects the program or `jxl-oxide` can't
/// decode its output). Visually distinct from any real render.
fn unsupported_placeholder() -> ImagePayload {
    const W: u32 = 256;
    const H: u32 = 256;
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let idx = ((y * W + x) * 4) as usize;
            let shade: u8 = if (x + y) % 24 < 12 { 40 } else { 56 };
            rgba[idx]     = shade;
            rgba[idx + 1] = shade;
            rgba[idx + 2] = shade;
            rgba[idx + 3] = 255;
        }
    }
    to_payload(&rgba, W, H)
}

/// VP8L lossless WebP encoder for gallery previews. Trades CPU for a much
/// smaller payload than fast-PNG — the user's server is CPU-idle and upload
/// bandwidth is the bottleneck. Still lossless so compare/zoom stays exact.
fn encode_preview_webp(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    use image::codecs::webp::WebPEncoder;
    use image::{ExtendedColorType, ImageEncoder};
    let mut buf = Vec::new();
    let enc = WebPEncoder::new_lossless(&mut buf);
    enc.write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .expect("webp encode");
    buf
}

// ── Encoding helpers ──────────────────────────────────────────────────────────

fn encode_png(rgba: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Rgba};
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba)
        .ok_or("failed to create image buffer")?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    prerender_gallery().await;

    let app = Router::new()
        .route("/api/generate", get(generate))
        .route("/api/random", get(randomize))
        .route("/api/random/batch", get(random_batch))
        .route("/api/gallery", get(gallery_handler))
        .route("/api/render", post(render))
        .route("/api/download/png", post(download_png))
        .route("/api/download/jxl", post(download_jxl))
        .route("/api/jxl_size", post(jxl_size))
        .fallback_service(ServeDir::new("static"));

    let addr = "0.0.0.0:3000";
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
