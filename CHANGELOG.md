# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `enchant_iced::ThinkingOrb`: full port of libraries.dev thinking-orbs. All nine
  states (working, searching, solving, listening, connecting, weaving, composing,
  breathing, shaping) at the 64, 32 and 20 px designs, speed, dot count and size,
  tint, light/dark/auto theme, pause, reduced-motion static frame, accessible
  label, custom frame functions and per-knob `OrbOptions`. Frames are checked
  against the reference's golden fixture (`tests/thinking_orb_golden.rs`) and
  painted by a software rasteriser that reproduces Canvas2D coverage (visual
  comparison against libraries.dev/orbs: NCC >= 0.99 per orb).
- `enchant_iced::gravity`: the cursor-gravity runtime (nearest-orb pull, bent
  pointer sprite, trail, typing release, frame-budget watchdog), configured
  with `Gravity`, `CursorSprite` and `set_tuning`.
- `enchant_iced::widgets::gooey`: full port of libraries.dev's liquid-gooey.
  `Liquid` / `LiquidItem` provide the merged goo surface (software goo blur and
  alpha contrast, `box-shadow` chains with spread and inset, waviness) plus
  every item effect from the reference: `morph`/`EvolveOptions`, `move`
  (spring lag, trailing droplets, velocity stretch), `bend` (vertical bow and
  horizontal cap deformation), `dissolve` (turbulence warp, edge holes, timed
  release), image `melt`, built-in dragging, and timed or spring transitions.
  The `gooey` example reproduces the full demo gallery (tabs, chips, plus menu,
  split pair, email field, slider, drag card, melting cards, melt pair, avatar
  group) with real iced widgets as children. Raster and engine behavior is
  covered by deterministic tests (goo extent, stretch axis, bend sag, dissolve
  proximity, move transition/velocity).
- `thinking_orb` example (gallery with controls). New dependency: `libm`
  (bit-exact `sin`/`cos`/`exp` parity with the reference); `serde_json` for tests.
- `LICENSE-MIT` and `LICENSE-APACHE`; the project now ships the license texts its
  manifest has always declared.
- `THIRD_PARTY_NOTICES.md`, recording the license and origin of every third-party
  asset bundled with the examples, including Magic UI reference material.
- CI workflow running `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
  and `cargo test`.
- `CONTRIBUTING.md` and `SECURITY.md`.

### Changed

- `confetti` ported to the builder-first API used by the rest of the crate:
  `Confetti`, `ConfettiButton`, `ConfettiOptions`, and `ConfettiShape` replace
  `ConfettiProps`/`ConfettiAction`/`ConfettiBurst` and the free `confetti` and
  `confetti_button` functions. The example now follows the Magic UI
  canvas-confetti gallery.
- Dependency revisions moved forward: `iced-shadcn` to
  `96982c1c2a3c567d4ee847b72521711556fd98c1`, `twill` to
  `83e8ac1a00766b5304ec88d25a9f3e99274ef0e8`. The `twill` revision is deliberately
  the one `iced-shadcn` pins: taking twill's own tip resolves a second copy of the
  crate and breaks type identity across the `iced-shadcn` boundary.
- `README.md` and the crate README rewritten to match the actual workspace. They no
  longer document the removed `crates/wasm-demo` app.
- Whole workspace reformatted with `cargo fmt --all`.
- Real-photograph placeholder avatars replaced with generated gradient art.

### Fixed

- Liquid gooey rasters stay visible while they animate: each rendered layer now
  holds an explicit iced image allocation, so a texture is resident on the
  frame that records it instead of landing a frame late while its handle is
  already replaced (the drag card's pill and the moving tabs indicator used to
  disappear during motion).
- `cargo test --workspace` no longer fails: the ported-helper exports were missing
  from `HELPER_EXPORTS` in the example inventory test.
- `cargo clippy --all-targets -- -D warnings` no longer fails.
- Module-level doc comments now link to the public Magic UI sources instead of an
  internal local checkout path.
- Package `include` list now ships `assets/`, `examples/`, and `tests/`, which the
  library embeds at compile time through `include_bytes!`.

## [0.29.0]

The first version published in this repository's current form. Earlier history is
in the git log.
