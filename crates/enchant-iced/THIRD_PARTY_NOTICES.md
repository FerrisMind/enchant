# Third-party notices

`enchant-iced` is licensed under Apache-2.0 OR MIT (see `LICENSE-APACHE` and
`LICENSE-MIT`). This file records third-party material bundled in this
repository.

Rust dependencies are consumed through Cargo and are not vendored here; each
carries its own license in its own distribution.

## Reference material

### Magic UI

Many components in this repository are ports of Magic UI components. The
upstream project is licensed under the MIT License:

- Copyright (c) Magic UI
- https://github.com/magicuidesign/magicui
- https://magicui.design

Each port names its upstream source file in a module-level doc comment.
`crates/enchant-iced/tests/fixtures/registry-examples.ts` is a derived snapshot
of Magic UI's component registry metadata and is likewise MIT-licensed, with the
notice above preserved.

### Libraries.dev

Several widgets are ports of [Libraries.dev](https://libraries.dev) effect
libraries (source: https://github.com/Jakubantalik/Libraries.dev). Those packages
are licensed under the MIT License:

- Copyright (c) 2026 Jakub Antalik
- https://libraries.dev
- https://github.com/Jakubantalik/Libraries.dev

Ported packages include (non-exhaustive):

| Upstream package | enchant-iced surface |
| --- | --- |
| `border-beam` | `border_beam` |
| `thinking-orbs` | `thinking_orb` |
| `bot-avatars` | `bot_avatar` |
| `liquid-gooey` | `gooey` / `Liquid` |
| `voice-glow` | `voice_glow` |
| `metal-fx` | `metal_fx` |

Each port names its upstream source in a module-level doc comment.
`crates/enchant-iced/tests/fixtures/border_beam/` holds captured reference frames
from Libraries.dev `border-beam` used for visual parity tests.

## Fonts

### Geist

- Files: `crates/enchant-iced/assets/Geist/*.ttf`
- Copyright (c) Vercel, Inc.
- Licensed under the SIL Open Font License, Version 1.1
- Full license text: `crates/enchant-iced/assets/Geist/OFL.txt`
- https://vercel.com/font

## Brand marks

The following files are logos or brand marks used to demonstrate animation
effects. They are the trademarks of their respective owners and are used
nominatively to identify the corresponding services; no affiliation or
endorsement is implied.

| Files | Owner |
| --- | --- |
| `examples/assets/beam/google-docs.svg`, `examples/assets/beam/google-drive.svg` | Google LLC |
| `examples/assets/beam/messenger.svg`, `examples/assets/beam/whatsapp.svg` | Meta Platforms, Inc. |
| `examples/assets/beam/notion.svg` | Notion Labs, Inc. |
| `examples/assets/beam/openai.svg` | OpenAI |
| `examples/assets/beam/zapier.svg` | Zapier, Inc. |
| `examples/assets/cool-mode/rust-logo.png` | Rust Foundation (Rust logo, trademark) |
| `examples/assets/beam/user.svg` | generic icon, no third-party rights |

The Rust logo is a trademark of the Rust Foundation and is used here to
identify the Rust language in a demo.

## Demo imagery

Photographic and illustrative assets used as placeholder content in examples:

- `examples/assets/android-demo.jpg`, `examples/assets/android-demo-video.jpg`
- `examples/assets/hero-light.png`, `examples/assets/hero-dark.png`
- `examples/assets/backlight/hero.jpg`
- `examples/assets/lens-camp.jpg`
- `examples/assets/marquee/*.png`
- `examples/assets/avatars/*.png`
- `examples/assets/iced-shimmer-port.png`, `examples/assets/iced-shimmer-crop.png`
- `examples/assets/magicui-shimmer-ref.png`, `examples/assets/magicui-shimmer-t0.png`, `examples/assets/magicui-shimmer-t1.png`

These are decorative placeholders inside widget previews. The `magicui-shimmer-*`
captures are screenshots of the Magic UI reference implementation used for visual
comparison. `avatars/*.png` and `marquee/*.png` are generated placeholder art.

## Chrome and device frames

`crates/enchant-iced/assets/*.svg` (Safari, iPhone, and Android device frames and
their overlays) are simplified outline drawings of device chrome. They are not
official assets of Apple Inc. or Google LLC; those names and device shapes are
referenced descriptively.
