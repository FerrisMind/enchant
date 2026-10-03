# enchant

Magic UI-inspired interaction layer for [`iced`](https://github.com/iced-rs/iced).
`enchant-iced` ports the Magic UI component surface to native Rust: animated text
effects, glow and beam surfaces, patterned backgrounds, media frames, and a
builder-first widget API per component.

Author: FerrisMind

## Workspace layout

- `crates/enchant-iced`: the library crate — widgets, primitives, sections, and theme helpers.

`examples/` inside the crate is the demo surface: 82 runnable examples, one per
ported component or scenario.

## Requirements

- Rust 1.93 or newer (edition 2024)
- A GPU/window backend supported by `iced` 0.14 (wgpu). Examples open native windows.

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

`cargo run --example` with no name lists every example:

```powershell
cargo run --example
```

## Documentation

```powershell
cargo doc -p enchant-iced --no-deps --open
```

## Component status

Readiness is tracked at two levels, and they are not the same thing: an example
can compile and run while still not matching the Magic UI original.

### Builds and runs — all 79

Every user-facing example compiles. CI compiles all of them on each push, and
`crates/enchant-iced/tests/example_inventory.rs` fails the build if a public
component loses its example or drops out of the upstream Magic UI registry
inventory. So this level cannot silently rot.

### Verified against the Magic UI reference — 49 of 82

Matching is checked by hand: run the example, compare it against the Magic UI
original, record the verdict. The per-example ledger is maintained with the
launcher, which serves a local page with a checkbox per example:

```powershell
python scripts/example_launcher.py
```

It writes `logs/example-parity.json`, `logs/example-parity.log`, and
`logs/example-parity-problems.txt` (`logs/` is gitignored).

**Last recorded run: 2026-09-05 — 49 of 82 verified.**

Verified: `android`, `animated_beam`, `animated_beam_multiple_outputs`,
`animated_circular_progress_bar`, `animated_gradient_text`, `animated_grid_pattern`,
`animated_list_demo`, `animated_shiny_text`, `animated_subscribe_button`,
`animated_theme_toggler`, `aurora_text`, `avatar_circles`, `backlight`,
`border_beam`, `border_track`, `dia_text_reveal`, `file_tree`, `flickering_grid`,
`globe`, `glyph_matrix`, `grid_pattern`, `hexagon_pattern`, `highlighter`,
`hyper_text`, `interactive_grid_pattern`, `iphone`, `kinetic_text`,
`line_shadow_text`, `marquee`, `morphing_text`, `motion_grid`, `noise_texture`,
`orbiting_circles`, `particles`, `pixel_image`, `rainbow_button`, `safari`,
`scroll_progress`, `shimmer_button`, `shine_border`, `shiny_button`,
`smooth_cursor`, `sparkles_text`, `spinning_text`, `striped_pattern`, `terminal`,
`typing_animation`, `warp_background`, `word_rotate`.

Not verified yet: `bento_grid`, `bento_grid_vertical`, `blur_fade`,
`client_tweet_card`, `code_comparison`, `comic_text`, `confetti`, `cool_mode`,
`dock`, `dot_pattern`, `dotted_map`, `glare_hover`, `hero_video_dialog`,
`icon_cloud`, `interactive_hover_button`, `lens`, `light_rays`, `magic_card`,
`meteors`, `neon_gradient_card`, `number_ticker`, `pointer`, `progressive_blur`,
`pulsating_button`, `retro_grid`, `ripple`, `ripple_button`,
`scroll_based_velocity`, `text_3d_flip`, `text_animate`, `text_reveal`,
`tweet_card`, `video_text`.

The ledger is updated by hand, so it lags the code. Parity rework landed after the
run above for `lens`, `cool_mode`, `dock`, and `confetti` — those entries are stale
until the launcher is re-run.

Treat the "not verified yet" list as "not yet compared against the reference",
not as "known broken": they build, ship an example, and pass the test suite.

### Not part of the public surface — 3

`animated_subscribe_button`, `border_track`, and `motion_grid` are internal helper
examples. They are deliberately excluded from the public component inventory, so
they are not covered by the compatibility expectations above.

## Notes on parity

- Each port documents its upstream reference in a module-level doc comment and links
  to the corresponding Magic UI source.
- `tools/visual-verify/` holds Playwright/pixelmatch helpers used to capture and
  compare native output against Magic UI reference screenshots. They are
  development tooling and are not part of the library.

## Licensing

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Third-party components bundled with the examples (font, brand marks, reference
material) are documented in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
