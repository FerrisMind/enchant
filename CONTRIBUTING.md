# Contributing

Thanks for taking a look at `enchant`. This is a small, opinionated port project,
so please open an issue before starting significant work.

## Before you open a pull request

Run the same checks CI runs:

```powershell
cargo fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All four must be clean. CI runs them on every push to `main` and on every pull request.

## Requirements

- Rust 1.93 or newer (edition 2024).
- A working GPU/window backend if you plan to run examples: `cargo run --example <name>`.

## Porting conventions

Each component is a port of a Magic UI component, and the port keeps three things
in sync:

1. **A module-level doc comment naming the upstream source.** Every ported module and
   example states the upstream file it was derived from and links to it.
2. **A builder-first Rust API.** Private fields, fluent setters that validate, a
   `#[must_use]` builder, and a terminal `into_element` / `view`. Public enums are
   `#[non_exhaustive]`. Accessors do not use a `get_` prefix.
3. **A runnable example.** A component without an example is not finished. The test in
   `crates/enchant-iced/tests/example_inventory.rs` enforces that every public
   component has an example binary and is covered by the upstream registry inventory.
   If you add a public helper that is not a component, add it to `HELPER_EXPORTS` in
   that test rather than inventing an example for it.

## Formatting

`cargo fmt --all` is authoritative. Do not hand-format around it.

## Licensing

Contributions are accepted under the project's dual license, Apache-2.0 OR MIT.
Do not add assets you cannot license. Any new bundled asset (image, font, icon,
mark, reference capture) must be listed in `THIRD_PARTY_NOTICES.md` with its
source and license, and must be an asset you have the right to redistribute.
