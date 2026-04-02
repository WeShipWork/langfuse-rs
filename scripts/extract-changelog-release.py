#!/usr/bin/env python3

from __future__ import annotations

import re
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: extract-changelog-release.py <version>", file=sys.stderr)
        return 1

    version = sys.argv[1].removeprefix("v")
    changelog_path = Path("CHANGELOG.md")
    content = changelog_path.read_text(encoding="utf-8")

    heading_pattern = re.compile(
        rf"^## \[{re.escape(version)}\] - .*?$",
        re.MULTILINE,
    )
    start_match = heading_pattern.search(content)
    if start_match is None:
        print(f"version {version} not found in {changelog_path}", file=sys.stderr)
        return 1

    start = start_match.end()
    next_heading_match = re.search(r"^## \[", content[start:], re.MULTILINE)
    reference_block_match = re.search(r"^\[[^\]]+\]:\s+", content[start:], re.MULTILINE)

    end_candidates = [len(content)]
    if next_heading_match:
        end_candidates.append(start + next_heading_match.start())
    if reference_block_match:
        end_candidates.append(start + reference_block_match.start())

    end = min(end_candidates)

    section = content[start:end].strip()
    if not section:
        print(f"version {version} has no changelog body", file=sys.stderr)
        return 1

    print(section)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
