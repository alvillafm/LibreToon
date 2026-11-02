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

## FFmpeg and Imageio

[assets/tools/ffmpeg.exe](tools/ffmpeg.exe) is the owned FFmpeg 7.1.5-LibreToon-1 build, GPL version 3 or later. Its exact source-locked build, x264/zlib and runtime notices are documented separately. It is not MIT. Retained Imageio BSD materials describe the earlier distribution, not this executable.

See [tools/THIRD_PARTY.md](tools/THIRD_PARTY.md), [binary metadata](tools/BINARY_METADATA.txt) and [build evidence](tools/BUILD_EVIDENCE.json). The complete source ZIP and original notices are generated and validated with each Windows distribution. Preserve them together with the executable; the evidence records its scope and is not a blanket legal certification.

## Project artwork

See [ASSET_PROVENANCE.md](ASSET_PROVENANCE.md) for the maintainer declaration of Codex-assisted visuals and the ChatGPT-generated blue-mouse/LT logo. [ARTWORK_PROVENANCE.md](ARTWORK_PROVENANCE.md) retains historical/hash evidence. Third-party fonts/glyphs/shaders remain separately attributed; this declaration is not an independent legal originality or rights certificate.

## Packaging status

Windows CI retains the root MIT LICENSE and third-party notices, builds and reproduces FFmpeg, runs export tests and source review, then validates the exact staged binary/source pair before upload. The same artifact includes complete corresponding source and SOURCE-ACCESS.txt. Local checks passed; a remote Actions run/public release was not initiated. See tools/SOURCE_STATUS.json and the build guide.
