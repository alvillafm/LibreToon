# UI font fallback

`NotoSansCJK-Regular.ttc` is the unmodified Noto Sans CJK regular collection from the official [notofonts/noto-cjk](https://github.com/notofonts/noto-cjk) project:

https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTC/NotoSansCJK-Regular.ttc

SHA-256: `b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a`

Redistributed under the SIL Open Font License 1.1; see `OFL-NotoSansCJK.txt`. Copyright and font metadata remain in the original collection. UI fallback selects Japanese face 0, Korean face 1 or Simplified Chinese face 2. Existing Latin/Cyrillic UI fonts remain first in the fallback chain.

The collection is embedded into the executable, so UI glyphs do not require a font installation or a runtime download. Windows packaging keeps this license and omits a redundant standalone copy of the embedded font. Building from source requires the TTC at this location.

This does not change the editable Text tool: artwork fonts are still chosen separately and stored as vector outlines.

## Copyright and attribution evidence

See [COPYRIGHTS.md](COPYRIGHTS.md) for notices read from the actual local fonts, including the Adobe notice in Noto CJK. Preserve that notice with the OFL text when distributing the embedded font. Notices for fonts supplied by egui are collected in [../licenses/fonts/](../licenses/fonts/README.md).

The recorded Noto SHA-256 was rechecked locally on 2 November 2026 and matches. The upstream/unmodified description above is existing project documentation, not a fresh upstream download comparison in this pass.
