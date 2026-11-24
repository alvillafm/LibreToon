## LibreToon v1.0.0
Changes since v0.9.0 — vector editing, animation workflow, performance and safer saving.

**Drawing & color**

- Expanded Bézier editing: Corner/Smooth/Symmetric/Automatic nodes, multi-node selection, linked or broken handles, joining/splitting, alignment and numerical transforms.
- Simplify, Expand Stroke and Offset Path with previews.
- Gradient vector strokes, vector stabilizer, pressure curves, custom raster tips and alpha lock.
- Bucket region/gap-bridge preview, background fills and per-frame results.

**Animation & rigging**

- Continuous tween bars; move entire tweens or multiple drawing/bone/pin keys.
- Fixed accidental bone/pin movement on selection and grouped Properties edits into single undo steps.
- Independent timing curves per supported transform channel.
- Rig setup guidance, batch attach/mirror/rename and weight diagnostics.
- Lip Sync key protection, phrase regeneration and direct source-pose editing.

**Performance & projects**

- Direct GPU-resident viewport presentation and better cache reuse.
- Configurable GPU cache and undo-history budgets.
- Streamed native storage removes the total 256 MiB project ceiling.
- Clearer save status, stronger save/recovery handling and undoable resource cleanup.

**Import, export & learning**

- Editable SVG and image-sequence import.
- Improved experimental XFL gradient export.
- Transparent animated GIF and ProRes 4444 MOV export.
- Illustrated wiki with five progressive tutorials and practice projects.
- Layer Objects selection/context menus and layout improvements.

515 automated Rust tests passed in the latest development run, plus packaging/documentation checks. Performance varies by hardware and scene; compatibility limits and unfinished audit work are documented in the full release notes.
