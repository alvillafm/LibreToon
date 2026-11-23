# Drawing and Color

- **Vector layers (V)** store paths and editable nodes. Choose **Pen** or **Node** to edit a selected path. Alt-click an anchor to convert it to a corner. Alt-drag a handle to adjust it independently.
- **Raster layers (R)** store pixels. Choose a raster brush preset and adjust hardness and opacity in **Properties**.
- Open **Color** and choose **Fill** or **Stroke** before picking a swatch. Edit a linked swatch to recolor its associated vector artwork. Use the eyedropper to sample a reference image loaded with **Tools → RefTool**.
- For several similar drawings, use the **Paint Bucket** and choose a multi-frame mode in **Properties**. Click inside the region to fill. Check the results on each drawing before proceeding.

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
