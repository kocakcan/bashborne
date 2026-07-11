#!/usr/bin/env python3
"""One-off generator for assets/walk_frames.png.

Bashborne's bundled Kenney sheet (roguelike_characters.png) has exactly one
static frontal frame per character -- no walk-cycle art exists. Since no
image-generation tool is available, this script fakes a 3-frame walk cycle
(rest / step-left / step-right) by shifting each character's leg region
(bottom ~5px) 1px left or right, with the vacated edge column filled by
replicating the sprite's own edge pixels (so it reads as a leg sliding
rather than leaving a transparent gap).

Output sheet uses the same 17px-pitch/16px-tile convention as the existing
sheets (see render/assets.rs::cell): 3 columns (Rest, StepLeft, StepRight) x
1 row (the player). Only the player sprite animates in render/explore.rs --
NPCs stay on their existing static frame -- so that's the only row emitted.
"""

from PIL import Image

CELL = 17
TILE = 16
LEG_H = 5  # bottom N px treated as the "legs" region to shift

SRC = "assets/roguelike_characters.png"
DST = "assets/walk_frames.png"

# (label, col, row) source cells, in the row order the output sheet uses.
SOURCE_CELLS = [
    ("player", 0, 5),
]


def cell_box(col, row):
    x = col * CELL
    y = row * CELL
    return (x, y, x + TILE, y + TILE)


def shift_legs(frame: Image.Image, dx: int) -> Image.Image:
    """Returns a copy of `frame` with its bottom LEG_H rows shifted `dx`
    pixels horizontally, replicating the vacated edge column from the
    original so no transparent gap appears."""
    out = frame.copy()
    legs = frame.crop((0, TILE - LEG_H, TILE, TILE))
    shifted = Image.new("RGBA", (TILE, LEG_H), (0, 0, 0, 0))
    if dx > 0:
        shifted.paste(legs.crop((0, 0, TILE - dx, LEG_H)), (dx, 0))
        edge_col = legs.crop((0, 0, 1, LEG_H))
        for x in range(dx):
            shifted.paste(edge_col, (x, 0))
    elif dx < 0:
        adx = -dx
        shifted.paste(legs.crop((adx, 0, TILE, LEG_H)), (0, 0))
        edge_col = legs.crop((TILE - 1, 0, TILE, LEG_H))
        for x in range(adx):
            shifted.paste(edge_col, (TILE - 1 - x, 0))
    else:
        shifted = legs
    out.paste(shifted, (0, TILE - LEG_H))
    return out


def main():
    src = Image.open(SRC).convert("RGBA")
    cols = 3
    rows = len(SOURCE_CELLS)
    out_w = (cols - 1) * CELL + TILE
    out_h = (rows - 1) * CELL + TILE
    out = Image.new("RGBA", (out_w, out_h), (0, 0, 0, 0))

    for row, (label, col, src_row) in enumerate(SOURCE_CELLS):
        base = src.crop(cell_box(col, src_row))
        frames = [base, shift_legs(base, -1), shift_legs(base, 1)]
        for f_idx, frame in enumerate(frames):
            out.paste(frame, (f_idx * CELL, row * CELL))
        print(f"row {row}: {label} <- cell({col},{src_row})")

    out.save(DST)
    print(f"wrote {DST} ({out_w}x{out_h})")


if __name__ == "__main__":
    main()
