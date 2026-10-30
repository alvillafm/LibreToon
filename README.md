# LibreToon

LibreToon is a native 2D animation editor built with Rust and egui/eframe, inspired by Adobe Animate. It is an early development version.

## Run and develop

On Windows, open `Abrir-LibreToon.cmd` to run `target/release/libretoon.exe`. Build with `cargo build --release --locked` after source changes. `Desarrollar.cmd` runs the development workflow. Rust stable, MSVC and the Windows SDK are required to build.

## Implemented

- Editable vector paths with Bézier anchors and handles; standard fill brush, vector brush, pen, shapes, eraser, paint bucket and variable stroke width.
- Selection, partial fill selection, lasso, polygon selection, magic wand, object selection, free transform, mirror, and internal copy/cut/paste (`Ctrl+C/X/V`). Paste inserts into the active layer at the original coordinates.
- Unified custom title bar with native minimize/maximize/close commands. Drag the empty header area to move the window; double-click it to maximize/restore.
- Layers, frame-by-frame timeline, playback and onion skin with independent previous/next frame counts directly beside the onion icon.
- Stage-only preview beside the rotate/fit controls. Artwork outside the stage remains in the document. Guides and editing overlays are not exported.
- Rulers with draggable horizontal/vertical guides. Activate Ruler under Canvas, then drag from a ruler. Move guides with Ruler; drag outside the canvas to remove them. Units: px, cm, mm and inches, using configurable pixels per inch (default 96). Guides are session view settings, cleared on opening or creating a document.
- Horizontal and vertical symmetry about the stage center, configurable under Symmetry. It affects committed brush strokes, shapes and pen paths; both axes generate four versions. It does not alter existing artwork.
- Hold the paint brush button for 400 ms, or right-click it, to select standard, fill, shading and lighting brushes. Fill Brush clips ink to existing fills in the active frame/layer, including its preview. It adds no ink on an empty layer. Toon shading/lighting preserve contours and have independent per-region budgets, controlled by Toon levels (default 1, up to 16). A region at its limit is unchanged on subsequent strokes. Increasing the limit permits more passes. Counters survive cutting, undo and native save/load; explicit recoloring resets them. Blend, exposure and contrast modes offer adjustable strength. Results are committed when the stroke ends.
- File > Document settings controls stage dimensions and FPS. File > Program preferences has the current Black theme, English (default), Spanish, and remappable tool shortcuts. Click a shortcut, then press a key combination; Escape cancels and Delete clears. Conflicting tool bindings are reassigned. Program commands such as copy/save/undo remain reserved. Preferences persist under `%LOCALAPPDATA%/LibreToon/preferences.json`.
- Linear/radial gradients on selected fills and the paint bucket, with 2–32 color stops, per-stop opacity and editable start/end coordinates. Stroke start/end caps can independently be round, flat, square or triangular.
- Gradient controls include draggable color stops, per-stop opacity, a rotation dial and on-stage start/end handles. Eyedropper preserves the selection and applies a sampled fill/gradient to it, mapping its coordinates to each target. Stroke caps use dedicated visual icon buttons.
- Click the numbered scene breadcrumb to open Scene manager. Add scenes, double-click names to rename, drag the hand icon onto another name to reorder, and preview all scenes in export order. Deletion requires confirmation and keeps at least one scene. Video/GIF exports and SVG sequences use the ordered movie timeline. Symbols are shared across scenes; each scene keeps its own artwork, layers, folders, cameras and perspective guides.
- Motion tweens interpolate position, shortest-path rotation, scale and a symbol instance’s selected frame. Transforming a held exposure at a later frame automatically creates a key and a span from the exposure’s first key. The motion icon also creates a span to the next populated key (or nine frames ahead). Editing inside a span materializes the evaluated pose as a new key. Purple timeline cells identify motion spans. Properties offers transform values, linear/ease-in/ease-out presets, a draggable timing graph and a Bézier path with on-stage handles. The trash icon removes the span. These are transform tweens, not path-topology morphs.
- Perspective guides with a horizon, 1–3 draggable vanishing points and adjustable ray density, stored in the document.
- Grouped timeline controls for layers, nested folders and keyframes. Drag layer names to reorder or onto folders to group; drop on the layer-name header to return to the root. Folder double-click collapses/expands, and its context menu renames it. Layer names rename on double-click. Visibility, locking and percentage opacity appear beside each layer. Clipping masks use the silhouette of the immediately underlying visible layer. Frame-number dragging scrubs; comma/period step backward/forward. Keyframes can be added, blanked, held longer, shortened or removed. New documents/layers start with a held blank keyframe across the initial duration.
- Multiple cameras in an orange timeline row. Add them from the Camera tool. Cuts are adjacent, can be reordered by dragging, and have configurable durations, position, rotation and zoom. The white viewport stays fixed while the content follows the drag. Pan, Rotate and Zoom icon modes edit the selected camera and automatically record a local key at the current frame. Position, zoom and rotation interpolate between keys; the camera motion section controls easing and Bézier offsets. Playback, Stage-only preview and exports use the active cut.
- Graphic symbols: select artwork and press F8 (or use Edit/the selection context menu). Double-click an instance with a selection tool to edit its own timeline; use Back to scene to return. The Properties/Library tabs manage instances and reusable definitions. Instances offer Loop, Play once and Single frame with a visual thumbnail frame picker. Playback modes use icon buttons; the Library displays a rendered preview of each symbol. Nested symbols are supported, without recursive references. Each symbol keeps its own layer folders. Editing contexts have separate undo histories. Library Images/Videos/Sounds sections reserve space for future media import; import is not implemented yet.
- Undo/redo, asynchronous session recovery, native project metadata and limited FLA/XFL interchange.

## Export

File > Export video offers MP4 (H.264), SVG, MKV (H.264), AVI (MPEG-4) and MOV (ProRes 422 HQ). File > Export GIF offers palette size, dithering and loop settings. Choose a name and folder, dimensions, frame range and FPS. Export runs on a worker with progress and cancellation. SVG can export the current frame or a numbered frame sequence.

The bundled `assets/tools/ffmpeg.exe` handles video and GIF and is automatically detected when opening export. No download is needed. A different executable can be selected, or the bundled path restored, in the export dialog. Keep this folder with the application when packaging it. GIF is available only through Export GIF. See `assets/tools/THIRD_PARTY.md` for FFmpeg's origin and license. Output is silent, with a white background. Raster output is limited to 4096 × 4096; dimensions retain aspect ratio with letterboxing. YUV codecs pad odd dimensions to the next even value. GIF timing is quantized to hundredths of a second. Frame-rate conversion repeats/drops frames without interpolation. SVG remains vector output at document dimensions.

## Interchange and limitations

FLA support is limited to implemented vector geometry, colors, layers and frames. Unsupported XFL content is rejected rather than silently discarded. Native metadata is checked against visible XFL so external edits are not hidden. There is no general compatibility promise for arbitrary Animate files, symbols, scripts, rigs or effects.

Selection and Boolean operations approximate fill boundaries; stroke subdivisions preserve Bézier handles and variable-width profiles. Selection, shading and fill operations currently have no spatial index. See `PERFORMANCE.md` for measurements and their limits.

Bone rigging, shape/topology morphing, media import and general filters/effects remain future work.

Advanced document data is retained in LibreToon's native metadata inside FLA. Standard XFL output concatenates scenes and bakes motion, symbols, cameras, clipping and layer opacity; gradients use a 24 × 24 polygon approximation in that interoperability projection. Native FLA retains editable gradients/symbols and SVG retains gradient definitions. The standard XFL projection is not an Adobe symbol library or camera layer. Unsupported external XFL content continues to be rejected.

## Validate

```powershell
cargo fmt --all -- --check
cargo test --locked
cargo test --release --locked export_all_containers_with_ffmpeg -- --ignored --nocapture
```

The export integration test encodes all six formats and decodes raster formats to verify three colored frames and their order. It requires bundled FFmpeg or an executable in PATH. In a restricted environment, point TEMP, TMP and LOCALAPPDATA to isolated writable directories before running tests or the application.
