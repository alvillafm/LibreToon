<p align="center"><img src="https://i.imgur.com/Y0voErz.png" align="center" width="200px"></p>

# LibreToon

### Made by an animator, for animators.

**LibreToon** is a free, open-source 2D animation software built entirely from scratch in Rust, without forks or existing application codebases.

Inspired by the traditional workflow of Adobe Animate, LibreToon aims to combine familiar animation tools with modern features, greater flexibility, and creative freedom.

<p align="center"><img src="https://i.imgur.com/N5REuDb.jpeg" width="1000px"></p>

### Built with AI. Driven by an animator.

LibreToon is a **vibe-coded project**, created by an animator. The idea is simple: instead of waiting for animation software to introduce the tools I've always wanted, I decided to build them myself, using AI-assisted coding to turn those ideas into reality.

> **A note about Adobe Animate:** LibreToon follows a familiar animation-oriented workflow and supports `.FLA` files, but it is not an Adobe Animate clone. It focuses on 2D animation rather than interactive content or application development. ActionScript is not supported.

---

## Features

### Copy frames between projects
Open both projects in tabs. Select a timeline range with Shift-click, choose **Copy frames**, then select the destination layer/frame and choose **Paste frames**. Ctrl+C/Ctrl+V (Cmd on macOS) use the frame clipboard while the timeline is active. Pasting adds the copied artwork to the existing drawing layer; it does not create a layer or erase its previous contents. Frame counts are preserved even when project FPS differ.

Symbols, images and animation dependencies are copied with new IDs. Project-level rigs are kept in a self-contained editable symbol; clipped artwork is sampled as editable geometry. Intersecting motion tweens in the destination are sampled to preserve their appearance, including outside the pasted range. Undo restores the entire paste and its imported resources. The frame clipboard is shared by tabs within the same LibreToon instance.

### Reusable animation clips
The **Animation** tab beside Library opens Animation Manager. Select a timeline range (Shift-click to extend it), then use **Create Animation Clip…** to choose layers, include their rig, and group the clip under a named resource. Characters, props, effects and frame-by-frame drawings use the same workflow; the original animation stays in place.

Animation Manager has its own colored folder tree. Use the folder button to create nested folders; rename and color them from their context menu or selected-folder properties. Drag animations or folders onto a folder to move them inside, or near a row edge to reorder them. Drop onto **Animations** or the empty area below the tree to move an item to the root. Deleting a folder moves its contents up one level without deleting its animations. Deleting an animation resource that is already in use asks for confirmation before removing its placed clips, including those in other scenes or symbols. Empty affected animation tracks are removed; Undo restores the deletion.

Drag a clip onto the stage to place an instance, or onto an animation track to sequence it. Move the block to change its start; drag an edge to trim, or hold Shift while dragging an edge to change playback speed. Right-click for normal looping, ping-pong playback, reverse, speed, duplication, editing and deletion. Trimming preserves the playback phase; stretching retimes the retained source range. Changing speed resizes the block to preserve its visible source window and is rejected if it would overlap another clip. Shift is latched when an edge drag begins, so releasing Shift before the mouse does not turn a stretch into a trim. Clips on one track cannot overlap. Deleting the final clip removes its track; Undo restores it. Layer order, visibility and locking also apply to animation tracks.

Animation tracks have a trash button in their header and a **Delete animation track** context action; deleting a track keeps its reusable resources. The rigging header also has a trash button and **Delete rigging track**. This removes bones, weights, pins and rig keys in the current scene/symbol, keeping the underlying artwork in its unrigged form. Undo restores the rig.

Double-click opens the shared source with the captured frame range selected. Source drawings, bones, puppet data and keyframes stay editable, and edits update its instances. **Duplicate animation** makes an independent source for variations. Native save/recovery and LibreToon FLA metadata retain clips; the visible XFL preview is baked for interoperability. This does not provide automatic motion retargeting between different skeletons.

LibreToon combines traditional animation tools with features that go beyond what Adobe Animate currently offers.

### Drawing & Painting
- **Classic Brush** — A familiar, traditional drawing experience inspired by Adobe Animate.
- **Advanced Vector Brush** — Fully customizable strokes with editable curves, giving you much more control than standard brush presets.
- **Symmetry & Mirror Tools** — Draw symmetrically without additional workarounds.
- **Perspective Tool** — Create backgrounds and environments with perspective guides.
- **Toon Shading & Lighting Brushes** — Dedicated tools for lighting and shading your artwork.
- **Rulers & Guides** — For precise drawing and alignment.
- **Independent Fill & Stroke Opacity** — Adjust both separately.
- **Object Selection Tool** — Select and manipulate individual objects.

### Animation & Scenes
- **Scene Manager** — Rename, organize, and reorder scenes with drag-and-drop.
- **Symbol Library** — Graphic symbols with looping, play-once, and single-frame playback modes.
- **Clipping Masks** — Yes, actual clipping masks!
- **Multiple Cameras** — Add as many cameras as you need and switch between shots without relying on camera keyframes.
- **Bone Rigging** — Inspired by Unity's bone tools, with multiple symbols per bone and adjustable influence.
- **Puppet Tool** — Inspired by After Effects, designed to preserve visual quality when deforming artwork.
- **3D Poseable Mannequin** — An adjustable 3D reference figure with poseable joints.

### Export
Export your animations to **GIF, MP4, AVI, MOV, MKV**, and other formats.

---

## What's Coming Next?

There is still a lot I want to add to LibreToon!

- **Concept Art Tool** — Manage references, pick colors directly from images, and more.
- **Lip Sync Tool** — Support for phonemes, sounds, and multiple languages.
- **Post-Processing Effects** — Additional visual effects and finishing tools.
- **3D Transform Tool** — More possibilities for manipulating artwork in 3D space.
- **Dedicated Video & Audio Tracks** — Work with video and sound in separate tracks instead of forcing everything into the traditional animation timeline.
- **Improved Scene Editing** — More control over individual scenes and their contents.
- **And much more!**

---

## Experimental Release — Expect Bugs!

**LibreToon is still in a very early, experimental stage of development.**

Some features may be incomplete, unstable, or behave unexpectedly. You will probably encounter bugs, crashes, and things that simply don't work as intended yet.

This is an ongoing project, not a finished product. Please keep that in mind when trying it out!

## Contributions & Feedback

LibreToon is an open-source project, and **everyone is welcome to contribute**.

Whether you're a developer, animator, or just someone interested in better 2D animation tools, your feedback is valuable.

- Report bugs and unexpected behavior.
- Suggest new features or improvements.
- Contribute code and help improve existing tools.
- Share your experience and ideas.

Feel free to open an **Issue** or submit a **Pull Request** on GitHub.

**LibreToon is just getting started. Let's see how far we can take it!**

### Exposure resizing and alignment

Drag the right edge of an exposure to extend or shorten it; no double-click is required. Dragging its body still moves the keyframe. The final edge changes scene duration and protects existing animation in other tracks.

**View** contains rulers, clear guides, a configurable grid and independent snapping switches for guides, stage axes, object edges and grid intersections. Hold **Alt** to bypass snapping. Grid size, subdivisions, origin, color, opacity and line/dot style are configurable. The grid and alignment indicators are editor overlays and are not exported. Shape tools show a crosshair and a drawing hint.

### In-app updates and uninstalling

**Help > Check for updates** downloads the package matching the current installation, verifies its GitHub SHA-256 digest, asks to save dirty documents, then closes LibreToon. A detached helper shows installation progress, updates the same location and reopens the application. Closing or postponing the download window defers installation. Projects, preferences and the separately downloaded FFmpeg stay outside the update payload.

Windows installed copies use the Inno Setup installer with the same AppId and explicit existing directory. Portable copies use the ZIP and update their current folder; they do not create another installed application. Windows installers register **LibreToon** in Installed apps and add an **Uninstall LibreToon** Start menu shortcut. Portable copies have no Windows uninstall registration.

On macOS the existing .app bundle is replaced (authorization may be requested for Applications). Move the app out of a mounted DMG before updating. Linux AppImages are replaced at their existing path; DEB installations use apt with PolicyKit authorization. Unsupported development/unmanaged copies are not relocated or converted silently. The helper keeps rollback backups for portable/bundle replacements and preserves the downloaded package and diagnostics on failure. Windows installer and apt failures follow those installers’ own recovery behavior.

The new updater must first be installed as part of a new build; earlier LibreToon executables retain their previous update behavior. Native macOS/Linux installation and authorization require verification on those systems.
