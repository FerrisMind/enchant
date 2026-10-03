# enchant-iced

Magic UI-inspired premium interaction layer for `iced`.

Author: FerrisMind

## Crate role

`enchant-iced` is the library crate of the `enchant` workspace. It exports the
widgets, sections, theme helpers, and animation primitives used by the example
set in `examples/`.

Builder-first API per component: private fields, fluent validated setters,
`#[must_use]` builders, terminal `into_element` / `view`.

## Included surface

- foundations: animation, text sequencing, anchor geometry, scroll/time helpers, effect canvas, media helpers
- layout and marketing widgets: `bento-grid`, mockups, `marquee`, `number-ticker`, `text-reveal`, `terminal`
- premium interaction widgets: `magic-card`, `animated-beam`, `file-tree`, `word-rotate`, `typing-animation`, `shimmer-button`, `shiny-button`, `avatar-circles`, `blur-fade`, `animated-circular-progress-bar`
- extended effects: `warp-background`, `progressive-blur`, `neon-gradient-card`, `meteors`, `grid-pattern`, `striped-pattern`, `dot-pattern`, `animated-grid-pattern`, `orbiting-circles`, `backlight`, `line-shadow-text`, `aurora-text`, `animated-shiny-text`, `hyper-text`, `sparkles-text`, `spinning-text`, `text-animate`, `highlighter`, `animated-list`
- interactive and media widgets: `morphing-text`, `scroll-progress`, `pointer`, `smooth-cursor`, `hexagon-pattern`, `interactive-grid-pattern`, `glare-hover`, `dock`, `comic-text`, `cool-mode`, `pixel-image`, `animated-theme-toggler`, `light-rays`, `text-3d-flip`, `lens`, `globe`, `tweet-card`, `client-tweet-card`, `particles`, `retro-grid`, `flickering-grid`, `icon-cloud`, `scroll-based-velocity`, `confetti`, `video-text`, `dotted-map`

Full public re-exports are listed in `src/lib.rs`.

## Platform notes

- no DOM refs or `ResizeObserver`: anchor geometry is measured through widget `Id` plus runtime widget operations
- no CSS keyframes: motion is driven by `AnimatedValue`, `TextRunner`, `FrameDriver`, and timestamped update messages
- `magic-card` uses surface-local pointer tracking and theme-registry colors instead of DOM pointer listeners
- `tweet-card` is data-first; remote loading stays behind the optional `tweet-adapter` feature
- decorative surfaces stay generic in `iced-shadcn`; Magic-specific effects stay in `enchant-iced`

## Features

- `tweet-adapter` (off by default): enables remote tweet loading for `tweet-card`
  and `client-tweet-card` via `reqwest` + `serde`.

## Verify

```powershell
cargo check -p enchant-iced --all-targets
cargo test -p enchant-iced
cargo clippy -p enchant-iced --all-targets -- -D warnings
```

## Run an example

```powershell
cargo run --example magic_card
cargo run --example animated_beam
```

`cargo run --example` with no name lists every example. The set covers 82
components and scenarios.

## Licensing

Licensed under either of Apache License, Version 2.0 or MIT license, at your
option. See `LICENSE-APACHE` and `LICENSE-MIT`.
