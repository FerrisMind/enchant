# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
