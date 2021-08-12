# Censor
[![Version info](https://img.shields.io/crates/v/censor-analyser.svg)](https://crates.io/crates/censor-analyser)

`censor` is a standalone palette analyser inspired by DawnBringer's Palette Analyser for GrafX2 and having a compatible layout.
It helps creating and evaluating restricted palettes by providing visualisations and highlighting certain properties.

## Examples
![Aerugo](./examples/aerugo.png)
![Aurora](./examples/aurora.png)
![Cheese palette](./examples/cheese-palette.png)
![Obra Dinn](./examples/obra-dinn-ibm-8503.png)

## Installation
Install the rust toolchain using `rustup` or the repository. Then run:
```
$ cargo install censor-analyser
```

## Features
- Analyse palettes of 2-256 colours
- All widgets use CAM16UCS with perceptual colour distances
- Load colours from command line arguments, text files, images and Lospec
- Daemon mode for large amounts of analysis requests
- WASM support for analysing palettes on web pages client-side (currently broken)

## Widgets available
- Rectangular hue-lightness with fixed chroma
- Spectral colours (with the non-spectral CIExy line added)
- Spectral box (with `y` parametrising a parabola going through pure black, the spectral colour and pure white)
- Indexed palette
- Close colours (with different lightness weights)
- Internal similarity (which is `(mean_d/min_d)/n^(2/3)`)
- Acyclic check
- "Spectral" distribution (from hue-mapping nearest spectral colours relative to the white point)
- "Temperature" distribution (normalised `-log(CCT)`, might be changed later)
- Greyscale with differing lightness weights
- Cubes showing the colours in CAM16UCS space from different angles
- Hue-lightness plot
- Useful colour mixes (i.e. producing points that are further from other palette points)
- Lightness-chroma bars
- Lightness-sorted colours and neutralisers
- 12-bit RGB approximation
- Polar hue-chroma plot
- Polar hue-lightness with fixed chroma levels
- Complementaries
