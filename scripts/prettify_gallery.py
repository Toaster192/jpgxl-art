#!/usr/bin/env python3
"""
Re-indent every gallery/*.jxlart file with consistent 2-space indentation.

Token-based: the file is tokenised in source order, then:

  1. Known header directives + their args are pulled off the front and
     emitted one-per-line (even when the source had them glued onto a body
     line — see gallery/14-surma-deltapalette.jxlart).
  2. Each `Spline ... EndSpline` block is preserved verbatim and inserted
     between the header and the body.
  3. The remaining tokens are walked as a jxl-art tree: `if <var> <op> <thr>`
     has exactly two children, `-` + predictor-name + optional-offset is a
     leaf. Depth determines indentation.

Run from the project root:

    python3 scripts/prettify_gallery.py
"""

from __future__ import annotations

import pathlib
import sys


# Directive → arg-count.
# 'maybe_str' means "one optional string arg if next token looks like one".
DIRECTIVE_ARGS: dict[str, int | str] = {
    # Standard JXL header fields.
    "Bitdepth": 1,
    "Width": 1,
    "Height": 1,
    "Channels": 1,
    "Orientation": 1,
    "RCT": 1,
    # jxl-art extras.
    "Squeeze": 0,
    "DeltaPalette": 0,
    "Gaborish": 0,
    "XYB": 0,
    "Alpha": 0,
    "NotLast": 0,
    "EPF": 1,
    "Upsample": 1,
    "HiddenChannel": 1,
    "Noise": 8,
    "FramePos": 2,
    "Rec2100": "maybe_str",
}

REC2100_MODES = {"HLG", "PQ", "SDR"}


def is_signed_int(s: str) -> bool:
    if not s:
        return False
    t = s[1:] if s[0] in "+-" else s
    return bool(t) and t.isdigit()


def tokenise_preserving_splines(text: str):
    """Yield ('token', str) or ('spline', full_block_text) in source order."""
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if not stripped:
            i += 1
            continue
        parts = stripped.split()
        if parts[0] == "Spline":
            block = [line.rstrip()]
            i += 1
            while i < len(lines):
                block.append(lines[i].rstrip())
                if "EndSpline" in lines[i].split():
                    break
                i += 1
            i += 1
            yield ("spline", "\n".join(block))
            continue
        for p in parts:
            yield ("token", p)
        i += 1


def format_tree(tokens: list[str]) -> list[str]:
    out: list[str] = []
    pos = 0

    def indent(depth: int) -> str:
        return "  " * depth

    def consume_leaf(start: int) -> tuple[list[str], int]:
        """Consume one `- <name> [offset...]` leaf from `start`."""
        i = start
        i += 1  # '-'
        if i >= len(tokens):
            return tokens[start:i], i
        name = tokens[i]
        i += 1
        if name == "Set":
            if i < len(tokens):
                i += 1  # value
            return tokens[start:i], i
        # Offset: "0", "+ N", "- N", or a signed int literal. None if next is `if`/`-`.
        if i < len(tokens):
            tok = tokens[i]
            if tok == "0":
                i += 1
            elif tok in ("+", "-"):
                i += 1
                if i < len(tokens):
                    i += 1
            elif is_signed_int(tok):
                i += 1
            # else: no offset on this leaf.
        return tokens[start:i], i

    def walk(depth: int) -> None:
        nonlocal pos
        if pos >= len(tokens):
            return
        tok = tokens[pos]
        if tok == "if":
            # `if <var> <op> <thr>` + two children
            if pos + 3 >= len(tokens):
                out.append(indent(depth) + " ".join(tokens[pos:]))
                pos = len(tokens)
                return
            header = tokens[pos : pos + 4]
            out.append(indent(depth) + " ".join(header))
            pos += 4
            walk(depth + 1)
            walk(depth + 1)
        elif tok == "-":
            leaf, new_pos = consume_leaf(pos)
            pos = new_pos
            out.append(indent(depth) + " ".join(leaf))
        else:
            # Stray token that doesn't fit our grammar. Keep it so nothing is
            # silently dropped; a human can see the file needs manual attention.
            out.append(indent(depth) + tok)
            pos += 1

    while pos < len(tokens):
        walk(0)
    return out


def prettify(text: str) -> str:
    items = list(tokenise_preserving_splines(text))
    header_lines: list[str] = []
    splines: list[str] = []
    body_tokens: list[str] = []
    in_body = False

    i = 0
    while i < len(items):
        kind, value = items[i]
        if kind == "spline":
            splines.append(value)
            i += 1
            continue
        # kind == 'token'
        if not in_body:
            if value in DIRECTIVE_ARGS:
                spec = DIRECTIVE_ARGS[value]
                if spec == 0:
                    header_lines.append(value)
                    i += 1
                    continue
                if spec == "maybe_str":
                    args: list[str] = []
                    if i + 1 < len(items) and items[i + 1][0] == "token" \
                            and items[i + 1][1] in REC2100_MODES:
                        args.append(items[i + 1][1])
                        i += 1
                    header_lines.append(value + ((" " + " ".join(args)) if args else ""))
                    i += 1
                    continue
                if isinstance(spec, int):
                    args = []
                    consumed = 0
                    j = i + 1
                    while consumed < spec and j < len(items) and items[j][0] == "token":
                        args.append(items[j][1])
                        j += 1
                        consumed += 1
                    header_lines.append(value + " " + " ".join(args))
                    i = j
                    continue
            # Unknown top-level token — start of body.
            in_body = True
        if in_body:
            body_tokens.append(value)
            i += 1

    lines: list[str] = list(header_lines)
    lines.append("")
    for s in splines:
        lines.append(s)
        lines.append("")
    lines.extend(format_tree(body_tokens))
    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines) + "\n"


def main() -> int:
    root = pathlib.Path(__file__).resolve().parent.parent
    gallery = root / "gallery"
    if not gallery.is_dir():
        print(f"error: {gallery} is not a directory", file=sys.stderr)
        return 1
    changed = 0
    for path in sorted(gallery.glob("*.jxlart")):
        original = path.read_text()
        pretty = prettify(original)
        if pretty != original:
            path.write_text(pretty)
            print(f"  {path.relative_to(root)}")
            changed += 1
    print(f"Prettified {changed} of {len(list(gallery.glob('*.jxlart')))} files.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
