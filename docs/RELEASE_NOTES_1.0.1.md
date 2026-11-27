# LibreToon 1.0.1

Changes since 1.0.0.

## Editable rectangle corners

- Rectangle primitives now retain their corner parameters after creation and saving.
- Select a rectangle to edit one linked radius or four independent corner radii in Properties.
- Fill and outline share the same boundary and update together; stroke/fill colors, gradients and width profiles are preserved.
- Parameters follow affine transforms, including rotation and scaling. Existing square rectangle primitives can also be rounded.
- Radius changes form one undo gesture. Neighboring arcs are limited proportionally to prevent overlapping corners.
- Manual node edits remain available; changed boundaries stop using primitive controls rather than being overwritten.

## Oval primitives and quick copy

- Ellipse now creates editable oval primitives with start/end angles, inner-radius controls and full/half/ring presets. Parameters and combined fill/stroke survive native saving and affine transforms; manual node edits remain authoritative.
- Alt-drag with Selection, Object Selection or Transform duplicates and moves selected objects while leaving originals in place. A click creates no copy; Esc cancels, and the completed gesture takes one undo step.
- Copying preserves symbol/media resources while assigning independent instance/boundary IDs and copying supported rig bindings. Partial copies leave original paths intact.

## Shape masks

- Create rectangle/ellipse masks from the timeline toolbar, then edit their shapes and Bézier nodes independently of the artwork.
- Reveal or hide the masked interior, bypass the mask, return to its target, or remove it without changing the original drawing.
- Mask rows support normal exposures, keyframes and compatible shape/motion tweens. Editing boundaries are shown in green and excluded from export.
- Native saving retains mask mode and keys; vector and raster artwork use the shared geometric clipping path.

## Fill and outline selection

- Selecting rectangle primitives no longer automatically separates their fill from the stroke.
- Select a separate fill and closed outline in the same layer, then use **Properties → Link fill and outline** to fit the fill to the outline and keep them together. Undo restores both originals.
- Linking through the bucket accepts an existing bucket fill even when its flattened nodes differ from the closed outline.

## Linked bucket boundaries

- Linked bucket fills can follow regions enclosed by multiple separate vector strokes.
- Boundary references survive native saving/reopening. Node, handle and outline edits rebuild the fill at edit completion, preserving color, gradients and order in the same undo step.
- Open, missing or ambiguous boundaries keep their last valid fill instead of flooding the drawing; a status message explains the problem.
- Work is restricted to edited drawings; viewport rendering and playback do not recalculate linked regions.

## Windows startup compatibility

- Windows uses DirectX 12 by default instead of probing Vulkan drivers during initialization.
- A supplied crash report identifies an access violation inside Intel `igvk64.dll` version 31.0.101.2121. Avoiding that startup path is a targeted mitigation, pending confirmation on the affected mini PC.
- Fixed the post-processing shader for the legacy DirectX shader compiler by removing dynamic RGB vector indexing; shader-internal errors are captured for fallback.
- Explicit `WGPU_BACKEND` overrides remain supported. Defaults on other platforms are unchanged.
- Recoverable window-initialization errors now show a dialog and attempt to write a diagnostic log in the temporary directory. Native driver crashes cannot be caught by this Rust error handler.

## Verification

542 automated Rust tests passed, with no new Clippy warnings. Additional headless checks cover the corner-properties layout, GPU playback through DirectX 12 on Orbit and DBGT scenes, and all 16 post-processing effects against the reference renderer. The affected Intel mini PC was not available locally; confirmation on that device remains pending.
