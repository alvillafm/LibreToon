# Animation workflow corrections after 0.9.0

## Rig interaction and history

- Clicking a bone or existing puppet pin selects it without moving it, inserting a key or adding an undo state. A real drag starts the edit after a small screen-space threshold.
- Bone rotation and pin dragging retain the initial grab offset. Picking the side of a bone or pin no longer snaps it to the pointer.
- Inspector changes to object transforms, bone setup/pose, pin name/position and camera properties are grouped into continuous undo operations. Escape cancels an active grouped edit.
- Selected Bone pose keys move together with preserved offsets. Collision and timeline-bound checks run before any keys are changed.

## Tween timeline

- Motion and shape spans have continuous bars and diamond drawing keys.
- Drag the first key to move an entire tween, including a tween starting at frame 1. Occupied destinations are protected, and shared endpoint drawings remain available to adjacent tweens.
- Shift-select internal keys to move them together using the existing drawing-key selection workflow. Keys represent drawing/object states rather than individual property channels.

## Artwork import

- SVG imports editable curves, basic shapes, transforms, solid styles, common gradients and compound fill geometry. The dialog reports unsupported and approximated features before insertion. The SVG source is unchanged.
- Image sequences import in numeric filename order, with configurable start, exposures, fitting and optional project FPS. Decoding is bounded, asynchronous and cancellable; insertion is atomic and undoable.

See the wiki's Import and Export chapter for compatibility limits and step-by-step instructions. These changes do not add a native Vectorcraft project parser; SVG can be used if that application exports it.
