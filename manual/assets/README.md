# Tutorial asset provenance

The owl, forest and rig artwork was created procedurally for LibreToon's manual. No user documents or desktop captures are included. These assets follow the repository's MIT license.

`projects/*.ltoon` are native, editable projects. The generator validates them, writes them through the normal native encoder, reloads them, and renders the reloaded documents. `images/workspace-*.png` use the actual workspace, canvas and inspector code in a headless egui context; they are not UI mockups. PNG artwork and the source poses for GIFs use the normal export rasterizer. SVGs are explanatory diagrams rather than application screenshots.

## Regenerate

From the repository root, create `target/qa/manual-sandbox`. Run the ignored generator alone, redirecting `LOCALAPPDATA` to that sandbox and `LIBRETOON_MANUAL_OUTPUT` to the absolute `manual/assets` directory:

```powershell
New-Item -ItemType Directory -Force target/qa/manual-sandbox | Out-Null
$env:LOCALAPPDATA = (Resolve-Path target/qa/manual-sandbox).Path
$env:LIBRETOON_MANUAL_OUTPUT = Join-Path (Get-Location) 'manual/assets'
cargo test --locked manual_visuals::generate_manual_visuals -- --ignored --exact --nocapture --test-threads=1
python -m pip install -r scripts/docs/requirements-visuals.txt
python scripts/docs/finish_visuals.py
python -m mkdocs build --strict
```

Use a disposable terminal for these environment variables. Do not run this generator against a personal application-data directory. It opens no application window and sends no mouse or keyboard input.

Images follow the 0.9.0 interface. Recheck labels and layout when that interface changes. The sampled GIFs are teaching previews, not frame-rate or performance measurements.
