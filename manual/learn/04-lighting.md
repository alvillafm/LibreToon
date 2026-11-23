# 4 · Light the Night

**Result:** a cool night background and a warm light on the owl. **Start:** the forest shot, or a static owl with a separate background layer. Save as `Owl-04-Lighting.ltoon`.

### Compare the same frame

![Forest owl before the warm rim light](../assets/images/03-forest.png)

*Before · the scene without the extra light.*

![Forest owl after the warm rim light is applied](../assets/images/04-lighting.png)

*After · the same scene and camera with a warm rim light. Both images are actual LibreToon renders. The light targets the body, wing and eyes; the background keeps its original colors.*

[Download the lighting project](../assets/projects/04-lighting.ltoon). Toggle the Warm rim light layer to compare the result inside the app. This example demonstrates the light; the color-curve exercise below is an additional step for you to try.

## Add one light at a time

1. Enable the light preview in the stage toolbar.
2. Use the timeline's bulb menu to add a light. Start with **Rim** for an edge highlight, or **Point** for a localized light.
3. In Properties, choose the owl layers as receivers. Use a warm color and begin with a modest intensity.
4. Move the light or adjust its direction while watching the owl. Compare the result with the preview toggle off and on.
5. Add a cool ambient contribution only if the shadows become too dark. Judge each light separately before combining them.

## Add a restrained effect

1. Add an **Fx** adjustment layer above the affected artwork.
2. Enable FX preview. Add a color curve and make a small change to the scene's tone.
3. Check a frame at the beginning and end of your camera move.
4. Use **Prepare Preview** for the short range. If interaction is slow, try half-resolution viewport quality.
5. Export a short test and inspect it outside LibreToon. Preview quality does not reduce final export quality.

!!! success "Checkpoint"
    Switching lights or FX off makes a visible, understandable difference. The owl remains readable and the exported framing matches your intended camera shot.

!!! tip "If nothing changes"
    Check the preview toggles, light receivers, layer visibility, adjustment-layer order and active frame. Increase one parameter at a time.

**Next:** [Rig a Waving Wing](05-rig.md).
