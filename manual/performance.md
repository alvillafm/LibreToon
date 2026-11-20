# Performance and Preview

## Work quickly without changing the final render

Use the viewport quality selector above the stage. **1/2**, **1/4** and **1/8** reduce preview resolution. **Full** uses full viewport resolution. **Auto · Progressive** starts with a smaller preview and refines while paused.

These controls affect the canvas preview; exports retain their requested resolution and full quality.

## Prepare playback

Use **Prepare Preview** for the scene or the range you are reviewing. Finished frames are cached. Editing artwork, a light, or an effect can invalidate affected frames, so preparation may be needed again.

A cache holds completed frames; increasing its memory budget does not make the first calculation of a complex frame instantaneous. Configure the preview cache in **Preferences → Performance**, leaving memory available for the operating system and your project.

## GPU and fallback

Supported devices use WGPU for compositing, lighting and effects. **Preferences → Performance** exposes GPU pass and frame counters. Unsupported workloads or a failed GPU worker can fall back to CPU. Performance depends on scene size, layers, effects and the graphics device.

If a frame does not finish, pause playback and record the progress message. Follow [Troubleshooting](troubleshooting.md) rather than repeatedly starting new render requests.
