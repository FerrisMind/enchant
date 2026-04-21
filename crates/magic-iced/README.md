# magic-iced

Magic UI-inspired premium interaction layer for `iced`.

Author: FerrisMind

## Crate role

`magic-iced` is the reusable library layer inside the workspace. It exports the widgets, sections, theme helpers, and animation primitives used by the new `wasm-demo` app.

## Included surface

- foundations: animation, text sequencing, anchor geometry, scroll/time helpers, effect canvas, media helpers
- layout and marketing widgets: `bento-grid`, mockups, `marquee`, `number-ticker`, `text-reveal`, `terminal`
- premium interaction widgets: `magic-card`, `animated-beam`, `file-tree`, `word-rotate`, `typing-animation`, `shimmer-button`, `shiny-button`, `avatar-circles`, `blur-fade`, `animated-circular-progress-bar`
- extended effects: `warp-background`, `progressive-blur`, `neon-gradient-card`, `meteors`, `grid-pattern`, `striped-pattern`, `dot-pattern`, `animated-grid-pattern`, `orbiting-circles`, `backlight`, `line-shadow-text`, `aurora-text`, `animated-shiny-text`, `hyper-text`, `sparkles-text`, `spinning-text`, `text-animate`, `highlighter`, `animated-list`
- interactive and media widgets: `morphing-text`, `scroll-progress`, `pointer`, `smooth-cursor`, `hexagon-pattern`, `interactive-grid-pattern`, `glare-hover`, `dock`, `comic-text`, `cool-mode`, `pixel-image`, `animated-theme-toggler`, `light-rays`, `text-3d-flip`, `lens`, `globe`, `tweet-card`, `client-tweet-card`, `particles`, `retro-grid`, `flickering-grid`, `icon-cloud`, `scroll-based-velocity`, `confetti`, `video-text`, `dotted-map`

## Platform notes

- no DOM refs or `ResizeObserver`: anchor geometry is measured through widget `Id` plus runtime widget operations
- no CSS keyframes: motion is driven by `AnimatedValue`, `TextRunner`, `FrameDriver`, and timestamped update messages
- `magic-card` uses surface-local pointer tracking and theme-registry colors instead of DOM pointer listeners
- `tweet-card` is data-first; remote loading stays behind the optional `tweet-adapter` feature
- decorative surfaces stay generic in `iced-shadcn`; Magic-specific effects stay in `magic-iced`

## Verify

```powershell
cargo check -p magic-iced --all-targets
cargo test -p magic-iced
```

## Demo app

The old root-level phase showcases were removed. Run the workspace demo instead:

```powershell
trunk serve crates/wasm-demo/index.html
```
