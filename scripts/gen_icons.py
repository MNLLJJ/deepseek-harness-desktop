#!/usr/bin/env python3
"""Generate a placeholder icon set for the Tauri app (pure stdlib).

Produces:
  src-tauri/icons/app-icon.png   (512x512 source)
  src-tauri/icons/icon.png       (512x512)
  src-tauri/icons/32x32.png
  src-tauri/icons/128x128.png
  src-tauri/icons/128x128@2x.png (256x256)
  src-tauri/icons/icon.ico       (16/32/48/256)
  src-tauri/icons/icon.icns      (128/256/512)

Replace app-icon.png with the real DeepSeek logo, then run:
  npx tauri icon src-tauri/icons/app-icon.png
to regenerate the full set with proper branding.
"""
import os
import struct
import zlib

ROOT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")


def _chunk(typ: bytes, data: bytes) -> bytes:
    c = struct.pack(">I", len(data)) + typ + data
    return c + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)


def make_png(size: int, pixels) -> bytes:
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # filter type: None
        for x in range(size):
            raw.extend(pixels[y * size + x])
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", idat)
        + _chunk(b"IEND", b"")
    )


def _in_rrect(px, py, x0, y0, x1, y1, r):
    if px < x0 or px > x1 or py < y0 or py > y1:
        return False
    cx = x0 + r if px < x0 + r else (x1 - r if px > x1 - r else px)
    cy = y0 + r if py < y0 + r else (y1 - r if py > y1 - r else py)
    dx, dy = px - cx, py - cy
    return dx * dx + dy * dy <= r * r


def render(size: int):
    px = [None] * (size * size)
    for iy in range(size):
        v = iy / size
        for ix in range(size):
            u = ix / size
            # DeepSeek-ish blue gradient background
            col = (int(58 + 48 * v), int(95 + 46 * v), 254, 255)
            # white terminal window
            if _in_rrect(u, v, 0.27, 0.35, 0.73, 0.65, 0.07):
                col = (255, 255, 255, 255)
                # ">" chevron
                if 0.40 <= u <= 0.45 and 0.41 <= v <= 0.59:
                    col = (77, 107, 254, 255)
                if 0.45 <= u <= 0.53 and 0.47 <= v <= 0.53:
                    col = (77, 107, 254, 255)
                # cursor block "_"
                if 0.56 <= u <= 0.63 and 0.52 <= v <= 0.58:
                    col = (77, 107, 254, 255)
            px[iy * size + ix] = col
    return px


def make_ico(sizes):
    images = [(s, make_png(s, render(s))) for s in sizes]
    count = len(images)
    data = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count
    entries, blobs = b"", b""
    for s, png in images:
        w = 0 if s >= 256 else s
        h = 0 if s >= 256 else s
        entries += struct.pack("<BBBBHHII", w, h, 0, 0, 1, 32, len(png), offset)
        blobs += png
        offset += len(png)
    return data + entries + blobs


def make_icns(entries):
    total = 8 + sum(8 + len(d) for _, d in entries)
    data = b"icns" + struct.pack(">I", total)
    for code, d in entries:
        data += code + struct.pack(">I", 8 + len(d)) + d
    return data


def main():
    os.makedirs(ROOT, exist_ok=True)
    pngs = {
        "app-icon.png": 512,
        "icon.png": 512,
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
    }
    for name, s in pngs.items():
        with open(os.path.join(ROOT, name), "wb") as f:
            f.write(make_png(s, render(s)))
    with open(os.path.join(ROOT, "icon.ico"), "wb") as f:
        f.write(make_ico([16, 32, 48, 256]))
    with open(os.path.join(ROOT, "icon.icns"), "wb") as f:
        f.write(
            make_icns(
                [
                    (b"ic07", make_png(128, render(128))),
                    (b"ic08", make_png(256, render(256))),
                    (b"ic09", make_png(512, render(512))),
                ]
            )
        )
    print("icons written to", ROOT)


if __name__ == "__main__":
    main()
