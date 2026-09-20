"""Compose the approved foreground PNGs and export desktop icons (requires Pillow)."""

from pathlib import Path

from PIL import Image, ImageColor, ImageDraw, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
SIZE = 1024
TILE_INSET = 102
TILE_SIZE = SIZE - 2 * TILE_INSET
BACKGROUNDS = {"light": ("#F5F6F7", "#CED4DC"), "dark": ("#23272E", "#23272E")}


def background(theme: str) -> Image.Image:
    top, bottom = [ImageColor.getrgb(color) for color in BACKGROUNDS[theme]]
    strip = Image.new("RGB", (1, SIZE))
    strip.putdata([
        tuple(round(start + (end - start) * y / (SIZE - 1))
              for start, end in zip(top, bottom))
        for y in range(SIZE)
    ])
    return strip.resize((SIZE, SIZE)).convert("RGBA")


def compose_foregrounds() -> dict[str, Image.Image]:
    sources = {
        theme: Image.open(ROOT / "assets" / "app-icon" / f"{theme}.png").convert("RGBA")
        for theme in BACKGROUNDS
    }
    # Share crop and scale so switching appearance doesn't move the artwork.
    if sources["light"].size != sources["dark"].size:
        raise ValueError("Light and dark source canvases must have matching dimensions")
    bounds = [image.getchannel("A").point(lambda a: 255 if a >= 128 else 0).getbbox()
              for image in sources.values()]
    if any(bound is None for bound in bounds):
        raise ValueError("Icon sources must contain visible artwork")
    crop = (min(b[0] for b in bounds), min(b[1] for b in bounds),
            max(b[2] for b in bounds), max(b[3] for b in bounds))
    scale = SIZE * 0.78 / max(crop[2] - crop[0], crop[3] - crop[1])
    dimensions = (round((crop[2] - crop[0]) * scale),
                  round((crop[3] - crop[1]) * scale))
    tiles = {}
    for theme, source in sources.items():
        foreground = source.crop(crop).resize(dimensions, Image.Resampling.LANCZOS)
        tile = Image.new("RGBA", (SIZE, SIZE))
        tile.alpha_composite(foreground, ((SIZE - dimensions[0]) // 2,
                                          (SIZE - dimensions[1]) // 2))
        tiles[theme] = tile
    return tiles


def native_icon(tile: Image.Image) -> Image.Image:
    mask = Image.new("L", (SIZE, SIZE))
    ImageDraw.Draw(mask).rounded_rectangle(
        (TILE_INSET, TILE_INSET, SIZE - TILE_INSET - 1, SIZE - TILE_INSET - 1),
        radius=184, fill=255,
    )
    surface = Image.new("RGBA", (SIZE, SIZE))
    surface.paste(tile.resize((TILE_SIZE, TILE_SIZE), Image.Resampling.LANCZOS),
                  (TILE_INSET, TILE_INSET))
    surface.putalpha(mask)
    shadow_mask = Image.new("L", (SIZE, SIZE))
    shadow_mask.paste(mask, (0, 5))
    shadow_mask = shadow_mask.filter(ImageFilter.GaussianBlur(7)).point(lambda a: a // 6)
    result = Image.new("RGBA", (SIZE, SIZE))
    result.putalpha(shadow_mask)
    result.alpha_composite(surface)
    return result


def main() -> None:
    icons = ROOT / "src-tauri" / "icons"
    for theme, tile in compose_foregrounds().items():
        tile.resize((256, 256), Image.Resampling.LANCZOS).save(
            ROOT / "src" / "assets" / f"sozocraft-icon-{theme}.png"
        )
        native_tile = background(theme)
        native_tile.alpha_composite(tile)
        native = native_icon(native_tile)
        native.save(icons / ("icon.png" if theme == "light" else "icon-dark.png"))
        if theme == "light":
            for filename, size in [("32x32.png", 32), ("128x128.png", 128),
                                   ("128x128@2x.png", 256)]:
                native.resize((size, size), Image.Resampling.LANCZOS).save(icons / filename)
            native.save(icons / "icon.icns", format="ICNS")
            native.save(icons / "icon.ico", format="ICO",
                        sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])


if __name__ == "__main__":
    main()
