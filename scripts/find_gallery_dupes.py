#!/usr/bin/env python3
"""Report groups of gallery/*.jxlart files that are duplicates once
`/* ... */` block comments are stripped and whitespace is collapsed.

Run from the project root:

    python3 scripts/find_gallery_dupes.py
"""
from __future__ import annotations

import hashlib
import pathlib
import re
import sys
from collections import defaultdict


def strip_block_comments(s: str) -> str:
    """Replace each `/* ... */` block with a single space so token
    boundaries are preserved (mirrors src/tree.rs:strip_block_comments).
    Unterminated comments are kept verbatim."""
    out: list[str] = []
    i, n = 0, len(s)
    while i < n:
        if i + 1 < n and s[i] == "/" and s[i + 1] == "*":
            end = s.find("*/", i + 2)
            if end == -1:
                out.append(s[i:])
                break
            out.append(" ")
            i = end + 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def normalise(text: str) -> str:
    return re.sub(r"\s+", " ", strip_block_comments(text)).strip()


def main() -> int:
    root = pathlib.Path(__file__).resolve().parent.parent
    gallery = root / "gallery"
    if not gallery.is_dir():
        print(f"error: {gallery} is not a directory", file=sys.stderr)
        return 1

    buckets: dict[str, list[pathlib.Path]] = defaultdict(list)
    for path in sorted(gallery.glob("*.jxlart")):
        h = hashlib.sha256(normalise(path.read_text()).encode()).hexdigest()
        buckets[h].append(path)

    groups = [g for g in buckets.values() if len(g) >= 2]
    # Biggest groups first, then alphabetical.
    groups.sort(key=lambda g: (-len(g), g[0].name))

    total_files = sum(len(g) for g in groups)
    total_prunable = total_files - len(groups)

    for g in groups:
        print(f"=== {len(g)} duplicates ===")
        for p in g:
            print(f"  {p.relative_to(root)}")
        print()

    print(
        f"{len(groups)} duplicate group(s), "
        f"{total_files} file(s) across them, "
        f"{total_prunable} prunable."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
