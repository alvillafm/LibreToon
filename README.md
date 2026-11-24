<p align="center"><img src="https://i.imgur.com/RNhNwtU.png" width="250" alt="LibreToon logo"></p>

<p align="center">
  <a href="https://discord.gg/SVFTGRrzB">
    <img src="https://shieldcn.dev/discord/members/SVFTGRrzB.svg?variant=branded" alt="Discord Members">
  </a>
</p>

# LibreToon

### Made by an animator, for animators.

<p align="center">
  <a href="https://github.com/alvillafm/LibreToon/releases">
    <img src="https://img.shields.io/github/downloads/alvillafm/LibreToon/total" alt="Downloads">
  </a>
  <a href="https://github.com/alvillafm/LibreToon/stargazers">
    <img src="https://img.shields.io/github/stars/alvillafm/LibreToon" alt="Stars">
  </a>
</p>

**LibreToon 1.0.0** is a free, open-source 2D animation application built from scratch in **Rust** for **Windows, macOS and Linux**. Draw with vectors or pixels, animate frame by frame, build reusable character rigs, and composite scenes with cameras, lights and effects in one workspace.

<p align="center"><img src="https://i.imgur.com/0qjDPEz.png" width="1050" alt="LibreToon interface"></p>

**Built with AI. Driven by an animator.** LibreToon is an independent project led by an animator and developed with AI assistance.

> LibreToon is our independent reimagining of what an “Animate 2” could have been: a drawing-first animation workflow with its own Rust implementation. Projects use the native `.ltoon` format. FLA import and export exchange supported artwork and animation; compatibility reports identify omissions and approximations. ActionScript is not supported.

[Download LibreToon](https://github.com/alvillafm/LibreToon/releases) · [Read the Wiki](https://alvillafm.github.io/LibreToon/) · [Start the tutorials](manual/learn/index.md) · [1.0.0 release notes](docs/RELEASE_NOTES_1.0.0.md)

## Features

### Drawing tools and references

- **Classic Brush, Advanced Vector Brush and Raster Brush**, with pressure-sensitive drawing and erasing, including compatible stylus eraser ends.
- **Vector stabilization and smoothing**, document-pixel stroke widths, variable-width profiles, and an editable pressure-response curve.
- **Raster brush textures and custom tips:** hardness, opacity, texture strength, PNG/JPEG tip import, alpha lock, and reusable presets with `.ltbrush` import/export.
- **Pen, Node, Line and primitive shape tools**, plus text, erasing and Vector Trim.
- **Fill, Shade and Light brushes** for painting and toon shading, plus canvas pan, zoom and rotation.
- **Independent fill and stroke selection**, object selection, lasso/polygon selection, magic wand, connected-contour selection and clipping masks. Partial selection leaves the source intact until an edit is applied.
- **Symmetry, mirror, perspective guides, rulers, grids and snapping** for drawing and placement.
- **RefTool** for floating visual references and color sampling, plus a **poseable 3D mannequin** with joint rotation controls.
- **Tablet Diagnostics** to inspect pressure, input source and stylus state.

### Bézier and vector editing

- **Corner, Smooth, Symmetric and Automatic anchors**, saved with the artwork. Keep tangent handles aligned, mirror their lengths, or break their linkage for sharp corners.
- **Multi-node selection and editing:** Shift-click, marquee selection, group dragging, alignment, distribution, and numerical position/rotation/scale controls.
- Insert anchors while preserving the curve, delete nodes with optional curve preservation, and show all handles.
- **Connect, Merge and Break / Open** paths, with explicit separation of combined fills and strokes when needed.
- **Simplify, Expand Stroke and Offset Path** with candidate previews, Apply/Cancel and undo. Expansion and offset produce flattened vector boundaries.

### Color, gradients and painting

- A dedicated **Color panel** with Fill/Stroke targeting, color picker, eyedropper, editable project palettes, themed palette presets, linked colors and favourites.
- **Linked vector colors** recolor associated artwork when their swatch changes.
- **Independent fill and stroke opacity**, plus linear/radial gradients for both channels, editable stops and on-canvas gradient placement.
- Eyedropper modes for sampling a **color** or copying a **style**, including sampling imported images and references.
- **Paint Bucket gap-closing modes**, optional region/temporary-bridge preview, and confirmation before applying.
- **Multi-Frame Fill:** paint the current drawing and additional keyframes, or process selected keyframes, with background calculation, progress, cancellation and per-frame results.

### Animation and timeline

- **Frame-by-frame animation, Motion Tweens and Shape Tweens**, with editable timing and compatibility diagnostics for shape interpolation.
- **Continuous tween bars and diamond keys:** move entire spans, drag multiple internal drawing keys, and resize held exposures.
- **Onion skinning with configurable ranges**, **Shift & Trace** for temporary drawing alignment, and transforms across selected keyframes.
- An **editable Exposure Sheet** with layers as columns, keyboard navigation, key/blank-key editing, drawing classifications and timing on ones, twos or threes.
- **Visual animation curves**, including independent progress/easing channels for supported drawing, camera, bone and pin transforms.
- **Graphic symbols** with loop, play-once or single-frame playback, editable instance pivots, and cross-project copying of frames and their resources.
- **Animation Manager** for reusable clips, folders, trimming, speed changes, reverse, looping and ping-pong playback.
- Animation tags/ranges, dedicated rig-controller tracks, and grouped undo for continuous property edits.

### Character rigging

- **Bones and Puppet pins**, on-stage posing, editable rest anchors, rigid or weighted binding, weight visualization, IK constraints and reusable rig poses.
- **Character Symbols**, with or without bones, organize artwork across eight turnaround views. Define view-specific drawings and skeleton setups, and interpolate compatible artwork between drawn views.
- **Rigged Eye layers** with Open, Half closed and Closed states, vertical shape interpolation for eyelids, and horizontal interpolation across character views.
- **Pupil controls** with per-view travel areas, eyelid clipping, and linked or independent gaze controls.
- **Brow controls** with three editable deformation points per eyebrow.
- **Hand layers** with pose states, turnarounds and wrist alignment, plus generic **Slider, Switch and On/Off rigs**. Slider rigs can also be driven by bone angles.
- Floating **Character Rig Controllers**, organized into face, hand and other controls.
- A contextual **rig checklist and Build and test guide**, attachment controls, next-empty-view navigation, and pose testing without generating keys.
- Batch attachment, mirroring across poses/views and renaming; rig copying, preset import/export, and binding diagnostics.

### Dialogue and Lip Sync

- **Manual and automatic Lip Sync** using custom mouth poses, with mouth keys editable on the timeline.
- **Character mouth layers** reuse phoneme keys across turnaround views while keeping view-specific mouth drawings.
- A **Dialogue Lip Sync editor** with waveform, scrubbing, phrase preview, previous/next-key navigation and phrase-range generation.
- Protect manual corrections from regeneration, preserve keys outside the phrase, and open a pose's source drawing directly for editing.

### Scenes, cameras and media

- **Multiple scenes and cameras**, camera motion and focus animation, layer depth, parallax and depth of field.
- Layer folders and composition groups, drawing-order controls, and **Normal, Switch and Solo object folders**.
- **Layer Objects** for inspecting artwork and symbols, range/multiple selection, clipboard actions and ordering.
- A **Media Library** for embedded assets, usage inspection and replacement of compatible media.
- Dedicated **audio and video tracks** with placement and trimming, image clipboard paste, and numbered image-sequence import.

### Lighting and post-processing

- **Point, Directional, Ambient, Rim, Rim Shadow and Spot lights**, with receiving-layer controls, surface relief and painted light masks.
- **Projected shadows**, caster/receiver selection, light-following direction and occluder height.
- Spotlight shaping and **gobos**, including blinds, windows, leaves and silhouettes from library symbols.
- **Adjustment layers** with editable timeline spans, keyframes, effect ordering, and targets for artwork below, a layer or a folder.
- **16 post-processing effects:** Hue/Saturation, Color Levels, Bézier Color Curves, Bloom/Glow, Vignette, Color Integration, Gaussian Blur, Lens Blur, Sharpen, Exposure/Contrast, Posterize, Outline, Inner Shadow, Directional Blur, Grain/Paper Texture and RGB Separation.
- Reusable **FX stack and lighting presets**, including `.ltfx` and `.ltlights` import/export.

### Viewport and performance

- **WGPU-accelerated lighting and compositing** on compatible hardware, with direct presentation of GPU-resident viewport frames and a compatible fallback rendering path.
- Background geometry preparation, reusable drawing/surface caches and selective preview invalidation to reduce repeated work.
- **Full, 1/2, 1/4 and 1/8 viewport quality**, plus **Auto · Progressive** refinement while paused. Viewport quality does not reduce final export quality.
- **Prepare preview** for scenes, selected ranges or animation tags, with progress reporting and cached playback.
- Configurable **RAM preview cache, GPU image-cache and undo-history budgets** in Preferences. These control their respective caches/estimates rather than total process memory.

Performance varies with scene complexity, hardware and drivers. See the [measurements and reproducible benchmarks](PERFORMANCE.md).

### Project saving and storage

- Compact native **`.ltoon` projects**, shared embedded resources and streamed storage for large projects, without the former aggregate 256 MiB project ceiling.
- Configurable autosave/recovery and incremental versions, optional version retention, and visible **Saving / Saved / Changes / Attention** status with project and recovery paths.
- Background recovery writing, save-conflict detection, atomic publication and coordination between LibreToon instances saving to the same path.
- **Project storage report** with resource sizes, references, unused media and exact duplicates, plus background, undoable cleanup.
- Compatibility with earlier native project formats. Large ZIP64 projects require a reader that supports the newer container.

### Import and export

| Format or asset | Support |
| --- | --- |
| **LibreToon `.ltoon`** | Native editable projects, reusable rigs and practice projects. |
| **Adobe Animate `.fla` / XFL** | Import/export of supported artwork and animation, including supported fill/stroke gradients, with compatibility reporting. |
| **SVG** | Import editable vector paths, shapes, transforms, common gradients and local clones; export vector artwork. |
| **Images and image sequences** | Import stills or numerically ordered sequences with configurable placement, exposure and optional project FPS. |
| **Audio and video** | Import media for dedicated timeline tracks. |
| **PNG sequences and sprite sheets** | Export rendered frames, including PNG-sequence handoff to Toon Boom Studio; configure the sprite-sheet layout. |
| **Animated GIF** | Export with optional transparent background and alpha threshold. |
| **MP4, MKV, AVI and MOV** | Video export, with optional ProRes 4444/4444 XQ transparency for MOV. |

Video and GIF encoding use an external **FFmpeg** installation; the app offers setup for supported platforms. GIF has binary transparency; MOV ProRes 4444 and PNG can preserve partial alpha. Opaque background layers remain visible in transparent exports.

FLA and SVG support covers a subset of their formats. Read the import reports: ActionScript is unsupported, and SVG filters, CSS classes, text, linked images, masks and clipping are not fully reproduced. See the [import/export guide](manual/files.md) for details.

Batch-export scenes or animation tags and save reusable export presets.

### Workspace and platforms

- **Windows, macOS and Linux**, including Intel/Apple Silicon macOS packages and x64/ARM64 Linux AppImage and DEB packages.
- **Command search — Ctrl/Cmd+K**, searchable Preferences, configurable keyboard shortcuts and a contextual **Command Helper**.
- Light/dark themes, multilingual interface options, resizable panels and floating character controls.
- Home project rows with horizontal scrolling, recent/in-progress projects, and editable examples for animation, rigging, lighting and depth/focus.
- **In-app update checking** and supported installation-update workflows.

## Learn LibreToon

Browse the [illustrated Wiki](https://alvillafm.github.io/LibreToon/) or the [manual chapters](manual/index.md). The [five progressive tutorials](manual/learn/index.md) include captures, diagrams, animated examples and downloadable projects:

1. [Draw an owl](manual/learn/01-owl.md).
2. [Animate a blink](manual/learn/02-blink.md).
3. [Build a forest with depth and camera movement](manual/learn/03-forest.md).
4. [Light and composite a scene](manual/learn/04-lighting.md).
5. [Build a simple character rig](manual/learn/05-rig.md).

For changes since 0.9.0, read the [1.0.0 release notes](docs/RELEASE_NOTES_1.0.0.md). Developers can also consult the [capability and compatibility matrix](docs/CAPABILITIES.md) and [audit implementation status](docs/AUDIT_IMPLEMENTATION.md).

## Contributions & Feedback

Bug reports, suggestions and contributions are welcome. Open an [issue](https://github.com/alvillafm/LibreToon/issues), submit a pull request, or join the [Discord community](https://discord.gg/SVFTGRrzB).

LibreToon's own code is **MIT-licensed**. See the [license](LICENSE), [third-party notices](assets/THIRD_PARTY_NOTICES.md) and [asset provenance](assets/ASSET_PROVENANCE.md).
