# LibreToon 0.9.0

Changes since [0.8.0](https://github.com/alvillafm/LibreToon/releases/tag/v0.8.0).

This release focuses on rendering performance, drawing tools, animation controls, and reliability. It also adds editable example projects to help you explore the program.

## GPU rendering, lights, and compositing

- **A GPU compositor powered by WGPU.** Compatible devices now handle artwork rasterization, surface relief, lighting, masks, projected shadows, camera projection, compositing, depth of field, and all 16 existing post-processing effects on the GPU. Intermediate images remain on the device until the completed frame is needed for playback caching or export.
- GPU lighting supports the existing **Point, Directional, Ambient, Rim, Rim Shadow, and Spot** lights, including relief, height guides, painted light masks, and gobos.
- **Drawing and surface caches** reuse prepared artwork and relief where possible, reducing repeated work when moving the camera or adjusting lights. Cache memory is bounded.
- The compositor uses **its own GPU device**, separate from the interface, and initializes it in the background. This prevents interface rendering from sharing the compositor's error scopes and queue synchronization.
- **More consistent previews while editing.** The last processed image remains visible while a replacement is calculated during light and FX parameter changes. Playback can present completed frames even when the animation clock has already advanced.
- **Auto · Progressive viewport quality** first renders at quarter resolution, then refines to half and full resolution while paused. Already prepared full-resolution frames remain available for playback. Full quality remains the default.
- **Full-quality exports** use the GPU compositor when supported, with CPU rendering available for unsupported devices, sizes, or failed passes. The viewport quality setting does not reduce export resolution or quality.
- **GPU recovery:** failed render workers no longer silently restart the same request forever at 0%. GPU failures can recover through CPU rendering. A GPU worker that remains blocked for ten seconds is detached from its cache so subsequent CPU work does not wait behind it. If rendering still fails, the application reports the error.
- **Preferences → Performance** now shows GPU lighting passes and completed GPU frames, alongside viewport quality and cache usage.
- Projected shadows can **Follow light** and use an adjustable **Occluder height**. Existing projects retain their previous shadow settings.

Performance depends on the scene, GPU, and driver. Preparing new drawings can still cause frame-time spikes; this release does not guarantee real-time playback for every project or device. Hardware and software rasterization may produce small differences at antialiased edges. See [performance measurements and reproducible tests](../PERFORMANCE.md).

## Animation and rigging

- **Animation curves:** a visual Bézier easing editor for supported drawing tweens, cameras, bone keys, light-key intervals, and FX-key intervals. Adjust timing without replacing the underlying drawings or poses.
- **Bone-driven Slider Rigs:** connect a rigged slider layer to a bone and define its start and end angles. Bone rotation then selects or blends the layer's drawn states, allowing corrective drawings and pose-dependent changes. Manual control remains available when no bone driver is assigned.
- Bone-driver references are remapped when copying rigs, so copied controls retain their own bindings.
- **Exposure Sheet keyboard controls:** navigate layers and frames with the arrow keys, extend selections with Shift, insert keyframes with Insert, insert blank keys with Shift + Insert, and delete keys with Delete where editing is allowed. Navigation scrolls the selected cell into view.
- Improved validation and loading of saved rig poses and rig preset projects, including Character Symbols without bone bindings.

## Brushes and drawing workflow

- **Independent raster brush textures:** choose the preset's texture, a solid tip, graphite, dry bristle, or grainy chalk, and adjust texture strength separately from the brush preset.
- **Custom raster brush presets** can be saved, recalled, updated, removed, and imported or exported as `.ltbrush` files.
- Raster brush controls include a **stroke preview** that demonstrates pressure changing along the stroke.
- Clearer guidance explains document-pixel brush widths and Pen commands for breaking tangents, finishing a path, and cancelling an edit.

## FX and lighting presets

- Save and reuse named **FX stack presets**, including effect order and settings.
- Import and export FX presets as `.ltfx` files and lighting presets as `.ltlights` files.
- Preset imports validate their contents and enforce file and collection limits before applying them.

## Interface and project management

- **Command search — Ctrl/Cmd + K:** find tools and actions by name, see their shortcuts, and execute them directly. Workflow keywords also help locate actions such as filling gaps and preparing previews.
- **Searchable Preferences** filters categories and settings by their names and related terms. The preferences window also scrolls to accommodate smaller displays.
- Project-tab tooltips show **unsaved changes, the last disk-save age, and recovery information**, making the difference between a saved project and a recovery copy clearer.
- Missing recent projects offer **Locate project…** and **Open file location** actions.
- Recovery errors have a dedicated dialog. Opening the saved project separately preserves the recovery file instead of treating it as a replacement for that file.
- Cancelling a project-thumbnail job stops work between frames when leaving the Home screen.

## New editable examples

- **Night Train:** animated scenery with camera movement, lighting, and bloom.
- **Pip · Wave rig:** a rigged character with a waving animation.
- **Woodland · Focus pull:** layered scenery demonstrating depth and focus changes.
- **Orbit · Live lighting:** animated lights and surface shading.

Open them from **Home → Examples** to explore and edit their layers and settings.

## Packaging and downloads

- The manual GitHub Actions packaging workflow now includes **Linux ARM64 (aarch64)**, with native compilation and regression tests on an ARM64 Ubuntu runner. It produces an ARM64 AppImage, an AppImage tarball that preserves executable permissions, and an ARM64 DEB.
- **One download for all selected platforms:** the Actions summary links to a single ZIP containing the requested packages, organized by OS and architecture, with their security reports and SHA-256 manifests. The assembly checks that all requested architectures are present and their hashes match the passing security reports.

## Project files, imports, and stability

- Native `.ltoon` saving now also reuses identical drawings found in separate, nonconsecutive exposures. This reduces duplication in projects that repeat the same artwork; the size reduction depends on the project.
- Stronger ZIP and FLA validation rejects inconsistent stored-entry sizes and decompressed data that exceeds its declared size. XFL symbol and media imports respect their remaining resource budgets before allocating more data.
- Added regression coverage for importing the bundled FLA examples, saving them as native projects, and reopening them with their drawings preserved. FLA compatibility remains limited to supported animation content; ActionScript is not supported.
- Image and media imports enforce bounded input sizes and decoding allocations. External media helpers have timeouts and output limits to reduce hangs and excessive memory use when a decoder fails or produces unexpected output.
- Portable updates restore files already replaced if a later replacement fails, improving rollback after a partially applied update.
- Additional automated checks cover rendering parity, asynchronous viewport completion, GPU failure recovery, rig drivers, presets, project recovery, imports, and archive limits.

LibreToon remains experimental. Please report rendering, compatibility, and workflow issues with the project and steps needed to reproduce them where possible.
