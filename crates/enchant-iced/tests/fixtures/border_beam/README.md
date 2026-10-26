# Border beam parity fixtures

80 PNGs captured from the Libraries.dev `border-beam` v1.4.1 reference
(`packages/border-beam`, source at `enchant-refs/Libraries.dev`) with the
Playwright harness in `.superpowers/beam-parity/`:

- scene: 348x122 card, radius 20, 60px page padding, device scale factor 2
  (936x484 output);
- grid: 5 sizes x 8 palettes x 2 themes;
- frozen times: `sm`/`md` 0.49s, `line` 1.55s, `pulse-inner`/`pulse-outside`
  0.8s.

`ref-fixed/` is the same capture run with `PARITY_REPAIR=1`, which applies a
Vite transform that repairs upstream's malformed `rgba(r, g, b,, a)`
declarations for `forest`, `candy`, `ice` and `gold` in `styles.ts`. The port
repairs those declarations too (see the module-level "Deliberate deviation"
note), so parity is measured against `ref-fixed/`.

Regenerate with:

```sh
cd .superpowers/beam-parity
PARITY_REPAIR=1 npx vite --port 5173 --strictPort
PARITY_OUT=ref-fixed node capture.mjs
```

The test `tests/border_beam_parity.rs` fails if these fixtures are missing; it
compares 6x4 mean-colour blocks with a tolerance of 10/255 per channel.
