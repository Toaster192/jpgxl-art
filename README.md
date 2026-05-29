# ArtXL

**Live:** <https://jxl-art.toaster.work/>

A web playground for [jxl-art][jxl-art-post] — a tiny tree-based image
program format embedded in JPEG XL bitstreams. Edit a program in the
browser, render it, and watch a grid of automated mutations re-interpret
the same tree in different ways. Encoded outputs are typically tens of
bytes — the default sky-and-grass landscape is **72 bytes** of JXL.

## What it does

The frontend is a single-page editor (vanilla JS, no build step). The
backend is a small Axum server in Rust that:

- Accepts a jxl-art program text.
- Encodes it via [`jxl_from_tree`][libjxl] into a real JXL file (no
  residuals — the MA decision tree drives every pixel).
- Decodes that JXL back to RGBA via [`jxl-oxide`][jxl-oxide] for display.
- Generates ~30 structured mutations of the tree per request and renders
  each in parallel.

Click any rendered card to zoom in, pin it to the comparison bar at the
bottom, save to your local-only collection, or share a permalink.

## Run locally

```bash
make setup     # one-time: clones libjxl v0.11.2, builds ./jxl_from_tree
make run       # cargo run --release; opens on http://localhost:3000
```

`make setup` configures libjxl against your system's highway / brotli /
lcms2 libraries instead of building them from scratch, and patches the
output binary to load its own private copies of `libjxl_threads` and
`libjxl_cms` from `./lib/`. Required system packages:

- Arch / CachyOS: `pacman -S highway brotli lcms2 patchelf cmake`
- Debian / Ubuntu: `apt install libhwy-dev libbrotli-dev liblcms2-dev cmake patchelf`

Plus a working Rust toolchain (1.75+ recommended).

There are no tests for the frontend; the Rust side runs:

```bash
cargo test                              # parser round-trip + mutation invariants
cargo test --test gallery_encode -- --ignored   # full gallery encode loop (~3 min)
```

CI runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --release`, and the fast tests on every PR.

## Deploy

`deploy/` has everything for a fresh-VPS bring-up: the systemd unit, an
idempotent `bootstrap.sh`, and a runbook (`deploy/README.md`) covering the
Cloudflare Tunnel setup and the GitHub Actions secrets the workflow needs.

## References

- Surma's introduction to jxl-art: <https://surma.dev/things/jxl-art/>
- The `jxl_from_tree` source (in libjxl):
  <https://github.com/libjxl/libjxl/blob/v0.11.2/tools/jxl_from_tree.cc>
- More example programs and a community editor: <https://jpegxl.info/art/>

[jxl-art-post]: https://surma.dev/things/jxl-art/
[libjxl]: https://github.com/libjxl/libjxl
[jxl-oxide]: https://github.com/tirr-c/jxl-oxide
