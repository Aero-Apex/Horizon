#!/usr/bin/env python3
"""Makes the "Horizon Connected Glass" resource pack from the game's own glass
textures: the 47 connected-texture tiles of every glass block (OptiFine's
`optifine/ctm` format, which Horizon reads), so neighbouring panes of glass
lose the border between them.

    python tools/connected_glass.py [RESOURCES_ROOT] [OUTPUT.zip]

RESOURCES_ROOT is the prepared vanilla resources (it holds `assets/`), by
default `.local/horizon-client/26.3`. The output goes to `resourcepacks/`.
The textures are Mojang's, so the pack is generated on the player's machine
and is not distributed.
"""
import io
import json
import sys
import zipfile
from pathlib import Path

from PIL import Image

# The ctm method's tile for every combination of connections, as in
# `ctm.rs`: left 1, down 4, right 16, up 64 and the corners between two
# connected sides 2 (left-down), 8 (down-right), 32 (right-up), 128 (up-left).
TILES = [
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15,
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14,
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15,
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14,
    36, 17, 36, 17, 24, 19, 24, 43, 36, 17, 36, 17, 24, 19, 24, 43,
    16, 18, 16, 18, 6, 46, 6, 21, 16, 18, 16, 18, 28, 9, 28, 22,
    36, 17, 36, 17, 24, 19, 24, 43, 36, 17, 36, 17, 24, 19, 24, 43,
    37, 40, 37, 40, 30, 8, 30, 34, 37, 40, 37, 40, 25, 23, 25, 45,
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15,
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14,
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15,
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14,
    36, 39, 36, 39, 24, 41, 24, 27, 36, 39, 36, 39, 24, 41, 24, 27,
    16, 42, 16, 42, 6, 20, 6, 10, 16, 42, 16, 42, 28, 35, 28, 44,
    36, 39, 36, 39, 24, 41, 24, 27, 36, 39, 36, 39, 24, 41, 24, 27,
    37, 38, 37, 38, 30, 11, 30, 32, 37, 38, 37, 38, 25, 33, 25, 26,
]
COLORS = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray",
    "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black",
]
BLOCKS = [("glass", "glass")] + [(f"{c}_stained_glass", f"{c}_stained_glass") for c in COLORS]


def connections_of(tile):
    """One set of connections that gives `tile` (the lowest)."""
    return TILES.index(tile)


def make_tile(image, c):
    """`image` with the borders on the connected sides taken away."""
    w, h = image.size
    s = w // 16  # texture scale
    px = image.load()
    out = image.copy()
    o = out.load()
    left, down, right, up = c & 1, c & 4, c & 16, c & 64
    corner = {"ld": c & 2, "dr": c & 8, "ru": c & 32, "ul": c & 128}

    def inner(x, y):
        # What the interior shows next to the border pixel (x, y).
        ix = min(max(x, s), w - 1 - s)
        iy = min(max(y, s), h - 1 - s)
        return px[ix, iy]

    for x in range(w):
        for t in range(s):
            if up and s <= x < w - s:
                o[x, t] = inner(x, t)
            if down and s <= x < w - s:
                o[x, h - 1 - t] = inner(x, h - 1 - t)
    for y in range(h):
        for t in range(s):
            if left and s <= y < h - s:
                o[t, y] = inner(t, y)
            if right and s <= y < h - s:
                o[w - 1 - t, y] = inner(w - 1 - t, y)
    # A corner is open when both its sides connect and the block between does.
    for (a, b, d, (cx, cy)) in [
        (up, left, corner["ul"], (0, 0)),
        (up, right, corner["ru"], (w - s, 0)),
        (down, left, corner["ld"], (0, h - s)),
        (down, right, corner["dr"], (w - s, h - s)),
    ]:
        if a and b and d:
            for dx in range(s):
                for dy in range(s):
                    o[cx + dx, cy + dy] = inner(cx + dx, cy + dy)
    return out


def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".local/horizon-client/26.3")
    output = Path(sys.argv[2] if len(sys.argv) > 2 else "resourcepacks/Horizon Connected Glass.zip")
    textures = root / "assets/minecraft/textures/block"
    output.parent.mkdir(parents=True, exist_ok=True)
    made = 0
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as pack:
        pack.writestr(
            "pack.mcmeta",
            json.dumps({
                "pack": {
                    "pack_format": 46,
                    "supported_formats": {"min_inclusive": 46, "max_inclusive": 255},
                    "min_format": 46,
                    "max_format": 255,
                    "description": "§bConnected glass§r, made from the game's own textures by Horizon",
                }
            }),
        )
        for name, texture in BLOCKS:
            path = textures / f"{texture}.png"
            if not path.is_file():
                continue
            image = Image.open(path).convert("RGBA")
            folder = f"assets/minecraft/optifine/ctm/{name}"
            for tile in range(47):
                buffer = io.BytesIO()
                make_tile(image, connections_of(tile)).save(buffer, "PNG")
                pack.writestr(f"{folder}/{tile}.png", buffer.getvalue())
            pack.writestr(
                f"{folder}/{name}.properties",
                f"method=ctm\nmatchBlocks=minecraft:{name}\ntiles=0-46\nconnect=block\n",
            )
            made += 1
    print(f"{output}: {made} glass blocks")


if __name__ == "__main__":
    main()
