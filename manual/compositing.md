# Cameras, Lights and FX

Create a camera and set keys for position, rotation, zoom, or focus. Use **Layer Depth** to assign distances for parallax and depth of field.

Add a light from the bulb menu in the timeline. Choose its receivers, set intensity and color, and adjust direction or position on the stage. Add an adjustment layer with **Fx** and choose effects in **Properties**. Layers below it receive the effects by default.

Use the stage toolbar to toggle the FX or light preview and choose viewport quality. **Prepare Preview** caches a scene, a selected frame range, or an animation tag for playback.

The WGPU renderer keeps geometry planes, relief, lights, masks, shadows and post-processing on the GPU until the completed frame is ready. **Preferences → Performance** shows the number of GPU light passes and completed GPU frames. Cached geometry and relief are reused when possible. Unsupported devices or image sizes use the CPU renderer.

**Full** retains full viewport resolution. The optional **Auto · Progressive** mode first displays a quarter-resolution update, then refines it to half and full resolution while paused. Already prepared full-resolution frames remain available during playback. Parameter edits keep the previous processed preview visible while its replacement is calculated.

Exports always use the requested output resolution and full quality. The same GPU pipeline is used when supported; the viewport quality selector does not lower export quality.

Projected shadows can **Follow light**, with an adjustable occluder height. Choose the caster and receiver layers to control which artwork casts and receives the shadow. Existing projects retain their original shadow settings.