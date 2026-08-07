# enchant

Native Magic UI experiments for `iced`, organized as a Cargo workspace
(same layout pattern as [shadcn-rs](https://github.com/FerrisMind/shadcn-rs)).

Author: FerrisMind

## Workspace layout

- `crates/enchant-iced`: reusable library crate with widgets, primitives, and theme helpers.
- `crates/wasm-demo`: Trunk-powered demo application that replaces the old phase showcase examples.

## Run

```powershell
cargo check --workspace --all-targets
cargo test -p enchant-iced
cargo clippy -p enchant-iced --all-targets -- -D warnings
cargo build -p wasm-demo --target wasm32-unknown-unknown
trunk serve crates/wasm-demo/index.html
```

## Routes

- `/`: marketing home
- `/components`: native component catalog
- `/docs/components/:slug`: component detail pages with preview/code tabs

## Notes

- Legacy `phase0..5_showcase` examples and capture artifacts were removed from the workspace root.
- The demo shell is built with `enchant-iced`, `iced-shadcn`, Lucide, and twill.
- Magic UI build inputs for the demo live in `crates/wasm-demo/upstream/magicui-www/` (minimal snapshot; sync from `magic-refs` via `scripts/sync-magicui-snapshot.ps1`). The project does not depend on `magic-refs` at build time.
