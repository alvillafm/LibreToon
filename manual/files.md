# Import and Export

## Choose the right format

| Format | Purpose | Important limit |
| --- | --- | --- |
| `.ltoon` | Editable native project | Keep the original when testing a newer version |
| `.fla` | Exchange supported Animate artwork | Import may omit or approximate unsupported content; export is experimental |
| PNG sequence | Rendered frames with transparency when configured | Produces multiple files |
| Sprite sheet | Frames arranged in a grid | Choose rows and columns for your destination |
| SVG | Vector delivery | Verify the result in the receiving application |
| Video / GIF | Playback and sharing | Video encoding requires FFmpeg |

## Import FLA

Open a compatible FLA, inspect the stage and timeline, then read **File → FLA Import Report**. Check symbols, masks, timing and unsupported features before saving a native copy. ActionScript is not executed or supported.

## Export a short test

1. Save your editable project.
2. Choose the required export command from **File**.
3. Set the output size, frame range and destination.
4. Export a short representative range first, including a camera move and any lights or FX.
5. Open the result outside LibreToon and check timing, sharpness, audio and transparency as applicable.

Viewport quality does not lower final export quality. The renderer uses the requested output resolution. For video, configure a compatible FFmpeg executable when prompted. An installed FFmpeg can be selected manually on platforms without a bundled download.
