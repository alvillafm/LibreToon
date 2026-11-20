# LibreToon 0.8.0

Changes since 0.7.0.

## Character rigging

- **Create Character** turns selected vector artwork or symbols into an editable Character Symbol, with or without bones. Find it in the stage context menu and in the floating toolbar when working with Bones. Use **Edit Rig** to reopen the setup.
- **Character Symbol** has eight drawing views for turnarounds. Body and head turns can be keyed on the stage. Use **Artwork**, **Skeleton**, and **Test Pose** to draw, edit joints by view, or preview a layer without writing animation keys.
- Reorder rigged layers by dragging them. Copy artwork, bones, or guides separately. **Copy View Setup** can fill empty cells across a whole view. Onion skinning has adjustable ranges in the view grid.
- The movable **Rig Checklist** shows the current view’s attachments, guides, missing drawings, and optional Shape Tween checks.
- Bind artwork rigidly to one bone, lock a bone’s weights during auto-weighting, or display areas with no assigned weight. IK chains can stop at a chosen bone and use joint angle limits. Removing a binding, clearing pose keys, and removing the whole rig are separate actions. Removing the whole rig asks for confirmation.

## Face, hands, and custom controls

- **Character Rig Controllers** groups the controls into **Face Controls**, **Hand Controls**, and **Other Controls**.
- **Eyes** have Open, Half closed, and Closed drawings in each view. Vertical Shape Tween blends eyelid states. Horizontal Shape Tween blends between views at the current eyelid setting. Compatible vertical tweens can be enabled across drawn views in one operation.
- **Pupils** have a travel area per view and can be clipped to the animated eye opening. Animate them independently or use **Move together** to key the same gaze direction on all pupils.
- **Brows** have three draggable control points. Their deformation preserves smooth curve joins when you raise or lower an end, and the points travel with the drawing during a turnaround.
- **Mirror layer** mirrors artwork while keeping the view columns. **Mirror to opposite views** also swaps left and right views. **Duplicate Side** mirrors the matching eye, pupil, and brow together and reconnects the copied eye mask.
- **Hands** have named poses, a wrist point for alignment, an independent turnaround, and pose thumbnails in the scene. Mirror the rig to the other wrist or enable optional tweening between compatible hand poses.
- **Slider Rig** blends compatible drawn states. **On/Off Rig** switches between two states and can hide artwork when Off is empty. **Switch Rig** selects a named drawing, such as an expression or an accessory.

## Lip sync during turnarounds

- **Add Rigged Layer → Lip Sync** copies a library symbol’s mouth poses into the character’s front view. Expand the layer to edit each pose by view, copy a single pose, or fill a view’s empty cells in one operation.
- Set keys manually in Properties or generate them from audio in **Dialogue Lip Sync**. The same key plays the corresponding drawing as the character turns. Missing angles use the nearest drawing of the same pose.
- Enable horizontal tweening for individual mouth poses when their drawings are compatible. Removing a rigged mouth layer cleans up its dialogue routes when you leave Character Symbol.

## Timeline and reusable rigs

- Expand **Rigging** to edit control tracks. Select ranges, move or duplicate keys, and apply easing to continuous channels. Filter controls by name.
- Save reusable body poses, expressions, hand poses, and full poses in **Animation → Rig poses**.
- **Copy Character Rig** and **Paste Character Rig** move bound artwork and rig data between projects as an independent symbol. **Save Rig Preset** and **Import Rig Preset** make that rig reusable as a local project file.

## Drawing, symbols, and documentation

- **Shape Tween** interpolates compatible vector drawings between timeline keyframes and is also available for character view transitions.
- **Link fill to outline** in Paint Bucket gives the fill and its closed contour shared geometry, so the fill follows node edits. The selector keeps linked outlines and fills together.
- **Convert to Symbol** opens a dialog for naming the symbol and choosing Loop, Play once, Single frame, and whether playback follows the timeline.
- Open the new **User Manual** from Help for guides to drawing, animation, rigging, compositing, and reference workflows.

