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
- premium interaction widgets: `magic-card`, `animated-beam`, `file-tree`, `word-rotate`, `typing-animation`, `shimmer-button`, `shiny-button`, `avatar-circles`, `blur-fade`, `animated-circular-progress-bar`, `liquid-gooey` (`Liquid` / `LiquidItem`)
- extended effects: `warp-background`, `progressive-blur`, `neon-gradient-card`, `meteors`, `grid-pattern`, `striped-pattern`, `dot-pattern`, `animated-grid-pattern`, `orbiting-circles`, `backlight`, `line-shadow-text`, `aurora-text`, `animated-shiny-text`, `hyper-text`, `sparkles-text`, `spinning-text`, `text-animate`, `highlighter`, `animated-list`, `voice-glow` (sound-reactive `VoiceBeam` glow, optional `microphone` feature)
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
cargo run --example gooey
```

`cargo run --example` with no name lists every example. The set covers 82
components and scenarios.

## Demo

Captured window demos for each example. Click a name to jump:

[android](#android) · [animated_beam](#animated_beam) · [animated_beam_multiple_outputs](#animated_beam_multiple_outputs) · [animated_circular_progress_bar](#animated_circular_progress_bar) · [animated_gradient_text](#animated_gradient_text) · [animated_grid_pattern](#animated_grid_pattern)
[animated_list_demo](#animated_list_demo) · [animated_shiny_text](#animated_shiny_text) · [animated_subscribe_button](#animated_subscribe_button) · [aurora_text](#aurora_text) · [backlight](#backlight) · [bento_grid](#bento_grid)
[bento_grid_vertical](#bento_grid_vertical) · [blur_fade](#blur_fade) · [border_beam](#border_beam) · [border_track](#border_track) · [bot_avatar](#bot_avatar) · [client_tweet_card](#client_tweet_card)
[code_comparison](#code_comparison) · [comic_text](#comic_text) · [confetti](#confetti) · [cool_mode](#cool_mode) · [dia_text_reveal](#dia_text_reveal) · [dock](#dock)
[dot_pattern](#dot_pattern) · [dotted_map](#dotted_map) · [file_tree](#file_tree) · [flickering_grid](#flickering_grid) · [glare_hover](#glare_hover) · [globe](#globe)
[glyph_matrix](#glyph_matrix) · [gooey](#gooey) · [grid_pattern](#grid_pattern) · [hero_video_dialog](#hero_video_dialog) · [hexagon_pattern](#hexagon_pattern) · [highlighter](#highlighter)
[hyper_text](#hyper_text) · [icon_cloud](#icon_cloud) · [interactive_grid_pattern](#interactive_grid_pattern) · [interactive_hover_button](#interactive_hover_button) · [iphone](#iphone) · [kinetic_text](#kinetic_text)
[lens](#lens) · [light_rays](#light_rays) · [line_shadow_text](#line_shadow_text) · [magic_card](#magic_card) · [marquee](#marquee) · [metal_fx](#metal_fx)
[meteors](#meteors) · [morphing_text](#morphing_text) · [motion_grid](#motion_grid) · [neon_gradient_card](#neon_gradient_card) · [noise_texture](#noise_texture) · [number_ticker](#number_ticker)
[orbiting_circles](#orbiting_circles) · [particles](#particles) · [pixel_image](#pixel_image) · [pointer](#pointer) · [progressive_blur](#progressive_blur) · [pulsating_button](#pulsating_button)
[rainbow_button](#rainbow_button) · [retro_grid](#retro_grid) · [ripple](#ripple) · [ripple_button](#ripple_button) · [safari](#safari) · [scroll_based_velocity](#scroll_based_velocity)
[scroll_progress](#scroll_progress) · [shimmer_button](#shimmer_button) · [shine_border](#shine_border) · [shiny_button](#shiny_button) · [smooth_cursor](#smooth_cursor) · [sparkles_text](#sparkles_text)
[spinning_text](#spinning_text) · [striped_pattern](#striped_pattern) · [terminal](#terminal) · [text_3d_flip](#text_3d_flip) · [text_animate](#text_animate) · [text_reveal](#text_reveal)
[thinking_orb](#thinking_orb) · [tweet_card](#tweet_card) · [typing_animation](#typing_animation) · [video_text](#video_text) · [voice_glow](#voice_glow) · [warp_background](#warp_background)
[word_rotate](#word_rotate)

### android

![android](docs/demos/android.png)

```powershell
cargo run --example android
```

### animated_beam

![animated_beam](docs/demos/animated_beam.gif)

```powershell
cargo run --example animated_beam
```

### animated_beam_multiple_outputs

![animated_beam_multiple_outputs](docs/demos/animated_beam_multiple_outputs.gif)

```powershell
cargo run --example animated_beam_multiple_outputs
```

### animated_circular_progress_bar

![animated_circular_progress_bar](docs/demos/animated_circular_progress_bar.gif)

```powershell
cargo run --example animated_circular_progress_bar
```

### animated_gradient_text

![animated_gradient_text](docs/demos/animated_gradient_text.gif)

```powershell
cargo run --example animated_gradient_text
```

### animated_grid_pattern

![animated_grid_pattern](docs/demos/animated_grid_pattern.gif)

```powershell
cargo run --example animated_grid_pattern
```

### animated_list_demo

![animated_list_demo](docs/demos/animated_list_demo.gif)

```powershell
cargo run --example animated_list_demo
```

### animated_shiny_text

![animated_shiny_text](docs/demos/animated_shiny_text.gif)

```powershell
cargo run --example animated_shiny_text
```

### animated_subscribe_button

![animated_subscribe_button](docs/demos/animated_subscribe_button.gif)

```powershell
cargo run --example animated_subscribe_button
```

### aurora_text

![aurora_text](docs/demos/aurora_text.gif)

```powershell
cargo run --example aurora_text
```

### backlight

![backlight](docs/demos/backlight.gif)

```powershell
cargo run --example backlight
```

### bento_grid

![bento_grid](docs/demos/bento_grid.gif)

```powershell
cargo run --example bento_grid
```

### bento_grid_vertical

![bento_grid_vertical](docs/demos/bento_grid_vertical.gif)

```powershell
cargo run --example bento_grid_vertical
```

### blur_fade

![blur_fade](docs/demos/blur_fade.gif)

```powershell
cargo run --example blur_fade
```

### border_beam

![border_beam](docs/demos/border_beam.gif)

```powershell
cargo run --example border_beam
```

### border_track

![border_track](docs/demos/border_track.gif)

```powershell
cargo run --example border_track
```

### bot_avatar

![bot_avatar](docs/demos/bot_avatar.gif)

```powershell
cargo run --example bot_avatar
```

### client_tweet_card

![client_tweet_card](docs/demos/client_tweet_card.gif)

```powershell
cargo run --example client_tweet_card
```

### code_comparison

![code_comparison](docs/demos/code_comparison.gif)

```powershell
cargo run --example code_comparison
```

### comic_text

![comic_text](docs/demos/comic_text.gif)

```powershell
cargo run --example comic_text
```

### confetti

![confetti](docs/demos/confetti.gif)

```powershell
cargo run --example confetti
```

### cool_mode

![cool_mode](docs/demos/cool_mode.gif)

```powershell
cargo run --example cool_mode
```

### dia_text_reveal

![dia_text_reveal](docs/demos/dia_text_reveal.gif)

```powershell
cargo run --example dia_text_reveal
```

### dock

![dock](docs/demos/dock.gif)

```powershell
cargo run --example dock
```

### dot_pattern

![dot_pattern](docs/demos/dot_pattern.gif)

```powershell
cargo run --example dot_pattern
```

### dotted_map

![dotted_map](docs/demos/dotted_map.png)

```powershell
cargo run --example dotted_map
```

### file_tree

![file_tree](docs/demos/file_tree.gif)

```powershell
cargo run --example file_tree
```

### flickering_grid

![flickering_grid](docs/demos/flickering_grid.gif)

```powershell
cargo run --example flickering_grid
```

### glare_hover

![glare_hover](docs/demos/glare_hover.gif)

```powershell
cargo run --example glare_hover
```

### globe

![globe](docs/demos/globe.gif)

```powershell
cargo run --example globe
```

### glyph_matrix

![glyph_matrix](docs/demos/glyph_matrix.gif)

```powershell
cargo run --example glyph_matrix
```

### gooey

![gooey](docs/demos/gooey.gif)

```powershell
cargo run --example gooey
```

### grid_pattern

![grid_pattern](docs/demos/grid_pattern.png)

```powershell
cargo run --example grid_pattern
```

### hero_video_dialog

![hero_video_dialog](docs/demos/hero_video_dialog.gif)

```powershell
cargo run --example hero_video_dialog
```

### hexagon_pattern

![hexagon_pattern](docs/demos/hexagon_pattern.png)

```powershell
cargo run --example hexagon_pattern
```

### highlighter

![highlighter](docs/demos/highlighter.gif)

```powershell
cargo run --example highlighter
```

### hyper_text

![hyper_text](docs/demos/hyper_text.gif)

```powershell
cargo run --example hyper_text
```

### icon_cloud

![icon_cloud](docs/demos/icon_cloud.gif)

```powershell
cargo run --example icon_cloud
```

### interactive_grid_pattern

![interactive_grid_pattern](docs/demos/interactive_grid_pattern.gif)

```powershell
cargo run --example interactive_grid_pattern
```

### interactive_hover_button

![interactive_hover_button](docs/demos/interactive_hover_button.gif)

```powershell
cargo run --example interactive_hover_button
```

### iphone

![iphone](docs/demos/iphone.gif)

```powershell
cargo run --example iphone
```

### kinetic_text

![kinetic_text](docs/demos/kinetic_text.gif)

```powershell
cargo run --example kinetic_text
```

### lens

![lens](docs/demos/lens.gif)

```powershell
cargo run --example lens
```

### light_rays

![light_rays](docs/demos/light_rays.gif)

```powershell
cargo run --example light_rays
```

### line_shadow_text

![line_shadow_text](docs/demos/line_shadow_text.gif)

```powershell
cargo run --example line_shadow_text
```

### magic_card

![magic_card](docs/demos/magic_card.gif)

```powershell
cargo run --example magic_card
```

### marquee

![marquee](docs/demos/marquee.gif)

```powershell
cargo run --example marquee
```

### metal_fx

![metal_fx](docs/demos/metal_fx.gif)

```powershell
cargo run --example metal_fx
```

### meteors

![meteors](docs/demos/meteors.gif)

```powershell
cargo run --example meteors
```

### morphing_text

![morphing_text](docs/demos/morphing_text.gif)

```powershell
cargo run --example morphing_text
```

### motion_grid

![motion_grid](docs/demos/motion_grid.gif)

```powershell
cargo run --example motion_grid
```

### neon_gradient_card

![neon_gradient_card](docs/demos/neon_gradient_card.gif)

```powershell
cargo run --example neon_gradient_card
```

### noise_texture

![noise_texture](docs/demos/noise_texture.gif)

```powershell
cargo run --example noise_texture
```

### number_ticker

![number_ticker](docs/demos/number_ticker.gif)

```powershell
cargo run --example number_ticker
```

### orbiting_circles

![orbiting_circles](docs/demos/orbiting_circles.gif)

```powershell
cargo run --example orbiting_circles
```

### particles

![particles](docs/demos/particles.gif)

```powershell
cargo run --example particles
```

### pixel_image

![pixel_image](docs/demos/pixel_image.gif)

```powershell
cargo run --example pixel_image
```

### pointer

![pointer](docs/demos/pointer.gif)

```powershell
cargo run --example pointer
```

### progressive_blur

![progressive_blur](docs/demos/progressive_blur.gif)

```powershell
cargo run --example progressive_blur
```

### pulsating_button

![pulsating_button](docs/demos/pulsating_button.gif)

```powershell
cargo run --example pulsating_button
```

### rainbow_button

![rainbow_button](docs/demos/rainbow_button.gif)

```powershell
cargo run --example rainbow_button
```

### retro_grid

![retro_grid](docs/demos/retro_grid.gif)

```powershell
cargo run --example retro_grid
```

### ripple

![ripple](docs/demos/ripple.gif)

```powershell
cargo run --example ripple
```

### ripple_button

![ripple_button](docs/demos/ripple_button.gif)

```powershell
cargo run --example ripple_button
```

### safari

![safari](docs/demos/safari.gif)

```powershell
cargo run --example safari
```

### scroll_based_velocity

![scroll_based_velocity](docs/demos/scroll_based_velocity.gif)

```powershell
cargo run --example scroll_based_velocity
```

### scroll_progress

![scroll_progress](docs/demos/scroll_progress.gif)

```powershell
cargo run --example scroll_progress
```

### shimmer_button

![shimmer_button](docs/demos/shimmer_button.gif)

```powershell
cargo run --example shimmer_button
```

### shine_border

![shine_border](docs/demos/shine_border.gif)

```powershell
cargo run --example shine_border
```

### shiny_button

![shiny_button](docs/demos/shiny_button.gif)

```powershell
cargo run --example shiny_button
```

### smooth_cursor

![smooth_cursor](docs/demos/smooth_cursor.gif)

```powershell
cargo run --example smooth_cursor
```

### sparkles_text

![sparkles_text](docs/demos/sparkles_text.gif)

```powershell
cargo run --example sparkles_text
```

### spinning_text

![spinning_text](docs/demos/spinning_text.gif)

```powershell
cargo run --example spinning_text
```

### striped_pattern

![striped_pattern](docs/demos/striped_pattern.gif)

```powershell
cargo run --example striped_pattern
```

### terminal

![terminal](docs/demos/terminal.gif)

```powershell
cargo run --example terminal
```

### text_3d_flip

![text_3d_flip](docs/demos/text_3d_flip.gif)

```powershell
cargo run --example text_3d_flip
```

### text_animate

![text_animate](docs/demos/text_animate.gif)

```powershell
cargo run --example text_animate
```

### text_reveal

![text_reveal](docs/demos/text_reveal.gif)

```powershell
cargo run --example text_reveal
```

### thinking_orb

![thinking_orb](docs/demos/thinking_orb.gif)

```powershell
cargo run --example thinking_orb
```

### tweet_card

![tweet_card](docs/demos/tweet_card.gif)

```powershell
cargo run --example tweet_card
```

### typing_animation

![typing_animation](docs/demos/typing_animation.gif)

```powershell
cargo run --example typing_animation
```

### video_text

![video_text](docs/demos/video_text.gif)

```powershell
cargo run --example video_text
```

### voice_glow

![voice_glow](docs/demos/voice_glow.gif)

```powershell
cargo run --example voice_glow
```

### warp_background

![warp_background](docs/demos/warp_background.gif)

```powershell
cargo run --example warp_background
```

### word_rotate

![word_rotate](docs/demos/word_rotate.gif)

```powershell
cargo run --example word_rotate
```

## Component status

Two levels, tracked separately:

- **Builds and runs** — every user-facing example compiles, and CI enforces that
  each public component keeps its example and stays in the upstream Magic UI
  registry inventory (`tests/example_inventory.rs`).
- **Verified against the Magic UI reference** — a per-example verdict recorded by
  hand with `python scripts/example_launcher.py`. The ledger lives in `logs/`
  (gitignored); see the workspace README for the current verified and not-yet
  verified lists.

Examples that are internal helpers, not part of the Magic UI public surface,
are listed in `tests/example_inventory.rs` (`INTERNAL_HELPER_EXAMPLES`); the
liquid-gooey gallery (`gooey`) and the effect benchmarks live there because
their upstream is libraries.dev rather than Magic UI.

## Licensing

Licensed under either of Apache License, Version 2.0 or MIT license, at your
option. See `LICENSE-APACHE` and `LICENSE-MIT`.
