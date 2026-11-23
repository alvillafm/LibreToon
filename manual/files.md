# Import and Export

## Choose the right format

| Format | Purpose | Important limit |
| --- | --- | --- |
| `.ltoon` | Editable native project | Keep the original when testing a newer version |
| `.fla` | Exchange supported Animate artwork | Import may omit or approximate unsupported content; export is experimental |
| `.svg` | Import editable vector artwork | Inspect the compatibility notices; not every SVG appearance feature is supported |
| PNG sequence | Rendered frames with transparency when configured | Produces multiple files |
| Sprite sheet | Frames arranged in a grid | Choose rows and columns for your destination |
| SVG | Vector delivery | Verify the result in the receiving application |
| Video / GIF | Playback and sharing | Video encoding requires FFmpeg |

## Import FLA

Open a compatible FLA, inspect the stage and timeline, then read **File → FLA Import Report**. Check symbols, masks, timing and unsupported features before saving a native copy. ActionScript is not executed or supported.

## Import SVG artwork

Choose **File → Import → SVG vector artwork…**. After parsing, the dialog reports the editable path count and any compatibility notices. Choose whether to fit the artwork to the stage, then click **Import vectors**. A new vector layer is added at the current frame; the source SVG is preserved. Undo removes the import in one step.

Supported artwork includes paths, cubic and quadratic curves, arcs, basic shapes, nested group transforms, solid fill/stroke colors, opacity, and common linear/radial gradients, including linked Inkscape gradient definitions. Compound fills become editable polygon boundaries; their original Bézier strokes remain available. Some stroke transforms and radial gradients are approximated.

SVG text, linked images, scripts, CSS classes, filters, masks and clipping features are not reproduced as native SVG effects. Read the dialog's notices before importing. In the source editor, convert text or special objects to paths when possible. SVG is an artwork exchange format here, not a project or animation interchange format.

## Import an image sequence

1. Choose **File → Import → Image sequence…** and select the still images you want to import. PNG, JPEG, BMP, TIFF and WebP files are supported; some decoders may require FFmpeg.
2. Check the displayed first and last filenames. Files are sorted numerically, so `frame2.png` precedes `frame10.png`.
3. Choose a layer name, first timeline frame and **Frames per image**. A value of 2 holds every image for two frames.
4. Optionally fit the images to the stage while keeping aspect ratio. Leave this off to retain their pixel dimensions.
5. Enable **Set project FPS** only if you want to change the project's frame rate. Otherwise the current FPS is retained.
6. Click **Import sequence**. Decoding runs in the background with progress and cancellation. The completed sequence is inserted on a new layer as image keyframes and held exposures. Existing drawings are retained, and the timeline extends if needed.

Imported images are embedded in the project. The sequence must fit within the 10,000-frame timeline and the 256 MiB media budget. A failed decode does not insert a partial sequence. One Undo removes the whole import, including its added media.

## Export a short test

1. Save your editable project.
2. Choose the required export command from **File**.
3. Set the output size, frame range and destination.
4. Export a short representative range first, including a camera move and any lights or FX.
5. Open the result outside LibreToon and check timing, sharpness, audio and transparency as applicable.

Viewport quality does not lower final export quality. The renderer uses the requested output resolution. For video, configure a compatible FFmpeg executable when prompted. An installed FFmpeg can be selected manually on platforms without a bundled download.
