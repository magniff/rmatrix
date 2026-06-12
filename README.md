# rmatrix

<img width="1501" height="869" alt="Screenshot 2026-06-12 at 12 25 30 PM" src="https://github.com/user-attachments/assets/90083510-b9e5-4cde-bd96-fbd668334994" />


Digital rain for your terminal — like `cmatrix`, but with richer effects:
true-color glow trails, brightness ripples that wash over the rain, and the
film's actual glyph mix.

## Features

- **True-color trails** — every cell gets a 24-bit color interpolated along a
  glow curve: a hot, bold, near-white head bleeding down into the theme color
  and finally into darkness at the tail.
- **Ripples** — waves of brightness travel across the rain: point splashes
  blooming out of a spot, and wide planar waves sweeping in from off-screen.
  Each wave rolls its own strength; the hot ones overdrive the code toward
  white-green and bold it while the front passes. Click anywhere to drop a
  ripple under your cursor.
- **Authentic glyphs** — half-width katakana (single-column, like the film)
  mixed with Latin capitals, mirrored numerals (Unicode lookalikes Ƨ Ɛ Г —
  terminals can't mirror), and the occasional kanji, with proper handling for
  their double-width cells.
- **Living rain** — streams fall at fractional speeds with glyphs anchored in
  screen space, shimmering by in-place mutation rather than scrolling rigidly.
- **Themes** — green, cyan, amber, purple, and a rainbow that sweeps hue
  across the screen width.

## Install & run

Needs a terminal with 24-bit color support (most modern ones) and a font
fallback chain that covers katakana (macOS and most Linux setups do).

```sh
cargo run --release
```

## Options

```
-c, --color <THEME>   green | cyan | amber | purple | rainbow  (default: green)
-d, --density <N>     stream spawn density, higher = denser    (default: 0.9)
-m, --mutation <N>    glyph flicker rate, higher = busier      (default: 8.0)
-f, --fps <N>         target frames per second                 (default: 60)
-h, --help            show help
```

## Keys

| Key           | Action                                      |
| ------------- | ------------------------------------------- |
| `q` / `Esc` / `Ctrl-C` | quit                               |
| `space`       | pause / resume                              |
| `1`..`5`      | switch theme (green/cyan/amber/purple/rainbow) |
| `+` / `-`     | faster / slower glyph mutation              |
| `r`           | drop a ripple at a random spot              |
| `w`           | send a wide wave sweeping across            |
| mouse click   | drop a ripple right there                   |
