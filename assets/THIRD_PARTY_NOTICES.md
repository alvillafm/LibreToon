# LibreToon — third-party notices

Documentation updated 2 November 2026. LibreToon's root LICENSE declares MIT for the project. That declaration does not replace the licenses of bundled third-party components, fonts or artwork.

This is a partial attribution record, not a compliance certificate.

## Rust dependencies

The ten direct dependencies in Cargo.toml declare MIT or MIT OR Apache-2.0 in their locally available manifests. Transitive dependencies include additional terms.

See [licenses/RUST-DEPENDENCIES.md](licenses/RUST-DEPENDENCIES.md) for package/version declarations and evidence status, and [licenses/RUST-NOTICES.txt](licenses/RUST-NOTICES.txt) for original license and notice texts from checksum-verified registry archives and pinned upstream revisions.

**All 452 external locked package archives have now been checked against Cargo.lock, including the previously pending 226.** Their license declarations are recorded. Ten of the eleven notice gaps have been closed using original release/source notices. dispatch is documented using its original MIT/author declaration and a labelled canonical MIT template; its original copyright notice remains unavailable. See the inventory. Manifest and checksum verification are not a complete compliance certificate. AND requirements, including font and Unicode terms, remain mandatory.

## Fonts

- Noto Sans CJK: SIL OFL 1.1; the inspected local face declares © 2014-2021 Adobe (http://www.adobe.com/). See [fonts/README.md](fonts/README.md), [font copyright evidence](fonts/COPYRIGHTS.md), and [OFL text](fonts/OFL-NotoSansCJK.txt).
- Fonts supplied with epaint_default_fonts 0.37.0: [notices and license texts](licenses/fonts/README.md), including Hack/Bitstream Vera, Ubuntu Font Licence, NotoEmoji OFL, and the MIT-licensed emoji/icon font and subset.
- The font metadata/candidate files were inspected, but the exact subset embedded in every platform/feature combination has not been established.
- User-selected artwork fonts are not bundled by this notice record and their individual terms have not been audited.

## Optional FFmpeg download

LibreToon installers and portable packages no longer contain an FFmpeg executable or
its corresponding-source bundle. Video/GIF export can use a user-selected executable
or, with explicit consent, download an immutable Imageio/PyPI wheel directly to that
user's application-data folder. No Python runtime or library is installed.
The archive size, HTTPS URL and SHA-256 are pinned in ffmpeg-downloads.json.

FFmpeg remains a separate process and retains its own license. Its reported license,
provider notices, version and exact download origin are kept beside the downloaded
executable. Imageio's notices refer to the upstream download, not LibreToon's MIT code.
The historical owned FFmpeg build/source/licensing evidence is retained under
assets/tools and scripts/ffmpeg in the source repository, excluded from packages.


## Project artwork

See [ASSET_PROVENANCE.md](ASSET_PROVENANCE.md) for the maintainer declaration of Codex-assisted visuals and the ChatGPT-generated blue-mouse/LT logo. [ARTWORK_PROVENANCE.md](ARTWORK_PROVENANCE.md) retains historical/hash evidence. Third-party fonts/glyphs/shaders remain separately attributed; this declaration is not an independent legal originality or rights certificate.

## Packaging status

CI packages only LibreToon/resources/notices and rejects accidental inclusion of ffmpeg,
ffprobe or an FFmpeg source ZIP. It does not compile, reproduce, download or redistribute
FFmpeg. Historical source/reproduction gates remain available for any future explicit
redistribution of those owned binaries; they are no longer a dependency of app packaging.
The optional client download is supplied directly by the named upstream provider and
is not a declaration that historical source-access limitations have been resolved.
