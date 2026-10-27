#!/usr/bin/env python3
"""Regenerate enchant / enchant-iced READMEs in shadcn-rs / twill style."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEMOS_DIR = ROOT / "crates" / "enchant-iced" / "docs" / "demos"
# Relative to crates/enchant-iced/README.md (same pattern as egui-shadcn examples).
CRATE_DEMO_DIR = "docs/demos"
# Relative to workspace README.md
WORKSPACE_DEMO_DIR = "crates/enchant-iced/docs/demos"


def collect_demos() -> list[tuple[str, str]]:
    by_base: dict[str, list[Path]] = {}
    for path in DEMOS_DIR.iterdir():
        if path.suffix.lower() not in {".png", ".gif"}:
            continue
        by_base.setdefault(path.stem, []).append(path)

    demos: list[tuple[str, str]] = []
    for name in sorted(by_base):
        files = by_base[name]
        gif = next((f for f in files if f.suffix.lower() == ".gif"), None)
        png = next((f for f in files if f.suffix.lower() == ".png"), None)
        prefer = None
        if gif is not None and gif.stat().st_size >= 2000:
            prefer = gif
        elif png is not None and png.stat().st_size >= 2000:
            prefer = png
        if prefer is None:
            continue
        demos.append((name, prefer.name))
    return demos


def main() -> None:
    demos = collect_demos()
    links = [f"[{name}](#{name})" for name, _ in demos]
    toc_lines = [
        " · ".join(links[i : i + 6]) for i in range(0, len(links), 6)
    ]

    demo_parts: list[str] = []
    for name, file in demos:
        rel = f"{CRATE_DEMO_DIR}/{file}"
        demo_parts.append(
            "\n".join(
                [
                    f"### {name}",
                    "",
                    '<p align="center">',
                    "  <picture>",
                    f'    <img alt="{name}" src="{rel}" width="720" />',
                    "  </picture>",
                    "</p>",
                    "",
                ]
            )
        )

    crate = f"""# enchant-iced

<p align="center">
  <strong>Magic UI-inspired premium interaction layer for iced</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/enchant-iced"><img alt="crates.io" src="https://img.shields.io/crates/v/enchant-iced?label=enchant-iced"></a>
  <a href="https://docs.rs/enchant-iced"><img alt="docs.rs" src="https://img.shields.io/docsrs/enchant-iced?label=docs.rs"></a>
  <a href="https://opensource.org/licenses/MIT"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg"></a>
</p>

---

> [!WARNING]
> API STABILITY NOTICE: `enchant-iced` API is currently unstable and may change between versions, including breaking changes.
> Always pin exact crate versions and review release notes before upgrading.

## Overview

`enchant-iced` ports the [Magic UI](https://magicui.design) component surface to native Rust on [iced](https://github.com/iced-rs/iced): animated text, glow and beam surfaces, patterned backgrounds, media frames, and a builder-first widget API per component.

Builder-first API: private fields, fluent validated setters, `#[must_use]` builders, terminal `into_element` / `view`.

Depends on [`iced-shadcn`](https://crates.io/crates/iced-shadcn) for decorative surfaces and [`twill`](https://crates.io/crates/twill) for styling tokens.

## Included surface

- foundations: animation, text sequencing, anchor geometry, scroll/time helpers, effect canvas, media helpers
- layout and marketing widgets: `bento-grid`, mockups, `marquee`, `number-ticker`, `text-reveal`, `terminal`
- premium interaction widgets: `magic-card`, `animated-beam`, `file-tree`, `word-rotate`, `typing-animation`, `shimmer-button`, `shiny-button`, `avatar-circles`, `blur-fade`, `animated-circular-progress-bar`, `liquid-gooey`
- extended effects: `warp-background`, `progressive-blur`, `neon-gradient-card`, `meteors`, `grid-pattern`, `orbiting-circles`, `aurora-text`, `particles`, `confetti`, `voice-glow`, and more

Full public re-exports are listed in `src/lib.rs`.

## Features

- `tweet-adapter` (off by default): remote tweet loading for `tweet-card` / `client-tweet-card` via `reqwest` + `serde`
- `microphone` (off by default): sound-reactive `voice-glow` via `cpal`

## Installation

```toml
[dependencies]
enchant-iced = "0.29"
iced = "0.14"
iced-shadcn = "0.5"
```

MSRV: Rust `1.93`.

## Run an example

```powershell
cargo run --example magic_card
cargo run --example animated_beam
cargo run --example gooey
```

`cargo run --example` with no name lists every example.

## Demos

Window captures for each example. Click a name to jump:

{chr(10).join(toc_lines)}

{chr(10).join(demo_parts)}
## Platform notes

- no DOM refs or `ResizeObserver`: anchor geometry is measured through widget `Id` plus runtime widget operations
- no CSS keyframes: motion is driven by `AnimatedValue`, `TextRunner`, `FrameDriver`, and timestamped update messages
- `magic-card` uses surface-local pointer tracking and theme-registry colors instead of DOM pointer listeners
- `tweet-card` is data-first; remote loading stays behind the optional `tweet-adapter` feature

## Verify

```powershell
cargo check -p enchant-iced --all-targets
cargo test -p enchant-iced
cargo clippy -p enchant-iced --all-targets -- -D warnings
```

## Component status

Two levels, tracked separately:

- **Builds and runs** — every user-facing example compiles; CI and `tests/example_inventory.rs` keep the inventory honest
- **Verified against the Magic UI reference** — hand-checked with `python scripts/example_launcher.py`; ledger lives in `logs/` (gitignored)

Internal helpers (not Magic UI public surface) are listed in `tests/example_inventory.rs` (`INTERNAL_HELPER_EXAMPLES`).

## License

Dual-licensed under [MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0).

---

**Inspired by** [Magic UI](https://magicui.design) · **UI kit** [iced-shadcn](https://github.com/FerrisMind/shadcn-rs) · **Styling** [twill](https://github.com/FerrisMind/twill) · **Framework** [iced](https://github.com/iced-rs/iced)
"""

    (ROOT / "crates" / "enchant-iced" / "README.md").write_text(
        crate, encoding="utf-8", newline="\n"
    )

    showcase_name = "marquee"
    showcase_file = next(
        (file for name, file in demos if name == showcase_name), demos[0][1]
    )
    if not any(name == showcase_name for name, _ in demos):
        showcase_name = demos[0][0]
    showcase_url = f"{WORKSPACE_DEMO_DIR}/{showcase_file}"

    workspace = f"""# enchant

<p align="center">
  <strong>Magic UI-inspired interaction layer for iced</strong><br>
  <i>Animated text, glow surfaces, patterned backgrounds, and builder-first widgets</i>
</p>

<p align="center">
  <a href="https://crates.io/crates/enchant-iced"><img alt="crates.io" src="https://img.shields.io/crates/v/enchant-iced?label=enchant-iced"></a>
  <a href="https://docs.rs/enchant-iced"><img alt="docs.rs" src="https://img.shields.io/docsrs/enchant-iced?label=docs.rs"></a>
  <a href="https://opensource.org/licenses/MIT"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg"></a>
</p>

<p align="center">
  <picture>
    <img alt="{showcase_name}" src="{showcase_url}" width="900" />
  </picture>
  <br>
  <sub>{showcase_name} — one of {len(demos)} captured example demos</sub>
</p>

---

> [!WARNING]
> API STABILITY NOTICE: `enchant-iced` API is currently unstable and may change between versions, including breaking changes.
> Always pin exact crate versions and review release notes before upgrading.

## Overview

`enchant` is a workspace whose public library crate is [`enchant-iced`](crates/enchant-iced): a Magic UI-inspired premium interaction layer for [`iced`](https://github.com/iced-rs/iced).

- builder-first widget API per component
- examples under `crates/enchant-iced/examples/` (one runnable demo per component or scenario)
- decorative surfaces via [`iced-shadcn`](https://github.com/FerrisMind/shadcn-rs); style tokens via [`twill`](https://github.com/FerrisMind/twill)

See the crate README for installation, feature flags, platform notes, and the full demo gallery:

**[crates/enchant-iced/README.md](crates/enchant-iced/README.md)**

## Workspace layout

- `crates/enchant-iced` — library crate (widgets, primitives, sections, theme helpers)

## Requirements

- Rust `1.93` or newer (edition 2024)
- A GPU/window backend supported by `iced` 0.14 (wgpu)

## Installation

```toml
[dependencies]
enchant-iced = "0.29"
iced = "0.14"
iced-shadcn = "0.5"
```

## Build and verify

```powershell
cargo check --workspace --all-targets
cargo test -p enchant-iced
cargo clippy -p enchant-iced --all-targets -- -D warnings
cargo fmt --all --check
```

## Run an example

```powershell
cargo run --example magic_card
cargo run --example animated_beam
cargo run --example bento_grid
```

`cargo run --example` with no name lists every example.

## Demos

Jump to the full gallery (TOC + every PNG/GIF capture):

**[Example demos →](crates/enchant-iced/README.md#demos)**

## Documentation

```powershell
cargo doc -p enchant-iced --no-deps --open
```

## Component status

Readiness is tracked at two levels:

- **Builds and runs** — CI compiles every user-facing example; `crates/enchant-iced/tests/example_inventory.rs` guards inventory drift
- **Verified against the Magic UI reference** — hand-checked via the launcher:

```powershell
python scripts/example_launcher.py
```

The ledger writes under `logs/` (gitignored).

## License

Dual-licensed under [MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0).

---

**Inspired by** [Magic UI](https://magicui.design) · **UI kit** [iced-shadcn](https://github.com/FerrisMind/shadcn-rs) · **Styling** [twill](https://github.com/FerrisMind/twill) · **Framework** [iced](https://github.com/iced-rs/iced)
"""

    (ROOT / "README.md").write_text(workspace, encoding="utf-8", newline="\n")
    print(f"wrote READMEs demos={len(demos)} showcase={showcase_name}")


if __name__ == "__main__":
    main()
