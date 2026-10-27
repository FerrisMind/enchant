# enchant-iced

<p align="center">
  <strong>Premium interaction layer for iced</strong>
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

`enchant-iced` ports the [Magic UI](https://magicui.design) component surface and [Libraries.dev](https://libraries.dev) effect libraries to native Rust on [iced](https://github.com/iced-rs/iced): animated text, glow and beam surfaces, patterned backgrounds, media frames, and a builder-first widget API per component.

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

[android](#android) · [animated_beam](#animated_beam) · [animated_beam_multiple_outputs](#animated_beam_multiple_outputs) · [animated_circular_progress_bar](#animated_circular_progress_bar) · [animated_gradient_text](#animated_gradient_text) · [animated_grid_pattern](#animated_grid_pattern)
[animated_list_demo](#animated_list_demo) · [animated_shiny_text](#animated_shiny_text) · [animated_subscribe_button](#animated_subscribe_button) · [aurora_text](#aurora_text) · [backlight](#backlight) · [bento_grid](#bento_grid)
[blur_fade](#blur_fade) · [border_beam](#border_beam) · [border_track](#border_track) · [bot_avatar](#bot_avatar) · [client_tweet_card](#client_tweet_card) · [code_comparison](#code_comparison)
[comic_text](#comic_text) · [confetti](#confetti) · [cool_mode](#cool_mode) · [dia_text_reveal](#dia_text_reveal) · [dock](#dock) · [dot_pattern](#dot_pattern)
[dotted_map](#dotted_map) · [file_tree](#file_tree) · [flickering_grid](#flickering_grid) · [glare_hover](#glare_hover) · [globe](#globe) · [glyph_matrix](#glyph_matrix)
[gooey](#gooey) · [grid_pattern](#grid_pattern) · [hero_video_dialog](#hero_video_dialog) · [hexagon_pattern](#hexagon_pattern) · [highlighter](#highlighter) · [hyper_text](#hyper_text)
[icon_cloud](#icon_cloud) · [interactive_grid_pattern](#interactive_grid_pattern) · [interactive_hover_button](#interactive_hover_button) · [iphone](#iphone) · [kinetic_text](#kinetic_text) · [lens](#lens)
[light_rays](#light_rays) · [line_shadow_text](#line_shadow_text) · [magic_card](#magic_card) · [marquee](#marquee) · [metal_fx](#metal_fx) · [meteors](#meteors)
[morphing_text](#morphing_text) · [motion_grid](#motion_grid) · [neon_gradient_card](#neon_gradient_card) · [noise_texture](#noise_texture) · [number_ticker](#number_ticker) · [orbiting_circles](#orbiting_circles)
[particles](#particles) · [pixel_image](#pixel_image) · [pointer](#pointer) · [progressive_blur](#progressive_blur) · [pulsating_button](#pulsating_button) · [rainbow_button](#rainbow_button)
[retro_grid](#retro_grid) · [ripple](#ripple) · [ripple_button](#ripple_button) · [safari](#safari) · [scroll_based_velocity](#scroll_based_velocity) · [scroll_progress](#scroll_progress)
[shimmer_button](#shimmer_button) · [shine_border](#shine_border) · [shiny_button](#shiny_button) · [smooth_cursor](#smooth_cursor) · [sparkles_text](#sparkles_text) · [spinning_text](#spinning_text)
[striped_pattern](#striped_pattern) · [terminal](#terminal) · [text_3d_flip](#text_3d_flip) · [text_animate](#text_animate) · [text_reveal](#text_reveal) · [thinking_orb](#thinking_orb)
[tweet_card](#tweet_card) · [typing_animation](#typing_animation) · [video_text](#video_text) · [voice_glow](#voice_glow) · [warp_background](#warp_background) · [word_rotate](#word_rotate)

### android

<p align="center">
  <picture>
    <img alt="android" src="docs/demos/android.png" width="720" />
  </picture>
</p>

### animated_beam

<p align="center">
  <picture>
    <img alt="animated_beam" src="docs/demos/animated_beam.gif" width="720" />
  </picture>
</p>

### animated_beam_multiple_outputs

<p align="center">
  <picture>
    <img alt="animated_beam_multiple_outputs" src="docs/demos/animated_beam_multiple_outputs.gif" width="720" />
  </picture>
</p>

### animated_circular_progress_bar

<p align="center">
  <picture>
    <img alt="animated_circular_progress_bar" src="docs/demos/animated_circular_progress_bar.gif" width="720" />
  </picture>
</p>

### animated_gradient_text

<p align="center">
  <picture>
    <img alt="animated_gradient_text" src="docs/demos/animated_gradient_text.gif" width="720" />
  </picture>
</p>

### animated_grid_pattern

<p align="center">
  <picture>
    <img alt="animated_grid_pattern" src="docs/demos/animated_grid_pattern.gif" width="720" />
  </picture>
</p>

### animated_list_demo

<p align="center">
  <picture>
    <img alt="animated_list_demo" src="docs/demos/animated_list_demo.gif" width="720" />
  </picture>
</p>

### animated_shiny_text

<p align="center">
  <picture>
    <img alt="animated_shiny_text" src="docs/demos/animated_shiny_text.gif" width="720" />
  </picture>
</p>

### animated_subscribe_button

<p align="center">
  <picture>
    <img alt="animated_subscribe_button" src="docs/demos/animated_subscribe_button.gif" width="720" />
  </picture>
</p>

### aurora_text

<p align="center">
  <picture>
    <img alt="aurora_text" src="docs/demos/aurora_text.gif" width="720" />
  </picture>
</p>

### backlight

<p align="center">
  <picture>
    <img alt="backlight" src="docs/demos/backlight.gif" width="720" />
  </picture>
</p>

### bento_grid

<p align="center">
  <picture>
    <img alt="bento_grid" src="docs/demos/bento_grid.gif" width="720" />
  </picture>
</p>

### blur_fade

<p align="center">
  <picture>
    <img alt="blur_fade" src="docs/demos/blur_fade.gif" width="720" />
  </picture>
</p>

### border_beam

<p align="center">
  <picture>
    <img alt="border_beam" src="docs/demos/border_beam.gif" width="720" />
  </picture>
</p>

### border_track

<p align="center">
  <picture>
    <img alt="border_track" src="docs/demos/border_track.gif" width="720" />
  </picture>
</p>

### bot_avatar

<p align="center">
  <picture>
    <img alt="bot_avatar" src="docs/demos/bot_avatar.gif" width="720" />
  </picture>
</p>

### client_tweet_card

<p align="center">
  <picture>
    <img alt="client_tweet_card" src="docs/demos/client_tweet_card.gif" width="720" />
  </picture>
</p>

### code_comparison

<p align="center">
  <picture>
    <img alt="code_comparison" src="docs/demos/code_comparison.gif" width="720" />
  </picture>
</p>

### comic_text

<p align="center">
  <picture>
    <img alt="comic_text" src="docs/demos/comic_text.gif" width="720" />
  </picture>
</p>

### confetti

<p align="center">
  <picture>
    <img alt="confetti" src="docs/demos/confetti.gif" width="720" />
  </picture>
</p>

### cool_mode

<p align="center">
  <picture>
    <img alt="cool_mode" src="docs/demos/cool_mode.gif" width="720" />
  </picture>
</p>

### dia_text_reveal

<p align="center">
  <picture>
    <img alt="dia_text_reveal" src="docs/demos/dia_text_reveal.gif" width="720" />
  </picture>
</p>

### dock

<p align="center">
  <picture>
    <img alt="dock" src="docs/demos/dock.gif" width="720" />
  </picture>
</p>

### dot_pattern

<p align="center">
  <picture>
    <img alt="dot_pattern" src="docs/demos/dot_pattern.gif" width="720" />
  </picture>
</p>

### dotted_map

<p align="center">
  <picture>
    <img alt="dotted_map" src="docs/demos/dotted_map.png" width="720" />
  </picture>
</p>

### file_tree

<p align="center">
  <picture>
    <img alt="file_tree" src="docs/demos/file_tree.gif" width="720" />
  </picture>
</p>

### flickering_grid

<p align="center">
  <picture>
    <img alt="flickering_grid" src="docs/demos/flickering_grid.gif" width="720" />
  </picture>
</p>

### glare_hover

<p align="center">
  <picture>
    <img alt="glare_hover" src="docs/demos/glare_hover.gif" width="720" />
  </picture>
</p>

### globe

<p align="center">
  <picture>
    <img alt="globe" src="docs/demos/globe.gif" width="720" />
  </picture>
</p>

### glyph_matrix

<p align="center">
  <picture>
    <img alt="glyph_matrix" src="docs/demos/glyph_matrix.gif" width="720" />
  </picture>
</p>

### gooey

<p align="center">
  <picture>
    <img alt="gooey" src="docs/demos/gooey.gif" width="720" />
  </picture>
</p>

### grid_pattern

<p align="center">
  <picture>
    <img alt="grid_pattern" src="docs/demos/grid_pattern.png" width="720" />
  </picture>
</p>

### hero_video_dialog

<p align="center">
  <picture>
    <img alt="hero_video_dialog" src="docs/demos/hero_video_dialog.gif" width="720" />
  </picture>
</p>

### hexagon_pattern

<p align="center">
  <picture>
    <img alt="hexagon_pattern" src="docs/demos/hexagon_pattern.png" width="720" />
  </picture>
</p>

### highlighter

<p align="center">
  <picture>
    <img alt="highlighter" src="docs/demos/highlighter.gif" width="720" />
  </picture>
</p>

### hyper_text

<p align="center">
  <picture>
    <img alt="hyper_text" src="docs/demos/hyper_text.gif" width="720" />
  </picture>
</p>

### icon_cloud

<p align="center">
  <picture>
    <img alt="icon_cloud" src="docs/demos/icon_cloud.gif" width="720" />
  </picture>
</p>

### interactive_grid_pattern

<p align="center">
  <picture>
    <img alt="interactive_grid_pattern" src="docs/demos/interactive_grid_pattern.gif" width="720" />
  </picture>
</p>

### interactive_hover_button

<p align="center">
  <picture>
    <img alt="interactive_hover_button" src="docs/demos/interactive_hover_button.gif" width="720" />
  </picture>
</p>

### iphone

<p align="center">
  <picture>
    <img alt="iphone" src="docs/demos/iphone.gif" width="720" />
  </picture>
</p>

### kinetic_text

<p align="center">
  <picture>
    <img alt="kinetic_text" src="docs/demos/kinetic_text.gif" width="720" />
  </picture>
</p>

### lens

<p align="center">
  <picture>
    <img alt="lens" src="docs/demos/lens.gif" width="720" />
  </picture>
</p>

### light_rays

<p align="center">
  <picture>
    <img alt="light_rays" src="docs/demos/light_rays.gif" width="720" />
  </picture>
</p>

### line_shadow_text

<p align="center">
  <picture>
    <img alt="line_shadow_text" src="docs/demos/line_shadow_text.gif" width="720" />
  </picture>
</p>

### magic_card

<p align="center">
  <picture>
    <img alt="magic_card" src="docs/demos/magic_card.gif" width="720" />
  </picture>
</p>

### marquee

<p align="center">
  <picture>
    <img alt="marquee" src="docs/demos/marquee.gif" width="720" />
  </picture>
</p>

### metal_fx

<p align="center">
  <picture>
    <img alt="metal_fx" src="docs/demos/metal_fx.gif" width="720" />
  </picture>
</p>

### meteors

<p align="center">
  <picture>
    <img alt="meteors" src="docs/demos/meteors.gif" width="720" />
  </picture>
</p>

### morphing_text

<p align="center">
  <picture>
    <img alt="morphing_text" src="docs/demos/morphing_text.gif" width="720" />
  </picture>
</p>

### motion_grid

<p align="center">
  <picture>
    <img alt="motion_grid" src="docs/demos/motion_grid.gif" width="720" />
  </picture>
</p>

### neon_gradient_card

<p align="center">
  <picture>
    <img alt="neon_gradient_card" src="docs/demos/neon_gradient_card.gif" width="720" />
  </picture>
</p>

### noise_texture

<p align="center">
  <picture>
    <img alt="noise_texture" src="docs/demos/noise_texture.gif" width="720" />
  </picture>
</p>

### number_ticker

<p align="center">
  <picture>
    <img alt="number_ticker" src="docs/demos/number_ticker.gif" width="720" />
  </picture>
</p>

### orbiting_circles

<p align="center">
  <picture>
    <img alt="orbiting_circles" src="docs/demos/orbiting_circles.gif" width="720" />
  </picture>
</p>

### particles

<p align="center">
  <picture>
    <img alt="particles" src="docs/demos/particles.gif" width="720" />
  </picture>
</p>

### pixel_image

<p align="center">
  <picture>
    <img alt="pixel_image" src="docs/demos/pixel_image.gif" width="720" />
  </picture>
</p>

### pointer

<p align="center">
  <picture>
    <img alt="pointer" src="docs/demos/pointer.gif" width="720" />
  </picture>
</p>

### progressive_blur

<p align="center">
  <picture>
    <img alt="progressive_blur" src="docs/demos/progressive_blur.gif" width="720" />
  </picture>
</p>

### pulsating_button

<p align="center">
  <picture>
    <img alt="pulsating_button" src="docs/demos/pulsating_button.gif" width="720" />
  </picture>
</p>

### rainbow_button

<p align="center">
  <picture>
    <img alt="rainbow_button" src="docs/demos/rainbow_button.gif" width="720" />
  </picture>
</p>

### retro_grid

<p align="center">
  <picture>
    <img alt="retro_grid" src="docs/demos/retro_grid.gif" width="720" />
  </picture>
</p>

### ripple

<p align="center">
  <picture>
    <img alt="ripple" src="docs/demos/ripple.gif" width="720" />
  </picture>
</p>

### ripple_button

<p align="center">
  <picture>
    <img alt="ripple_button" src="docs/demos/ripple_button.gif" width="720" />
  </picture>
</p>

### safari

<p align="center">
  <picture>
    <img alt="safari" src="docs/demos/safari.gif" width="720" />
  </picture>
</p>

### scroll_based_velocity

<p align="center">
  <picture>
    <img alt="scroll_based_velocity" src="docs/demos/scroll_based_velocity.gif" width="720" />
  </picture>
</p>

### scroll_progress

<p align="center">
  <picture>
    <img alt="scroll_progress" src="docs/demos/scroll_progress.gif" width="720" />
  </picture>
</p>

### shimmer_button

<p align="center">
  <picture>
    <img alt="shimmer_button" src="docs/demos/shimmer_button.gif" width="720" />
  </picture>
</p>

### shine_border

<p align="center">
  <picture>
    <img alt="shine_border" src="docs/demos/shine_border.gif" width="720" />
  </picture>
</p>

### shiny_button

<p align="center">
  <picture>
    <img alt="shiny_button" src="docs/demos/shiny_button.gif" width="720" />
  </picture>
</p>

### smooth_cursor

<p align="center">
  <picture>
    <img alt="smooth_cursor" src="docs/demos/smooth_cursor.gif" width="720" />
  </picture>
</p>

### sparkles_text

<p align="center">
  <picture>
    <img alt="sparkles_text" src="docs/demos/sparkles_text.gif" width="720" />
  </picture>
</p>

### spinning_text

<p align="center">
  <picture>
    <img alt="spinning_text" src="docs/demos/spinning_text.gif" width="720" />
  </picture>
</p>

### striped_pattern

<p align="center">
  <picture>
    <img alt="striped_pattern" src="docs/demos/striped_pattern.gif" width="720" />
  </picture>
</p>

### terminal

<p align="center">
  <picture>
    <img alt="terminal" src="docs/demos/terminal.gif" width="720" />
  </picture>
</p>

### text_3d_flip

<p align="center">
  <picture>
    <img alt="text_3d_flip" src="docs/demos/text_3d_flip.gif" width="720" />
  </picture>
</p>

### text_animate

<p align="center">
  <picture>
    <img alt="text_animate" src="docs/demos/text_animate.gif" width="720" />
  </picture>
</p>

### text_reveal

<p align="center">
  <picture>
    <img alt="text_reveal" src="docs/demos/text_reveal.gif" width="720" />
  </picture>
</p>

### thinking_orb

<p align="center">
  <picture>
    <img alt="thinking_orb" src="docs/demos/thinking_orb.gif" width="720" />
  </picture>
</p>

### tweet_card

<p align="center">
  <picture>
    <img alt="tweet_card" src="docs/demos/tweet_card.gif" width="720" />
  </picture>
</p>

### typing_animation

<p align="center">
  <picture>
    <img alt="typing_animation" src="docs/demos/typing_animation.gif" width="720" />
  </picture>
</p>

### video_text

<p align="center">
  <picture>
    <img alt="video_text" src="docs/demos/video_text.gif" width="720" />
  </picture>
</p>

### voice_glow

<p align="center">
  <picture>
    <img alt="voice_glow" src="docs/demos/voice_glow.gif" width="720" />
  </picture>
</p>

### warp_background

<p align="center">
  <picture>
    <img alt="warp_background" src="docs/demos/warp_background.gif" width="720" />
  </picture>
</p>

### word_rotate

<p align="center">
  <picture>
    <img alt="word_rotate" src="docs/demos/word_rotate.gif" width="720" />
  </picture>
</p>

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
- **Verified against the Magic UI / [Libraries.dev](https://libraries.dev) references** — hand-checked with `python scripts/example_launcher.py`; ledger lives in `logs/` (gitignored)

Internal helpers (not Magic UI / Libraries.dev public surface) are listed in `tests/example_inventory.rs` (`INTERNAL_HELPER_EXAMPLES`).

## License

Dual-licensed under [MIT](https://opensource.org/licenses/MIT) OR [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0).

Third-party attribution (Magic UI, Libraries.dev, fonts, demo assets): see [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).

---

**Inspired by** [Magic UI](https://magicui.design) · [Libraries.dev](https://libraries.dev) ([source](https://github.com/Jakubantalik/Libraries.dev)) · **UI kit** [iced-shadcn](https://github.com/FerrisMind/shadcn-rs) · **Styling** [twill](https://github.com/FerrisMind/twill) · **Framework** [iced](https://github.com/iced-rs/iced)
