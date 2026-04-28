mod codec;
mod gallery;
mod mutations;
mod render;
mod tree;

use axum::{
    body::Body,
    body::Bytes,
    extract::Query,
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
    routing::post,
    Json, Router,
};
use base64::Engine;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use tower_http::services::ServeDir;

/// Longest-edge cap applied to gallery thumbnails. Cards render at
/// ~360–720 CSS pixels even on retina, so 768 keeps a comfortable
/// zoom margin without shipping native-res pixels nobody sees.
const GALLERY_MAX_DIM: u32 = 768;

/// WebP quality for gallery thumbnails. Abstract jxl-art is extremely
/// high-entropy so even lossless VP8L only shaves ~10%; at q=75 VP8
/// lossy stays visually clean at card size while shrinking ~8×.
const GALLERY_WEBP_QUALITY: f32 = 75.0;

/// Cap on concurrent `spawn_blocking` render tasks. Each native-res JXL
/// decode owns ~w*h*7 bytes while Lanczos resize runs (2160×3840 entries
/// like `bg-024-erosion-and-shadows` peak around ~65MB). 8 keeps peak
/// under ~600MB during prerender while saturating a typical 8-core box.
const RENDER_CONCURRENCY: usize = 8;

use mutations::{
    is_degenerate, random_compounds, random_program_non_degenerate, Complexity, Mutation,
};
use render::{encode_jxl_from_tree, render_roundtrip};
use tree::ImageProgram;

// ── API types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ImagePayload {
    webp_b64: String,
    width: u32,
    height: u32,
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

#[derive(Deserialize, Default)]
struct RandomQuery {
    /// 0 = simple, 1 = normal (default), 2 = complex. Anything else maps
    /// to normal — keeps the endpoint forgiving on stale clients.
    #[serde(default)]
    complexity: u8,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn generate(Query(q): Query<SizeQuery>) -> Response {
    stream_response(ImageProgram::example_jxlart(), q.size, vec![])
}

async fn randomize(Query(q): Query<RandomQuery>) -> Response {
    let complexity = Complexity::from_u8(q.complexity);
    let prog = tokio::task::spawn_blocking(move || random_program_non_degenerate(complexity))
        .await
        .expect("random_program_non_degenerate panicked");
    stream_response(prog, 0, Mutation::showcase())
}

async fn random_batch(Query(q): Query<RandomQuery>) -> Response {
    const COUNT: usize = 20;
    let complexity = Complexity::from_u8(q.complexity);
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mut unordered = stream::iter(0..COUNT)
            .map(|i| async move {
                tokio::task::spawn_blocking(move || {
                    let prog = random_program_non_degenerate(complexity);
                    let program_text = prog.to_text();
                    let image = render_to_payload(&program_text, 0, encode_preview_webp);
                    (i, program_text, image)
                })
                .await
            })
            .buffer_unordered(RENDER_CONCURRENCY);

        while let Some(Ok((index, program_text, image))) = unordered.next().await {
            send_item(
                &tx,
                &StreamItem::BatchImage {
                    index,
                    total: COUNT,
                    program_text,
                    image,
                },
            )
            .await;
        }

        send_done(&tx).await;
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(ndjson_stream(rx)))
        .unwrap()
}

/// NDJSON lines + ETag computed once at startup by `prerender_gallery`.
/// `/api/gallery` replays the lines; the ETag lets the browser skip the
/// download entirely on repeat opens.
struct GalleryCache {
    lines: Vec<Bytes>,
    etag: String,
}

static GALLERY: OnceLock<GalleryCache> = OnceLock::new();

async fn gallery_handler(headers: HeaderMap) -> Response {
    let cache: &'static GalleryCache = GALLERY.get().expect("gallery cache not initialized");

    if let Some(inm) = headers.get(header::IF_NONE_MATCH) {
        if inm.to_str().map(|s| s == cache.etag).unwrap_or(false) {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, &cache.etag)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::empty())
                .unwrap();
        }
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        for b in &cache.lines {
            // Bytes::clone is an O(1) Arc bump.
            if tx.send(b.clone()).await.is_err() {
                break;
            }
        }
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .header(header::ETAG, &cache.etag)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from_stream(ndjson_stream(rx)))
        .unwrap()
}

/// Render every gallery entry once at startup and cache the serialized
/// NDJSON response in `GALLERY`. Subsequent clicks on the Gallery button
/// are instant — no encode, no decode, no subprocess. Each entry is
/// capped at `GALLERY_MAX_DIM` longest edge and encoded as lossy WebP so
/// the total payload stays around ~5MB instead of ~60MB.
async fn prerender_gallery() {
    let entries = gallery::entries();
    let total = entries.len();
    let start = std::time::Instant::now();
    println!("Pre-rendering gallery ({total} entries)…");

    // Render unordered so a slow entry doesn't stall the pipeline behind it,
    // then re-sort by `index` before sealing the cache so the gallery still
    // displays in `gallery::entries()` order across restarts.
    let mut tasks = stream::iter(entries.into_iter().enumerate())
        .map(|(index, e)| async move {
            tokio::task::spawn_blocking(move || {
                let size = if e.size == 0 {
                    GALLERY_MAX_DIM
                } else {
                    e.size.min(GALLERY_MAX_DIM)
                };
                let t0 = std::time::Instant::now();
                let image = render_to_payload_logged(
                    e.program_text,
                    size,
                    encode_gallery_webp,
                    Some(e.name),
                );
                let elapsed = t0.elapsed();
                let item = StreamItem::GalleryImage {
                    index,
                    total,
                    name: e.name.to_string(),
                    program_text: e.program_text.to_string(),
                    image,
                };
                let line = serde_json::to_string(&item)
                    .expect("gallery payload serialization cannot fail");
                (index, ndjson(line), e.name, elapsed)
            })
            .await
            .expect("gallery prerender task panicked")
        })
        .buffer_unordered(RENDER_CONCURRENCY);

    let mut indexed: Vec<(usize, Bytes)> = Vec::with_capacity(total);
    let pad = total.to_string().len();
    let mut done_count = 0usize;
    while let Some((index, line, name, elapsed)) = tasks.next().await {
        indexed.push((index, line));
        done_count += 1;
        println!(
            "  [{done_count:>pad$}/{total}] {name} ({:.1}s)",
            elapsed.as_secs_f64(),
        );
    }
    indexed.sort_by_key(|(i, _)| *i);

    let mut lines: Vec<Bytes> = Vec::with_capacity(total + 1);
    lines.extend(indexed.into_iter().map(|(_, line)| line));
    let done =
        serde_json::to_string(&StreamItem::Done).unwrap_or_else(|_| "{\"type\":\"done\"}".into());
    lines.push(ndjson(done));

    let etag = compute_etag(&lines);
    let total_bytes: usize = lines.iter().map(|b| b.len()).sum();

    GALLERY.set(GalleryCache { lines, etag }).ok();
    println!(
        "Gallery ready in {:.1}s ({:.1} MB).",
        start.elapsed().as_secs_f64(),
        total_bytes as f64 / (1024.0 * 1024.0),
    );
}

/// Append a trailing newline and wrap into a `Bytes` without the extra
/// intermediate allocation `Bytes::from(s + "\n")` would cause.
fn ndjson(mut line: String) -> Bytes {
    line.push('\n');
    Bytes::from(line)
}

/// Adapt an mpsc receiver into an axum-compatible stream of `Bytes` lines.
fn ndjson_stream(
    rx: tokio::sync::mpsc::Receiver<Bytes>,
) -> impl futures::Stream<Item = Result<Bytes, Infallible>> {
    futures::stream::unfold(rx, |mut rx| async move {
        rx.recv()
            .await
            .map(|bytes| (Ok::<_, Infallible>(bytes), rx))
    })
}

async fn send_item(tx: &tokio::sync::mpsc::Sender<Bytes>, item: &StreamItem) {
    if let Ok(line) = serde_json::to_string(item) {
        let _ = tx.send(ndjson(line)).await;
    }
}

async fn send_done(tx: &tokio::sync::mpsc::Sender<Bytes>) {
    let done =
        serde_json::to_string(&StreamItem::Done).unwrap_or_else(|_| "{\"type\":\"done\"}".into());
    let _ = tx.send(ndjson(done)).await;
}

fn compute_etag(lines: &[Bytes]) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for b in lines {
        b.as_ref().hash(&mut h);
    }
    format!("\"{:016x}\"", h.finish())
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
        let (rgba, w, h, _) = render_roundtrip(&text, 0)?;
        encode_png(rgba, w, h)
    })
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"artxl.png\"",
        )
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
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"artxl.jxl\"",
        )
        .body(Body::from(jxl))
        .unwrap())
}

// ── Streaming response ────────────────────────────────────────────────────────

type WebpEncoder = fn(&[u8], u32, u32) -> Vec<u8>;

/// Render `program_text` via the roundtrip pipeline and wrap as a payload;
/// on failure emit the striped "unsupported" placeholder. `encoder`
/// picks between `encode_preview_webp` (lossless, for interactive
/// render/mutation cards) and `encode_gallery_webp` (lossy, for the
/// pre-rendered gallery).
fn render_to_payload(program_text: &str, size: u32, encoder: WebpEncoder) -> ImagePayload {
    render_to_payload_logged(program_text, size, encoder, None)
}

/// Same as `render_to_payload`, but logs failures to stderr with a
/// caller-supplied label (e.g. the gallery entry name) so prerender
/// errors aren't silently swallowed into placeholders.
fn render_to_payload_logged(
    program_text: &str,
    size: u32,
    encoder: WebpEncoder,
    log_label: Option<&str>,
) -> ImagePayload {
    match render_roundtrip(program_text, size) {
        Ok((rgba, w, h, jxl_size)) => to_payload(&rgba, w, h, jxl_size, encoder),
        Err(e) => {
            if let Some(label) = log_label {
                eprintln!("render failed for {label:?}: {e}");
            }
            unsupported_placeholder()
        }
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
            render_to_payload(&program_text, size, encode_preview_webp)
        });
        if let Ok(image) = handle.await {
            send_item(
                &tx,
                &StreamItem::Original {
                    program_text: program_text_for_payload,
                    image,
                    mutation_count: 0,
                },
            )
            .await;
        }
        send_done(&tx).await;
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(ndjson_stream(rx)))
        .unwrap()
}

fn stream_response(prog: ImageProgram, size: u32, mutations: Vec<Mutation>) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        let mutation_count = mutations.len();

        let prog_orig = prog.clone();
        let orig_handle = tokio::task::spawn_blocking(move || {
            let program_text = prog_orig.to_text();
            let image = render_to_payload(&program_text, size, encode_preview_webp);
            (program_text, image)
        });

        let mut ordered = stream::iter(mutations)
            .map(|m| {
                let compound = m.is_compound();
                let p = prog.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        const MAX_RETRIES: usize = 5;
                        let mut current = m;
                        let mut retries = 0;

                        let (label, text, image) = loop {
                            let mutated = current.apply(&p);
                            let text = mutated.to_text();
                            match render_roundtrip(&text, size) {
                                Ok((rgba, w, h, jxl_size)) => {
                                    // For compound mutations, retry with a fresh random compound
                                    // if the result is degenerate (flat / solid colour).
                                    if compound && is_degenerate(&rgba) && retries < MAX_RETRIES {
                                        retries += 1;
                                        current = random_compounds(1).pop().unwrap();
                                        continue;
                                    }
                                    break (
                                        current.label(),
                                        text,
                                        to_payload(&rgba, w, h, jxl_size, encode_preview_webp),
                                    );
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
                    .await
                }
            })
            .buffered(RENDER_CONCURRENCY);

        if let Ok((program_text, image)) = orig_handle.await {
            send_item(
                &tx,
                &StreamItem::Original {
                    program_text,
                    image,
                    mutation_count,
                },
            )
            .await;
        }

        while let Some(Ok((label, program_text, image, compound, warning))) = ordered.next().await {
            send_item(
                &tx,
                &StreamItem::Mutation {
                    label,
                    program_text,
                    image,
                    compound,
                    warning,
                },
            )
            .await;
        }

        send_done(&tx).await;
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(ndjson_stream(rx)))
        .unwrap()
}

// ── Payload builder ───────────────────────────────────────────────────────────

fn to_payload(
    rgba: &[u8],
    width: u32,
    height: u32,
    jxl_size: u64,
    encoder: WebpEncoder,
) -> ImagePayload {
    let webp = encoder(rgba, width, height);
    ImagePayload {
        webp_b64: base64::engine::general_purpose::STANDARD.encode(&webp),
        width,
        height,
        jxl_size,
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
            rgba[idx] = shade;
            rgba[idx + 1] = shade;
            rgba[idx + 2] = shade;
            rgba[idx + 3] = 255;
        }
    }
    to_payload(&rgba, W, H, 0, encode_preview_webp)
}

/// VP8L lossless WebP encoder for preview payloads (used for
/// render/mutation/randomize cards). Lossless so compare/zoom stays exact.
fn encode_preview_webp(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    use image::codecs::webp::WebPEncoder;
    use image::{ExtendedColorType, ImageEncoder};
    let mut buf = Vec::new();
    let enc = WebPEncoder::new_lossless(&mut buf);
    enc.write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .expect("webp encode");
    buf
}

/// Lossy VP8 WebP encoder used only for the pre-rendered gallery. At
/// quality 82 on high-entropy abstract imagery this is ~5–8× smaller
/// than VP8L and visually indistinguishable at card size, cutting
/// `/api/gallery` from ~60MB to ~5MB.
fn encode_gallery_webp(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let enc = webp::Encoder::from_rgba(rgba, width, height);
    enc.encode(GALLERY_WEBP_QUALITY).to_vec()
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

fn main() {
    // Tokio's default blocking pool is 512 threads. Each gets its own glibc
    // malloc arena, which holds onto freed memory indefinitely (per-thread
    // arena fragmentation). Since our per-request blocking work is capped at
    // RENDER_CONCURRENCY, there's no reason to let hundreds of idle threads
    // hoard multi-GB of returned-but-not-returned-to-OS allocations.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(RENDER_CONCURRENCY * 2)
        .build()
        .expect("build tokio runtime");

    runtime.block_on(async {
        prerender_gallery().await;

        let app = Router::new()
            .route("/api/generate", get(generate))
            .route("/api/random", get(randomize))
            .route("/api/random/batch", get(random_batch))
            .route("/api/gallery", get(gallery_handler))
            .route("/api/render", post(render))
            .route("/api/download/png", post(download_png))
            .route("/api/download/jxl", post(download_jxl))
            .fallback_service(ServeDir::new("static"));

        let addr = "0.0.0.0:3000";
        println!("Listening on http://{}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
}
