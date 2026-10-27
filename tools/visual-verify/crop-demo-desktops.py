#!/usr/bin/env python3
"""Crop desktop/taskbar/IDE bleed from enchant-iced demo PNG/GIF captures."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
DEMOS = ROOT / "crates" / "enchant-iced" / "docs" / "demos"


def to_rgb(im: Image.Image) -> Image.Image:
    if im.mode == "RGBA":
        bg = Image.new("RGB", im.size, (0, 0, 0))
        bg.paste(im, mask=im.split()[3])
        return bg
    return im.convert("RGB")


def find_left(arr: np.ndarray, max_scan: int = 64) -> int:
    h, w, _ = arr.shape
    y0, y1 = h // 5, 4 * h // 5
    left = 0
    for x in range(min(max_scan, w // 3)):
        col = arr[y0:y1, x, :].astype(np.int16)
        chroma = np.max(col, axis=1) - np.min(col, axis=1)
        # Desktop icon strip: intermittent high-chroma blobs on otherwise dark/wallpaper.
        high = float((chroma > 45).mean())
        mean_chroma = float(chroma.mean())
        if high > 0.06 or mean_chroma > 28:
            left = x + 1
            continue
        # Require a few consecutive clean columns to commit.
        clean = 0
        for x2 in range(x, min(x + 4, w)):
            c2 = arr[y0:y1, x2, :].astype(np.int16)
            ch2 = np.max(c2, axis=1) - np.min(c2, axis=1)
            if float((ch2 > 45).mean()) <= 0.06 and float(ch2.mean()) <= 28:
                clean += 1
            else:
                break
        if clean >= 3:
            break
        left = x + 1
    return left


def find_right(arr: np.ndarray, max_scan: int = 64) -> int:
    h, w, _ = arr.shape
    y0, y1 = h // 5, 4 * h // 5
    right = w - 1
    for x in range(w - 1, max(w - max_scan, 2 * w // 3), -1):
        col = arr[y0:y1, x, :].astype(np.int16)
        chroma = np.max(col, axis=1) - np.min(col, axis=1)
        lum = col.mean(axis=1)
        # IDE text bleed: many mid-luma high-frequency pixels.
        high = float((chroma > 40).mean())
        flips = float(np.mean(np.abs(np.diff(lum.astype(np.float32))) > 18))
        if high > 0.08 or flips > 0.25:
            right = x - 1
            continue
        break
    return max(right, 0)


def find_bottom(arr: np.ndarray, max_scan: int = 280) -> int:
    h, w, _ = arr.shape
    bottom = h - 1

    def row_peaks(y: int) -> tuple[float, int]:
        lum = arr[y, :, :].astype(np.float32).mean(axis=1)
        avg = float(lum.mean())
        peaks = 0
        mean = avg
        for i in range(2, len(lum) - 2, 2):
            if (
                lum[i] > lum[i - 1] + 8
                and lum[i] > lum[i + 1] + 8
                and lum[i] > mean + 15
            ):
                peaks += 1
        return avg, peaks

    # Prefer cropping just below the last bright UI row when a taskbar sits under it.
    last_light = None
    for y in range(h - 1, max(h // 3, h - max_scan), -1):
        if float(arr[y, :, :].mean()) > 170:
            last_light = y
            break
    if last_light is not None:
        # Look for a taskbar band within ~90px under the light content.
        for y in range(min(h - 1, last_light + 8), min(h - 1, last_light + 90) + 1):
            avg, peaks = row_peaks(y)
            if (avg < 90 and peaks >= 3) or (avg < 55 and peaks >= 2):
                return max(last_light + 4, 0)

    for y in range(h - 1, max(h - max_scan, h // 3), -1):
        avg, peaks = row_peaks(y)
        if (avg < 85 and peaks >= 3) or (avg < 50 and peaks >= 2):
            bottom = y - 1
            continue
        if avg < 100 and peaks >= 2 and y > h - 64:
            bottom = y - 1
            continue
        break
    return max(bottom, 0)


def content_box(im: Image.Image) -> tuple[int, int, int, int]:
    arr = np.asarray(to_rgb(im))
    h, w, _ = arr.shape
    left = find_left(arr)
    right = find_right(arr)
    bottom = find_bottom(arr)
    top = 0
    # Safety clamp
    if right - left < w * 0.6:
        left, right = 0, w - 1
    if bottom - top < h * 0.55:
        bottom = h - 1
    # Trim DWM shadow / residual IDE bleed.
    pad_x, pad_y = 8, 4
    left = min(left + pad_x, max(0, right - 20))
    right = max(right - pad_x, left + 20)
    bottom = max(bottom - pad_y, top + 20)
    return left, top, right + 1, bottom + 1


def crop_image(im: Image.Image, box: tuple[int, int, int, int]) -> Image.Image:
    return im.crop(box)


def process_png(path: Path) -> dict:
    im = Image.open(path)
    box = content_box(im)
    w, h = im.size
    l, t, r, b = box
    changed = (l, t, r, b) != (0, 0, w, h)
    if changed:
        crop_image(im, box).save(path)
    return {"file": path.name, "box": box, "from": f"{w}x{h}", "changed": changed}


def process_gif(path: Path) -> dict:
    im = Image.open(path)
    n = getattr(im, "n_frames", 1)
    # Box from middle frame
    im.seek(min(n // 2, n - 1))
    box = content_box(im)
    w, h = im.size
    l, t, r, b = box
    changed = (l, t, r, b) != (0, 0, w, h)
    if not changed:
        return {"file": path.name, "box": box, "from": f"{w}x{h}", "changed": False}

    frames: list[Image.Image] = []
    durations: list[int] = []
    for i in range(n):
        im.seek(i)
        frame = to_rgb(im.copy())
        frames.append(crop_image(frame, box))
        durations.append(int(im.info.get("duration", 70) or 70))

    frames[0].save(
        path,
        save_all=True,
        append_images=frames[1:],
        duration=durations,
        loop=im.info.get("loop", 0),
        optimize=False,
    )
    return {"file": path.name, "box": box, "from": f"{w}x{h}", "changed": True}


def main() -> None:
    results = []
    for path in sorted(DEMOS.glob("*.png")):
        if path.stat().st_size < 500:
            continue
        results.append(process_png(path))
    for path in sorted(DEMOS.glob("*.gif")):
        if path.stat().st_size < 2000:
            continue
        results.append(process_gif(path))

    changed = [r for r in results if r["changed"]]
    print(f"processed={len(results)} changed={len(changed)}")
    for r in changed[:30]:
        print(f"  {r['file']}: {r['from']} -> {r['box']}")
    if len(changed) > 30:
        print(f"  ... +{len(changed) - 30} more")


if __name__ == "__main__":
    main()
