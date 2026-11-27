# Drawing and Color

- **Vector layers (V)** store paths and editable nodes. Choose **Pen** or **Node** to edit a selected path. Alt-click an anchor to convert it to a corner. Alt-drag a handle to adjust it independently.
- **Raster layers (R)** store pixels. Choose a raster brush preset and adjust hardness and opacity in **Properties**.
- Open **Color** and choose **Fill** or **Stroke** before picking a swatch. Edit a linked swatch to recolor its associated vector artwork. Use the eyedropper to sample a reference image loaded with **Tools → RefTool**.
- For several similar drawings, use the **Paint Bucket** and choose a multi-frame mode in **Properties**. Click inside the region to fill. Check the results on each drawing before proceeding.

## Brush stabilization

Vector drawing tools offer **Stabilizer distance** in Properties. Zero keeps the original input. Increasing it makes the brush tip follow behind the pointer; a contrasting guide shows that delay. Distances use document pixels, so zoom does not change the stored brush width or stabilization distance. Releasing the pointer finishes at its final position. Smoothing and pressure response remain separate controls.

## Editing Bézier paths

Select a vector path, then choose **Node** or **Pen**. The floating toolbar and **Properties** offer four anchor types:

| Type | Handle behavior |
| --- | --- |
| Corner | Handles move independently; useful for sharp corners. |
| Smooth | Handles stay aligned, with independent lengths. |
| Symmetric | Handles stay aligned with equal lengths. |
| Automatic | Tangents follow neighboring anchors automatically. |

**Shift-click** anchors to add or remove them from the selection. Drag a selection box to select several anchors, then drag any selected anchor to move the group. **Ctrl/Cmd+A** selects all anchors on the selected paths. Enable **All handles** to display every handle.

**Alt-drag** a handle to break its linkage. Double-click a segment to insert an anchor while preserving the exact curve. **Delete** removes the selected anchors; **Preserve curve** attempts to retain the silhouette when bridging the remaining anchors. Press **Escape** during a drag to cancel it.

In Properties, **Connect** joins two compatible open stroke endpoints. **Merge** joins them at a shared position. **Break / Open** splits an open path at an interior anchor or opens a closed contour. Splitting a filled open path preserves its original fill as a separate element. **Separate fill / stroke** explicitly separates a combined shape when needed; ordinary node edits keep combined fills and strokes together.

Expand **Align and distribute** or **Transform selected nodes** to align or distribute anchors and apply a numerical translation, rotation or scale. **Simplify** opens a preview with a pixel tolerance and an option to preserve corners. Review the cyan candidate contour, then choose **Apply** or **Cancel**. Higher tolerances can change the silhouette, so inspect important corners before applying.

Anchor types are saved in `.ltoon` projects. Older projects infer their initial handle behavior from the stored geometry.

## Gradient outlines

Select a vector path and open **Properties → Gradient**. Choose **Stroke**, enable **Gradient**, and edit its color stops, opacity, linear/radial type and rotation. Choose **Fill** to edit the fill gradient independently. The same options apply to new paths when a vector drawing tool is active.

With Selection or Transform active, choose the desired Fill/Stroke channel and drag the gradient endpoints on the canvas to reposition it. Applying a solid stroke color replaces the stroke gradient. Native projects preserve both gradients; image/video exports render them, and SVG export converts gradient strokes into filled outline geometry.

## Preview bucket regions

In Paint Bucket properties, enable **Preview region and gap bridges**, then click the region. The background worker prepares a candidate without changing the document. Cyan shows the candidate boundary; orange shows temporary endpoint bridges where applicable. Choose **Apply** to commit or **Cancel** to leave the drawing unchanged. For multiple frames, select a result row to inspect that frame. Gap bridges never alter the original outlines. Closed stroke silhouettes can form barriers without endpoint bridges.

## Editable rectangle corners

Choose **Rectangle** and open **Rectangle corners** in Properties. **Link corners** uses a single radius; disable it to set Top left, Top right, Bottom right and Bottom left independently. The reset icon returns the corners to square.

After drawing, switch to **Selection**, select the rectangle and edit the same controls at the top of Properties. Fill and stroke share the generated boundary, so both follow the change. These settings survive saving/reopening and affine movement, rotation and scaling. Old square rectangle primitives can also be rounded. Arbitrary paths and older rounded paths are not automatically inferred as primitives.

Direct node edits that change the primitive's boundary make it an ordinary editable path: the corner controls stop applying so they cannot overwrite your node edits. Radius edits are grouped into one undo gesture. Adjacent radii are limited proportionally to avoid overlapping arcs.

## Fills that follow several strokes

Enable **Paint Bucket → Link fill to outline**, then click inside the region. A single closed path keeps its fill directly on the path. Regions bounded by several separate strokes retain references to those strokes in the native project. Existing bucket fills in multiple-stroke regions can be linked by clicking them with this option enabled.

After completing a node, handle or outline edit, the linked region is rebuilt automatically. Color, linked swatch, gradient and drawing order are preserved. The boundary change and rebuilt fill share one undo step; held exposures use the updated drawing too. Recalculation happens after edits, never during playback or viewport rendering.

If the boundary is opened, deleted, ambiguous, or splits into several separate regions, the last valid fill is kept and a status message explains that its outline needs restoring. Close the boundary again to update it. Links reference strokes in the same drawing/layer; they are not a cross-layer live-paint system. Old fills are not automatically linked until you enable the option and paint them again.

## Rejoin a separated fill and outline

Selecting or dragging artwork does not create a link. If a closed outline and its fill have already become separate, select **both** in the same layer, then choose **Properties → Link fill and outline**. The outline is the reference: the fill snaps to its boundary, keeping the fill color/gradient and the outline's stroke style. They become one editable shape and move together. Undo restores the two original objects.

New rectangle primitives keep their fill and stroke together when selected. **Separate fill / stroke** remains available in the node tools for deliberate independent editing. Bucket linking also replaces an existing bucket fill inside a closed outline, without requiring it to match that outline's nodes exactly.

## Shape masks

Select the artwork layer, then click **Shape mask** beside Clipping Mask in the timeline's layer toolbar. Its icon is a circle inside a square with corner points. Choose **Rectangle mask** or **Ellipse mask**. LibreToon adds a linked **Shape mask · [layer name]** row above the artwork and selects it for transformation.

In Properties, **Reveal inside** keeps artwork inside the mask; **Hide inside** cuts a hole through it. **Enable mask** bypasses the modifier without deleting its shapes. **Edit mask** and **Back to artwork** switch between the two linked rows. The green mask boundary is an editing guide and is not exported. Masks are geometric and have hard boundaries; opacity/feathered mattes are not part of this tool.

Resize/move the initial mask with Transform, use **Edit mask nodes** for its Bézier handles, or **Draw new mask** to replace the current drawing with a custom Pen path. **Add rectangle**, **Add ellipse** and **Add path** add shapes to the same mask; their occupied areas combine. Close a custom Pen path to make a filled masking region.

The mask has ordinary drawing exposures and keyframes. Add a key on its row, move or reshape the mask there, and use Motion Tween or Shape Tween between compatible keys. You can move keys, resize exposures and reorder the row through the normal timeline workflow. These changes preserve the original artwork. **Remove mask** releases its targets; undo restores the relationship. Native projects retain the shapes, keys, mode and enabled state. Export uses the same geometric clipping as the scene.

## Oval primitives, half circles and rings

Choose **Ellipse** and open **Oval primitive** in Properties. **Start angle** and **End angle** define a clockwise sector; angles crossing 360° wrap around. Equal start/end angles mean a full oval. **Inner radius** is a percentage of the outer oval: zero gives a solid sector, a positive value gives a ring or ring segment. Presets create a full oval, half circle or ring.

After creating an oval, select it with Selection/Transform to edit the same parameters. Fill and stroke follow the boundary together. Parameters survive native saving/reopening and affine movement/rotation/scaling. Unmodified classic four-node ellipses are also recognized. Direct node edits that change the parametric boundary turn it into an ordinary path; the controls stop applying rather than overwriting your artwork. Parameter changes during one inspector gesture share one undo step.

## Alt-drag quick copy

With **Selection**, **Object Selection** or **Transform**, hold **Alt** before dragging an object's body to duplicate it and move the copy. Multiple selected objects can be copied together. The originals stay in place; the copies remain selected. **Shift** still constrains movement to one axis. A click without movement creates no copy. **Esc** cancels the copying gesture, and one Undo removes the completed copy/move. The clipboard is unchanged.

This shortcut applies to object movement. Alt keeps its existing tangent, transform-handle and rigging meanings in the relevant tools. Symbol copies are new instances of the same source symbol; copied rig bindings get new instance identities. Partial selections copy the selected fragments without cutting the original paths.
