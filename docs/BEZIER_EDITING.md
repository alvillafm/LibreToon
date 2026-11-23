# Bezier editing improvements

The Node tool and Pen tool expose Corner, Smooth, Symmetric and Automatic anchors in Properties and a contextual canvas toolbar. Smooth preserves opposite handle length; Symmetric mirrors it; Alt-drag breaks linkage. Anchor modes persist in native documents; legacy geometry infers its initial mode.

Shift-click or a marquee selects several anchors. Dragging moves the group from a frozen gesture snapshot. A selection click does not materialize geometry or create an undo entry. Escape cancels the drag and restores the previous undo/redo state. Properties offers numerical transforms, alignment and distribution. Existing direct handle editing with the Selection tool remains available.

Double-clicking a segment inserts an anchor with an exact de Casteljau split. Connect/Merge joins compatible open strokes on the same vector layer, preserving width stops and rejecting incompatible object settings. Break/Open splits an interior anchor or opens a closed contour. Splitting an open filled path preserves its original fill separately. Ordinary node editing keeps a combined fill and outline together; Separate fill/stroke makes that separation explicit.

Deleting nodes can fit a replacement curve along the surviving endpoint tangents. Simplify runs one bounded background preview worker with a configurable pixel tolerance and corner preservation. It never changes document geometry before Apply; Cancel leaves artwork and history intact. Simplification and deletion are approximations: review the result for important silhouettes.

Geometry operations use the evaluated drawing when editing an intermediate tween frame and materialize the edit through the existing animation workflow. Node selection, drawing overlays and gesture snapshots evaluate each affected layer once rather than once per selected anchor/path where possible. No full-project geometry copies are introduced.

## Validation

Regression coverage includes linked/broken handle behavior, automatic tangents, native and legacy node data, exact curve subdivision, width profiles, topology changes with holes, simplified contour validity, preview cancellation, real mouse multi-selection/group dragging, numeric edits on an evaluated tween, and single-step undo. Visual QA uses the actual editor headlessly, with an isolated preferences directory; no desktop capture is needed.
