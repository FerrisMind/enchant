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

## Notes on parity

- Each port documents its upstream reference in a module-level doc comment and links
  to the corresponding Magic UI source.
- `tools/visual-verify/` holds Playwright/pixelmatch helpers used to compare native
  output against Magic UI reference captures. They are development tooling and are
  not part of the library.
- `scripts/example_launcher.py` is a local convenience launcher for the example set.

## Licensing

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Third-party components bundled with the examples (font, brand marks, reference
material) are documented in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
