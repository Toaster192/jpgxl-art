mod gallery;
mod mutations;
mod tree;

use axum::{Json, Router, body::Body, body::Bytes, extract::Query, http::header, response::Response, routing::get, routing::post};
use base64::Engine;
use futures::stream::{FuturesOrdered, FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tower_http::services::ServeDir;

use mutations::{is_degenerate, random_compounds, random_program_non_degenerate, Mutation};
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
                    let (rgba, w, h) = render_at_size(&prog, size);
                    let image = to_payload(&rgba, w, h);
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

async fn gallery_handler() -> Response {
    let entries = gallery::entries();
    let total = entries.len();
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mut ordered: FuturesOrdered<_> = entries
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                tokio::task::spawn_blocking(move || {
                    let image = match ImageProgram::from_text(e.program_text) {
                        Ok(prog) => {
                            let (rgba, w, h) = render_at_size(&prog, e.size);
                            to_payload(&rgba, w, h)
                        }
                        Err(_) => unsupported_placeholder(),
                    };
                    (i, e.name.to_string(), e.program_text.to_string(), image)
                })
            })
            .collect();

        while let Some(result) = ordered.next().await {
            if let Ok((index, name, program_text, image)) = result {
                let item = StreamItem::GalleryImage {
                    index, total, name, program_text, image,
                };
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
    Ok(stream_response(prog, req.size, mutations))
}

async fn download_png(
    Json(req): Json<RenderRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let prog = ImageProgram::from_text(&req.program_text)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    let (rgba, w, h) = prog.render_display();
    let png = encode_png(rgba, w, h)
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

#[derive(Serialize)]
struct JxlSizeResponse {
    size: u64,
}

async fn jxl_size(Json(req): Json<RenderRequest>) -> Json<JxlSizeResponse> {
    // Validate cheaply on the runtime thread; push subprocess cost to blocking.
    if ImageProgram::from_text(&req.program_text).is_err() {
        return Json(JxlSizeResponse { size: 0 });
    }
    let text = req.program_text;
    let size = tokio::task::spawn_blocking(move || {
        encode_jxl_from_tree(&text).map(|v| v.len() as u64).unwrap_or(0)
    })
    .await
    .unwrap_or(0);
    Json(JxlSizeResponse { size })
}

// ── Streaming response ────────────────────────────────────────────────────────

fn render_at_size(prog: &ImageProgram, size: u32) -> (Vec<u8>, u32, u32) {
    if size == 0 { prog.render_display() } else { prog.render_display_at(size) }
}

fn stream_response(prog: ImageProgram, size: u32, mutations: Vec<Mutation>) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mutation_count = mutations.len();

        let prog_orig = prog.clone();
        let orig_handle = tokio::task::spawn_blocking(move || {
            let program_text = prog_orig.to_text();
            let (rgba, w, h) = render_at_size(&prog_orig, size);
            let image = to_payload(&rgba, w, h);
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

                    let (label, text, rgba, w, h) = loop {
                        let mutated = current.apply(&p);
                        let text = mutated.to_text();
                        let (rgba, w, h) = render_at_size(&mutated, size);
                        // For compound mutations, retry with a fresh random compound
                        // if the result is degenerate (flat / solid colour).
                        if compound && is_degenerate(&rgba) && retries < MAX_RETRIES {
                            retries += 1;
                            current = random_compounds(1).pop().unwrap();
                            continue;
                        }
                        break (current.label(), text, rgba, w, h);
                    };

                    let image = to_payload(&rgba, w, h);
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

/// Dark-grey diagonally-striped 256×256 placeholder used when a gallery
/// entry's program text fails to parse (unsupported syntax). Visually
/// distinct from any valid render so it reads as "this one doesn't render
/// yet" at a glance.
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
