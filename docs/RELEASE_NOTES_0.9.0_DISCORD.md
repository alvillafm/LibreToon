## LibreToon v0.9.0
Changes since v0.8.0 — performance, tool polish, and reliability.

**Rendering and lights**
- New WGPU compositor for lights, masks, shadows, cameras, blur, and all 16 existing FX.
- Reuses prepared artwork and surface relief to reduce repeated work.
- Separate GPU devices for the interface and compositor, plus recovery from failed or stalled preview workers.
- Keeps processed previews visible during light/FX edits.
- Optional **Auto · Progressive** quality: 1/4 → 1/2 → Full while paused. Exports retain full quality.
- Projected shadows can follow lights, with adjustable occluder height.

**Drawing and animation**
- Raster brush textures, texture strength, custom presets, and pressure previews.
- Visual Bézier easing editor for tweens, cameras, bones, lights, and FX.
- **Bone-driven Slider Rigs** for corrective drawings and pose-dependent changes.
- Exposure Sheet keyboard navigation and keyframe editing.
- Import/export brush, FX, and lighting presets.

**Workflow and stability**
- Linux ARM64 AppImage/DEB builds and one combined ZIP download from Actions.
- **Ctrl/Cmd + K** command search and searchable Preferences.
- Clearer save/recovery information and actions to locate missing projects.
- Less duplicated artwork in native files, stronger FLA/ZIP validation, bounded media decoding, and safer update rollback.
- New editable examples: **Night Train**, **Pip · Wave rig**, **Woodland · Focus pull**, and **Orbit · Live lighting**.

Performance varies by scene and hardware; new drawings can still cause frame-time spikes. LibreToon remains experimental. Feedback and reproducible bug reports are welcome!
