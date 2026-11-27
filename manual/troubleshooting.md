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

## Windows crashes before the window opens

Version 1.0.1 uses DirectX 12 for Windows startup by default, avoiding automatic Vulkan driver enumeration. This addresses the startup path implicated by a supplied Intel `igvk64.dll` crash report. It is a compatibility mitigation; the affected mini PC still needs to be tested.

An explicit `WGPU_BACKEND` environment setting overrides the default. Remove an old Vulkan override when testing the default. For diagnostic use, `WGPU_BACKEND=dx12` selects DirectX 12 explicitly. If window initialization returns an error, LibreToon displays it and tries to write `LibreToon-startup-error.log` in the system temporary directory. A native driver access violation can terminate the process before this handler runs; its Windows crash report remains useful.
