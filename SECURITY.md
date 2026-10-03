# Security policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately rather than in a public issue.

Use GitHub's private vulnerability reporting for this repository:
**Security** tab → **Report a vulnerability**.

That channel opens a private advisory visible only to the maintainers. If the
report is accepted, a fix and a published advisory are coordinated through it.

## Scope

`enchant-iced` is a UI rendering library. It does not implement authentication or
authorization, and it does not process untrusted network input by default. The
areas most likely to matter:

- **`tweet-adapter` feature (off by default).** Enables remote loading through
  `reqwest` for `tweet-card` and `client-tweet-card`. If your application enables
  it, treat fetched content as untrusted input.
- **Asset decoding.** `resvg`, `image`, and `swash` decode SVG, raster, and font
  data. A defect in how untrusted files reach those decoders is in scope.

## Supported versions

The latest published version receives fixes. Older versions do not.
