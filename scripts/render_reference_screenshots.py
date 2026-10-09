#!/usr/bin/env python3
"""Render deterministic TermXBoard TestBackend captures to PNG."""

import json
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "docs" / "screenshots"
FONT = "/System/Library/Fonts/Menlo.ttc"
CELL_W, CELL_H = 10, 20


def main() -> None:
    TARGET.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="termxboard-captures-") as temp:
        subprocess.run(
            ["cargo", "run", "--quiet", "--example", "capture_frames", "--", temp],
            cwd=ROOT,
            check=True,
        )
        for source in sorted(Path(temp).glob("*.json")):
            render(source, TARGET / f"{source.stem}.png")


def render(source: Path, target: Path) -> None:
    frame = json.loads(source.read_text())
    image = Image.new("RGB", (frame["width"] * CELL_W, frame["height"] * CELL_H), (5, 8, 12))
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype(FONT, 15)
    for index, cell in enumerate(frame["cells"]):
        x = index % frame["width"] * CELL_W
        y = index // frame["width"] * CELL_H
        draw.rectangle((x, y, x + CELL_W, y + CELL_H), fill=tuple(cell["bg"]))
        if cell["s"] != " ":
            draw.text((x, y), cell["s"], font=font, fill=tuple(cell["fg"]))
    image.save(target, optimize=True)


if __name__ == "__main__":
    main()
