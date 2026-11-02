# LibreToon artwork provenance record

Recorded 2 November 2026. Repository evidence is distinguished from authorship/rights confirmation. This document is not a new license grant.

## Procedural toolbar icons

src/design.rs, Icon and draw, contains the vector drawing instructions. Its source comment describes them as "Original 20-unit line icons, drawn as geometry". These are project source drawings, distinct from the logo files and from icon glyphs in egui's third-party fonts. The repository declares MIT in its root LICENSE. The comment is an internal provenance declaration, not independent proof of originality or contributor ownership. No specific designer is attributed without evidence.

## Application artwork

| File | Evidence from repository history |
|---|---|
| libretoon.svg | Added in commit 205ee1b, 29 October 2026; inline SVG titled "LibreToon — ratón ilustrador"; no per-file author or license metadata found. |
| libretoon.png | Appears in a0e0d0e, 30 October 2026; updated in 0a749c9 the same day. |
| libretoon-icon.png | Appears in a0e0d0e, 30 October 2026; updated in 0a749c9 the same day. Used for the window icon by src/design.rs. |
| libretoon.ico | Appears in a0e0d0e, 30 October 2026; updated in 0a749c9 the same day. Embedded through build.rs. |

The header-logo helper calls the application artwork "supplied app artwork". The PNG files have no tEXt/iTXt/zTXt provenance chunks. This supports the recorded usage/history but does not identify the creator, the source of the supplied artwork, or the derivation between SVG/PNG/ICO versions. A Git commit is not proof of copyright ownership.

## Maintainer provenance update — 2 November 2026

The maintainer has now declared that project visuals were created with Codex assistance and the blue-mouse/LT logo was generated through ChatGPT. See [ASSET_PROVENANCE.md](ASSET_PROVENANCE.md) for the declaration, resource classification and its limits. The earlier historical observations above are retained as repository evidence; they are not the only provenance information now available. The grey SVG is not assumed to be the source of the blue raster logo. Exact conversion/generation records and an independent rights determination are still unavailable. No artwork bytes were changed.

## Current file identification

- libretoon.svg: SHA-256 cb40f010d6d6dcd4caaaa6a05e4dd15adee697c49c8899f151f34fa2d77f21b0
- libretoon.png: SHA-256 8807318035c649351d1c94e61702017f1a4a6b33c71f20f921da22adcd4f1664
- libretoon-icon.png: SHA-256 12aa5e96bf96e11664c53eb80c134dabfcc884854efd4e5fc06bcf73c6b06c88
- libretoon.ico: SHA-256 ee9855c5874da303482fd41f5e1aae7f8a7e82340900f77b96d7b9076226c667
