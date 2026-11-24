# LibreToon 1.0.0

Changes since **0.9.0**.

This update expands vector editing and animation workflows, improves GPU viewport presentation, and strengthens project saving and recovery. It also adds transparent exports, more rig-building tools, and an illustrated manual.

## Vector drawing and Bézier editing

- **Four persistent anchor types:** Corner, Smooth, Symmetric and Automatic. Smooth keeps handles aligned with independent lengths; Symmetric also matches their lengths; Corner allows independent handles; Automatic derives tangents from neighboring anchors.
- Node controls are available in **Properties** and a contextual canvas toolbar. Older artwork infers initial handle behavior from its existing geometry.
- **Multi-node editing:** Shift-click or marquee-select anchors, move them together, and use Ctrl/Cmd+A to select anchors on selected paths. An **All handles** option displays every tangent handle.
- **Alt-drag** breaks handle linkage. Double-clicking a segment inserts an anchor while preserving the curve. Escape cancels an active drag.
- **Connect, Merge and Break / Open** support joining compatible stroke endpoints, merging endpoints, splitting open paths and opening closed contours.
- Combined fills and strokes stay together during ordinary node editing. **Separate fill / stroke** provides an explicit separation command; splitting a filled open path preserves its fill separately.
- Added node **alignment, distribution and numerical transforms**, plus a **Preserve curve** option when deleting anchors.
- **Simplify** previews a candidate with adjustable pixel tolerance and corner preservation before Apply or Cancel.
- **Expand Stroke and Offset Path** generate a preview in the background, support cancellation, optionally preserve the original, and commit as one undo step. Generated boundaries are flattened geometry, rather than an exact analytical Bézier equivalent.
- Added an optional **vector brush stabilizer**, with document-pixel distance, a visible lag guide and final-pointer endpoint preservation. It is disabled by default.

## Color, gradients and raster brushes

- **Gradient vector strokes:** linear or radial gradients, editable stops, opacity, rotation and on-canvas positioning. Fill and stroke gradients can be edited independently and saved in native projects.
- Rendered exports preserve their appearance. SVG export represents gradient strokes as filled outline geometry, while experimental XFL export writes supported fill and stroke gradients using native gradient definitions.
- Improved eyedropper sampling of gradient strokes and transparent images, with separate **Color** and **Copy style** modes and an explicit color channel.
- Puppet deformation preserves stroke gradients and linked stroke colors when generating its rendered silhouette.
- Added an editable **pressure-response curve** and graph.
- Raster brushes support **custom PNG/JPEG tips**, asynchronous import, preview and portable preset storage.
- Added **raster alpha lock** to paint within existing opacity without filling transparent pixels.

## Paint Bucket

- Optional **Preview region and gap bridges** displays the candidate boundary and temporary endpoint bridges before applying. Cancel leaves the document unchanged; temporary bridges do not modify the original outlines.
- Multi-frame operations provide progress, cancellation and a persistent per-frame result report, including reasons a frame could not be filled. Select a result to inspect its frame.
- Complex single-frame fills are calculated in the background as well.
- Cached geometry and fill barriers reduce repeated calculations when recoloring or adding fills without changing outlines. Spatial indexing also reduces work when finding connected artwork and gap endpoints.

## Timeline, tweens and undo

- **Continuous tween bars:** motion and shape tweens display their span with diamond drawing keys.
- Drag the first key to relocate an entire tween, including one starting at the first timeline frame. Internal drawing keys can be Shift-selected and moved together while preserving their spacing.
- Tween moves protect occupied destinations and preserve shared boundary drawings for adjacent spans.
- **Rigging · Bone pose** supports moving multiple selected bone and pin keys together, with collision and timeline-bound checks.
- Clicking a bone or existing pin selects it without moving it, inserting a key or adding an undo state. A drag threshold and preserved grab offset prevent accidental jumps.
- Continuous Properties edits to supported object, camera, bone, pin and light settings form **one undo step per gesture**. Escape cancels an active grouped edit.
- Reordering artwork inside a tween preserves the evaluated drawing instead of reverting it to its held source.
- Selection of partial vector components is deferred: selecting and deselecting alone does not split the source geometry. Connected selection and the magic wand use the evaluated drawing.
- Improved **Shape Tween diagnostics** identify incompatible objects and allow inspection at either endpoint.

## Animation curves

- Supported drawings, cameras, bones and pins can use **independent timing curves per channel**, such as X, Y, rotation or scale, depending on the target.
- Channels without an override keep their shared timing. Disabling an override returns that channel to the shared curve.
- The graph displays the current frame's progress and uses a more compact, scrollable window constrained to the canvas.
- Channel data is saved in native projects while retaining compatibility with older curves. These are progress/easing curves between keys, not an absolute-value graph editor.

## Character rigging and Lip Sync

- The rig checklist includes an optional **Build and test** guide with context for the current character, piece, view, state and attachment.
- Contextual actions help draw a view, find the next empty view, define pupil areas or brow points, inspect the skeleton, and test a pose without generating keys.
- Character Symbol layer names support **Shift-range selection** and **Ctrl/Cmd toggling**.
- Added **batch attachment, mirroring across poses/views and prefix renaming**, each as one undo action. Copying an eye and its pupil together remaps their mask relationship to the copied eye.
- **Binding diagnostics** report used bones, pins, unweighted cells and invalid weights. Inspect deformation opens the weight overlay; diagnostics do not silently change the rig.
- Dialogue Lip Sync adds **previous/next mouth-key navigation** and explicit protection against automatic regeneration.
- Phrase regeneration preserves protected/manual corrections, keys outside the phrase, and the original pose after the phrase.
- **Edit pose drawing** opens a mouth's source frame directly. Character mouths open the corresponding layer and a drawn view of that pose; returning restores the previous scene frame.

## GPU viewport and performance

- **Fixed playback corrupting interface state:** offscreen GPU layer preparation now uses its own interface context, preventing scene rendering from replacing the main window's input and viewport state. Regression checks cover preserved UI state and Play/Pause mouse input after GPU composition.
- On compatible devices, the viewport can **present GPU-resident frames directly through the interface renderer**, avoiding a GPU-to-CPU frame download for each presentation. Explicit RAM preview preparation and export retain transfers where needed.
- Geometry preparation runs in the background, and resident frames are reused within a bounded cache. Unsupported cases retain a compatible rendering path.
- Edits preserve cached frames whose evaluated dependencies have not changed, including unaffected camera, light and FX intervals. FPS and scene changes invalidate the appropriate preview identity.
- Added a configurable **GPU image-cache budget** shared by resident frames, layer images and normal maps, with eviction of older cached data while preserving the displayed frame.
- Added a configurable **undo-history memory budget**, with background estimates that account for shared geometry and resources. This is not a global process RAM or VRAM limit.
- Repeated held-exposure synchronization and node alignment avoid redundant work. Picking reuses a bounded cache of curve geometry and lengths.
- Expensive geometry and storage operations share a bounded background job queue. Obsolete results are discarded when the document changes.
- Final export resolution and quality remain independent of viewport preview quality.

Performance depends on the project, adapter and driver. These changes do not guarantee real-time playback on every device. Some symbol and rig edits still invalidate caches conservatively; an already running geometry boolean is not immediately interruptible.

## Saving, recovery and project storage

- **Removed the aggregate 256 MiB native-project limit.** Large projects use streamed ZIP64 storage and loading, avoiding an archive-sized intermediate buffer.
- Older JSON and native containers remain readable. Smaller saves retain the compact compatible container when possible; large version-3 containers require this updated reader.
- Individual resource-decoding and structural validation limits remain. Practical project size still depends on available memory and disk space.
- Save publication uses a stable per-target lock between LibreToon instances, external-file checks and atomic publication. Recovery writes are synchronized before completion.
- Added a visible **Saving / Saved / Unsaved / Changes / Attention** status near project tabs, with copyable project and recovery paths.
- **File → Project storage…** reports embedded resources, compact metadata size, references, unused media and exact duplicates.
- Storage cleanup runs in the background, rechecks for stale results, remaps duplicates and supports undo. It does not delete source files from disk.
- Optional incremental-version retention controls how many versions are kept. **0 preserves all versions**; retention does not remove recovery files.

## Import and export

- **Editable SVG import:** paths, cubic and quadratic curves, arcs, basic shapes, nested transforms, solid styles, common gradients and compound fills.
- Local SVG **use/defs** instances and linked gradient definitions are expanded with bounded recursion and no network downloads. The import dialog reports unsupported or approximated features before insertion.
- **Image sequence import:** numeric filename ordering, start frame, exposure length, fitting and optional project FPS, with background decoding, progress, cancellation and one-step undo.
- Experimental **FLA/XFL export** uses native linear/radial gradients for supported fills and strokes instead of a polygon grid. FLA compatibility remains limited; ActionScript is unsupported.
- **Transparent animated GIF export**, with configurable alpha threshold and disposal behavior that avoids trails between frames. GIF transparency is binary; use PNG or MOV when partial alpha is required.
- **Transparent MOV export** using ProRes 4444, or 4444 XQ at quality 100, including semitransparent edges and export-preset storage. A compatible FFmpeg build is required. MP4, MKV and AVI remain opaque.

## Interface and documentation

- **Layer Objects** adds Shift-range selection, Ctrl/Cmd toggling and contextual Copy, Cut, Paste and drawing-order commands. Right-clicking an already selected object preserves the multi-selection.
- Adaptive timeline and inspector sizing preserve usable canvas space on smaller and scaled displays. Floating preview/controller windows are constrained to the canvas where updated.
- Custom buttons expose accessible names and visible keyboard focus. Additional disabled-state explanations clarify prerequisites for supported operations.
- Added an **illustrated GitHub Pages manual**, with workspace captures, diagrams, animated examples and downloadable practice projects.
- **Five progressive tutorials** cover building an owl, animating a blink, camera/depth work, lighting and a simple rig.
- Manual builds check local links and images. Pull requests validate the documentation; publication remains an explicit workflow action.

## Reliability and verification

- Expanded checks for malformed/truncated archives, CRC errors, duplicate entries, unsafe paths, recursive SVG references, save contention and failed writes.
- Added regression coverage for Bézier editing, selection, tween movement, grouped undo, rig copying, transparent exports, large native projects and GPU presentation/recovery.
- The latest completed development run passed **515 automated Rust tests**, plus packaging and documentation checks. Additional manual tests verified GPU presentation, transparent GIF/MOV files and selected interface layouts.
- Development checks prevent new Clippy warnings from exceeding the recorded baseline; existing warnings are not hidden or claimed to be eliminated.

## Compatibility and remaining work

- Keep an original copy before opening important projects in a newer version. Large native ZIP64 projects may not open in older LibreToon builds.
- SVG filters, CSS classes, text, linked images, masks and clipping are not fully reproduced. Read compatibility reports when exchanging artwork.
- This update does not add rational frame rates, temporal subframe motion blur, full ICC color management, vector stroke textures or absolute-value animation curves.
- Some translations and interface audits remain incomplete. Broader hardware/platform testing is still needed; the complete audit backlog has not been declared finished.
