#!/usr/bin/env python3
"""Scrape jxl-art source programs from the three jpegxl.info gallery pages
plus hand-supplied URLs, decode each zcode parameter, validate by
compiling with ./jxl_from_tree, dedupe, and write:

  gallery/<prefix>-NNN-<slug>.jxlart   (source files)
  src/gallery_external.rs              (Rust entry list)

Run from the project root.
"""

import base64, hashlib, os, re, subprocess, sys, tempfile, urllib.parse, urllib.request, zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GALLERY = os.path.join(ROOT, "gallery")
OUT_RS = os.path.join(ROOT, "src", "gallery_external.rs")

PAGES = [
    ("p1",  "https://jpegxl.info/art/"),
    ("bg",  "https://jpegxl.info/art/big_gallery.html"),
    ("hdr", "https://jpegxl.info/art/hdr.html"),
]

EXTRA_SURMA = [
    # User-supplied: covers some page-1 images whose zcodes are missing from
    # the jpegxl.info HTML (e.g. "block nebula" — URL #3 @ 98 bytes).
    "https://jxl-art.surma.technology/?zcode=zZE7C8IwGEX3%2FIpvl0DSh3YSqlQ71SdkLia2EZuWNhX115vGqh3cXCQQTj4ON%2BSGSa5zoMTxUCxklusno111lkps2lRpeU%2B1LFXIT22jC6E04KGDAKgTAPllmYyx93OE6%2FxDhEMIBORDPooU76tCizotxLpsrLuqpWnTlgsBYt9%2FYjvfA%2FXRTGouKisgeYQDTMHC1QIGBtTumHbTm5nSSUdsGfdGeMmSUcLAfXME2LXOK8NeKzhgf3jyzWMe",
    "https://jxl-art.surma.technology/?zcode=C3IOUTCy5AotKE7MLchJVTDicsosSUktKMlQMDTg4spMU0hWsFMw4FJQADIDilLLgDxDI1MgX0FBV8HPVUHXCMoOVzCEqAqHagAJOpal%2B2n7hSvoGkIFglNLFEyMLbgA",
    "https://jxl-art.surma.technology/?zcode=xVLBDoIwDL3vK%2FoDS9iERC4mmhA5LSQcpldlCkcRjTF8vGND7MAMb%2B7UvbbvvTbNLzelnors9hsiq6IpgcU8IKmqzmUDLOAhIdUJjrCCkADoUOow0KH5tLLVX1MG5lHIVdPnbTkVn44eE1QkDmbQR8cUcwS%2Bi8Wo2ApJ41EVwEYp3ZRltbqvD9ekrr%2F0Ot2UebMRmzjqlsGiqaikUs6pTcV%2BcDvn1%2B%2FYt8X%2Feut2tk0ntA4hX5JZu%2FjobGzbhgPFqeF2XWnPqEgT%2B8c2FxzzaBoPhzMTnkfDLw%3D%3D",
]


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as r:
        return r.read().decode("utf-8")


def decode_zcode(z: str) -> str:
    z = urllib.parse.unquote(z)
    pad = z + "=" * ((4 - len(z) % 4) % 4)
    raw = base64.urlsafe_b64decode(pad)
    return zlib.decompress(raw, -15).decode("utf-8")


def slugify(s: str) -> str:
    s = re.sub(r"[^\w\s-]", "", s).strip().lower()
    s = re.sub(r"[\s_-]+", "-", s)
    return s[:60] or "untitled"


def validate(text: str) -> int:
    """Compile with jxl_from_tree, return output byte length or -1 on failure."""
    with tempfile.NamedTemporaryFile("w", suffix=".xl", delete=False) as fi:
        fi.write(text); fin = fi.name
    fout = fin.replace(".xl", ".jxl")
    try:
        r = subprocess.run(["./jxl_from_tree", fin, fout],
                           capture_output=True, cwd=ROOT, timeout=10)
        return os.path.getsize(fout) if r.returncode == 0 and os.path.exists(fout) else -1
    finally:
        for p in (fin, fout):
            try: os.unlink(p)
            except FileNotFoundError: pass


def extract_from_html(page_tag: str, url: str):
    """Yield (prefix, idx, title, decoded_text) for each img/zcode pair."""
    html = fetch(url)
    # Walk imgs in document order; for each, find most recent <h3> and zcode before it.
    idx = 0
    for m in re.finditer(r'<img\s+src="([^"]+)"[^>]*>', html):
        src = m.group(1)
        if not (src.endswith(".jxl") or src.startswith("data:image/jxl")):
            continue
        window = html[max(0, m.start()-2000):m.end()+50]
        tag = html[m.start():m.end()]
        alt = re.search(r'\balt="([^"]+)"', tag)
        h3s = re.findall(r'<h3[^>]*>([^<]+)</h3>', window)
        title = (alt.group(1) if alt and alt.group(1).strip()
                 else (h3s[-1] if h3s else "untitled")).strip()
        zcodes = re.findall(r'zcode=([^"&\s)]+)', window)
        idx += 1
        if not zcodes:
            print(f"  {page_tag}-{idx:03d} [{title}] — no zcode, skipped")
            continue
        try:
            text = decode_zcode(zcodes[-1])
        except Exception as e:
            print(f"  {page_tag}-{idx:03d} [{title}] — decode fail: {e}")
            continue
        yield page_tag, idx, title, text


def extract_from_surma(url: str, idx: int):
    q = urllib.parse.urlparse(url).query
    params = urllib.parse.parse_qs(q)
    z = params.get("zcode", [None])[0]
    if not z:
        print(f"  surma-{idx:03d} — no zcode param in URL, skipped")
        return None
    try:
        text = decode_zcode(z)
    except Exception as e:
        print(f"  surma-{idx:03d} — decode fail: {e}")
        return None
    return ("surma", idx, f"Surma extra {idx}", text)


def main():
    if not os.path.exists(os.path.join(ROOT, "jxl_from_tree")):
        sys.exit("jxl_from_tree binary missing — run `make setup` first.")

    # Collect everything
    items = []
    for tag, url in PAGES:
        print(f"--- {tag}: {url} ---")
        for it in extract_from_html(tag, url):
            items.append(it)
    print("--- surma extras ---")
    for i, u in enumerate(EXTRA_SURMA, 1):
        it = extract_from_surma(u, i)
        if it: items.append(it)
    print(f"\nCollected {len(items)} decoded programs. Validating + writing…")

    seen_hashes = {}
    written = []  # (filename, title)
    skipped_dupe = 0
    skipped_invalid = 0

    for prefix, idx, title, text in items:
        h = hashlib.sha256(text.encode("utf-8")).hexdigest()
        if h in seen_hashes:
            skipped_dupe += 1
            continue
        size = validate(text)
        if size < 0:
            print(f"  INVALID: {prefix}-{idx:03d} [{title}] — skipping")
            skipped_invalid += 1
            continue
        seen_hashes[h] = True
        fname = f"{prefix}-{idx:03d}-{slugify(title)}.jxlart"
        # Collision safety across prefixes (shouldn't happen but cheap to guard).
        base = fname
        n = 2
        while fname in {w[0] for w in written}:
            fname = base.replace(".jxlart", f"-{n}.jxlart"); n += 1
        with open(os.path.join(GALLERY, fname), "w") as fh:
            fh.write(text)
        written.append((fname, title))

    # Emit src/gallery_external.rs
    lines = [
        "// @generated by scripts/scrape_gallery.py — do not edit by hand.",
        "// Programs sourced from https://jpegxl.info/art/ (big_gallery + hdr)",
        "// and three extra zcodes from jxl-art.surma.technology.",
        "",
        "use crate::gallery::GalleryEntry;",
        "",
        "pub fn entries() -> Vec<GalleryEntry> {",
        "    vec![",
    ]
    for fname, title in written:
        esc_title = title.replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'        GalleryEntry {{ name: "{esc_title}", program_text: include_str!("../gallery/{fname}"), size: 0 }},')
    lines.append("    ]")
    lines.append("}")
    lines.append("")
    with open(OUT_RS, "w") as fh:
        fh.write("\n".join(lines))

    print(f"\n✅ wrote {len(written)} .jxlart files and {OUT_RS}")
    print(f"   skipped {skipped_dupe} duplicates, {skipped_invalid} invalid")


if __name__ == "__main__":
    main()
