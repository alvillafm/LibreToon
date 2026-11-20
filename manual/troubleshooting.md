# Troubleshooting

## A save failed

Keep the document open. Try **Save As** to a new local filename in a writable folder. Check free disk space and record the full error. Do not delete the old project, recovery files or backups. See [Saving and Recovery](saving.md).

## The stage shows a different result from the export

Check the active camera, frame range, layer visibility and FX/light preview toggles. Export a single representative frame at the requested size and compare it at 100% zoom. Viewport preview quality and external video-player scaling can change what you see.

## Lighting is slow or progress seems stuck

Pause on one frame. Try a lower viewport quality and a small example project. Record whether the issue occurs with FX off, lights off, or both. Check **Preferences → Performance** and include the GPU counters and exact progress text in your report.

## A pupil or eye disappears

Check the active character view and eye state, layer order, attachment, and pupil **Clip to eye** setting. Use the rig checklist for that view. Test a copied view before building every angle. See [Rigging](rigging.md).

## A shape tween switches instead of blending

Start with a copied drawing and reshape its existing nodes. Deleting and redrawing paths can make their topology incompatible. Check the horizontal or vertical tween setting for the intended cells.

## A shortcut does nothing

Finish typing in any text field, return focus to the stage, and inspect **Program Preferences → Keyboard Shortcuts**. Use **Command Helper** for tool-specific modifiers.

## Report a useful bug

Include the app version, operating system, GPU if relevant, exact steps, expected result, actual result and any error text. A small `.ltoon` reproduction is especially useful. Remove private media before sharing. Submit it through [GitHub Issues](https://github.com/alvillafm/LibreToon/issues).
