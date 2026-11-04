# Rust dependency license evidence

Updated 2 November 2026. All 452 external packages in Cargo.lock were downloaded from the official crates.io registry, their archive SHA-256 values compared with Cargo.lock, and their packaged Cargo.toml license declarations inspected. **This includes the previously pending 226 packages.** No dependencies were installed and Cargo.lock was not changed.

This verifies artifact identity and declared terms, not authorship of every line or complete legal compliance. No target/feature graph or binary composition was reconstructed; the lockfile includes non-Windows and optional dependencies.

## Findings and choices

No declared expression forces GPL/LGPL licensing of LibreToon. self_cell permits Apache-2.0 OR GPL-2.0-only: use Apache-2.0. Both r-efi versions offer MIT OR Apache-2.0 OR LGPL-2.1-or-later: use MIT. This does not remove attribution duties. Other OR expressions permit a listed alternative; legacy slash expressions are preserved as upstream declarations, not rewritten as SPDX proof. Retain the collected notices for all applicable components.

Apache-2.0-only components require their license and any applicable NOTICE and modification notices. dpi is Apache-2.0 AND MIT (both apply). epaint_default_fonts additionally requires OFL-1.1 and Ubuntu-font-1.0; unicode-ident requires Unicode-3.0 in addition to its code-license alternative. ICU and related crates declare Unicode-3.0. BSD, ISC, Zlib, BSL-1.0 and other listed permissive terms also retain their notice obligations. These requirements are compatible with keeping our own code MIT; bundled components retain their own terms.

Original packaged license/copyright/NOTICE files plus 95 upstream notice instances obtained at package VCS commits or the separately content-checked official egui release commit are reproduced in RUST-NOTICES.txt (799 sections after follow-up; includes the explicitly labelled dispatch evidence/template). Font-specific notices remain in fonts/. RUST-DEPENDENCIES.json records package checksums, archive URLs, declarations, file names and pinned upstream notice URLs/hashes.

## Follow-up on the eleven attribution entries

Ten entries now have original notice texts: the eight ecolor/egui family packages use the official 0.37.0 tag at commit 841a328ba2dd8f6996beb7189ebb7fc00e668a2a (MIT and Apache-2.0); both r-efi versions contain their full notices in AUTHORS, which the earlier filename search missed. Copies and comparison evidence are in supplemental/.

All packaged src/ files of the eight egui-family entries match that tag byte for byte. Do not confuse this with the unresolvable .cargo_vcs_info.json revision: the evidence is the content comparison against the official release. The egui README and two epaint_default_fonts font/notice resources differ from the tag; those exceptions are explicitly recorded. Actual packaged font notices, including the different emoji/icon notice, remain preserved and applicable.

The eleventh entry, dispatch 0.2.0, is now documented with its original Cargo.toml.orig (MIT; author Steven Sheldon) and an explicitly labelled canonical SPDX MIT template. No standalone copyright/license file exists in the inspected crate or upstream revision. No copyright year/holder has been invented, and the canonical template is NOT presented as an original upstream notice. Thus **ten original-notice gaps are closed; dispatch original-copyright recovery/confirmation remains unresolved**. See supplemental/dispatch-0.2.0/ATTRIBUTION.md. This distinction must not be hidden by saying that all eleven original notices were verified.

## Full locked inventory

| Package | Version | Declared license | Notice evidence |
|---|---|---|---|
| ab_glyph | 0.2.32 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| accesskit | 0.25.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_atspi_common | 0.21.1 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_consumer | 0.39.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_ios | 0.2.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_macos | 0.27.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_unix | 0.24.1 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_windows | 0.35.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| accesskit_winit | 0.34.2 | Apache-2.0 | 0 packaged; 2 pinned upstream |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| ahash | 0.8.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| android-activity | 0.6.1 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| android-properties | 0.2.2 | MIT | 1 packaged; 0 pinned upstream |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| approx | 0.5.1 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| arboard | 3.7.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| arrayref | 0.3.9 | BSD-2-Clause | 1 packaged; 0 pinned upstream |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| as-raw-xcb-connection | 1.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| async-broadcast | 0.7.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| async-channel | 2.5.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-executor | 1.14.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-io | 2.6.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-lock | 3.4.2 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-process | 2.5.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-recursion | 1.2.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| async-signal | 0.2.14 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-task | 4.7.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| async-trait | 0.1.92 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| atspi | 0.29.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| atspi-common | 0.13.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| atspi-proxies | 0.13.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| bit-set | 0.10.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| bitflags | 1.3.2 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| block2 | 0.5.1 | MIT | 0 packaged; 1 pinned upstream |
| block2 | 0.6.2 | MIT | 0 packaged; 1 pinned upstream |
| blocking | 1.7.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| byteorder | 1.5.0 | Unlicense OR MIT | 2 packaged; 0 pinned upstream |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | 1 packaged; 0 pinned upstream |
| bytes | 1.12.1 | MIT | 1 packaged; 0 pinned upstream |
| calloop | 0.13.0 | MIT | 1 packaged; 0 pinned upstream |
| calloop | 0.14.5 | MIT | 1 packaged; 0 pinned upstream |
| calloop-wayland-source | 0.3.0 | MIT | 1 packaged; 0 pinned upstream |
| calloop-wayland-source | 0.4.1 | MIT | 1 packaged; 0 pinned upstream |
| cc | 1.10.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| cfg_aliases | 0.2.2 | MIT | 2 packaged; 0 pinned upstream |
| cgl | 0.3.2 | MIT / Apache-2.0 | 3 packaged; 0 pinned upstream |
| clipboard-win | 5.4.3 | BSL-1.0 | 0 packaged; 1 pinned upstream |
| codespan-reporting | 0.13.1 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| color | 0.3.3 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| combine | 4.6.9 | MIT | 1 packaged; 0 pinned upstream |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| core-graphics | 0.23.2 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| core-graphics-types | 0.1.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| crunchy | 0.2.4 | MIT | 1 packaged; 0 pinned upstream |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib | 3 packaged; 0 pinned upstream |
| dispatch | 0.2.0 | MIT | 0 packaged; 0 upstream — declaration/template documented; original copyright unavailable |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| dlib | 0.5.3 | MIT | 1 packaged; 0 pinned upstream |
| document-features | 0.2.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| dpi | 0.1.2 | Apache-2.0 AND MIT | 2 packaged; 0 pinned upstream |
| ecolor | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| eframe | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| egui | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| egui-wgpu | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| egui-winit | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| egui_glow | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| egui_system_font_provider | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| either | 1.19.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| emath | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| endi | 1.1.1 | MIT | 1 packaged; 0 pinned upstream |
| enumflags2 | 0.7.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| enumflags2_derive | 0.7.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| epaint | 0.37.0 | MIT OR Apache-2.0 | 0 packaged; 2 upstream |
| epaint_default_fonts | 0.37.0 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | 0 packaged; 2 upstream |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| errno | 0.3.14 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| error-code | 3.4.0 | BSL-1.0 | 1 packaged; 0 pinned upstream |
| euclid | 0.22.14 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| event-listener | 5.4.2 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| event-listener-strategy | 0.5.4 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| fastrand | 2.5.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| fax | 0.2.7 | MIT | 1 packaged; 0 pinned upstream |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| fearless_simd | 0.4.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| fixedbitset | 0.5.7 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| float_next_after | 1.0.0 | MIT | 1 packaged; 0 pinned upstream |
| float_next_after | 2.0.0 | MIT | 1 packaged; 0 pinned upstream |
| foldhash | 0.1.5 | Zlib | 1 packaged; 0 pinned upstream |
| foldhash | 0.2.0 | Zlib | 1 packaged; 0 pinned upstream |
| font-types | 0.12.7 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| fontique | 0.11.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| foreign-types | 0.5.0 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| futures-lite | 2.6.1 | Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| futures-macro | 0.3.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| geo | 0.33.1 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| geo-types | 0.7.20 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| geographiclib-rs | 0.2.7 | MIT | 1 packaged; 0 pinned upstream |
| gethostname | 1.1.0 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| gl_generator | 0.14.0 | Apache-2.0 | 0 packaged; 1 pinned upstream |
| glifo | 0.3.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib | 3 packaged; 0 pinned upstream |
| glutin | 0.32.3 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| glutin-winit | 0.5.0 | MIT | 1 packaged; 0 pinned upstream |
| glutin_egl_sys | 0.7.1 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| glutin_glx_sys | 0.6.1 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| guillotiere | 0.7.0 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| half | 2.7.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| harfrust | 0.12.0 | MIT | 1 packaged; 0 pinned upstream |
| hash32 | 0.3.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| heapless | 0.8.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| hex | 0.4.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| i_float | 1.16.0 | MIT | 1 packaged; 0 pinned upstream |
| i_key_sort | 0.10.3 | MIT | 0 packaged; 1 pinned upstream |
| i_overlay | 4.5.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| i_shape | 1.18.0 | MIT | 1 packaged; 0 pinned upstream |
| i_tree | 0.18.0 | MIT | 1 packaged; 0 pinned upstream |
| icu_collections | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| icu_locale_core | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| icu_normalizer | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| icu_normalizer_data | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| icu_properties | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| icu_properties_data | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| icu_provider | 2.4.0 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| idna | 1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| image | 0.25.10 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| itertools | 0.15.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| itoa | 1.0.18 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| jni | 0.22.4 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| jni-sys | 0.3.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| jobserver | 0.1.35 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| js-sys | 0.3.106 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| khronos-egl | 6.0.0 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| khronos_api | 3.1.0 | Apache-2.0 | 0 packaged; 1 pinned upstream |
| kurbo | 0.13.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| libc | 0.2.191 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| libloading | 0.8.9 | ISC | 1 packaged; 0 pinned upstream |
| libm | 0.2.16 | MIT | 1 packaged; 0 pinned upstream |
| libredox | 0.1.25 | MIT | 1 packaged; 0 pinned upstream |
| linebender_resource_handle | 0.1.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 packaged; 0 pinned upstream |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 packaged; 0 pinned upstream |
| litemap | 0.8.3 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| litrs | 1.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| log | 0.4.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| lyon_geom | 1.0.20 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| lyon_path | 1.0.19 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| lyon_tessellation | 1.0.22 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| memchr | 2.8.3 | Unlicense OR MIT | 2 packaged; 0 pinned upstream |
| memmap2 | 0.9.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| memoffset | 0.9.1 | MIT | 1 packaged; 0 pinned upstream |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | 4 packaged; 0 pinned upstream |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | 4 packaged; 0 pinned upstream |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| naga | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| naga-types | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| ndk | 0.9.0 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| ndk-sys | 0.6.0+11769913 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| nom | 8.0.0 | MIT | 1 packaged; 0 pinned upstream |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| objc-sys | 0.3.5 | MIT | 0 packaged; 1 pinned upstream |
| objc2 | 0.5.3 | MIT | 0 packaged; 1 pinned upstream |
| objc2 | 0.6.5 | MIT | 0 packaged; 1 pinned upstream |
| objc2-app-kit | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-cloud-kit | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-contacts | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-core-data | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-core-graphics | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-core-image | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-core-location | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-core-text | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-encode | 4.1.0 | MIT | 0 packaged; 1 pinned upstream |
| objc2-foundation | 0.2.2 | MIT | 2 packaged; 0 pinned upstream |
| objc2-foundation | 0.3.2 | MIT | 2 packaged; 0 pinned upstream |
| objc2-io-surface | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-link-presentation | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-metal | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-metal | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-quartz-core | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-quartz-core | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-symbols | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-ui-kit | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-ui-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 0 packaged; 1 pinned upstream |
| objc2-uniform-type-identifiers | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| objc2-user-notifications | 0.2.2 | MIT | 0 packaged; 1 pinned upstream |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| orbclient | 0.3.55 | MIT | 1 packaged; 0 pinned upstream |
| ordered-float | 5.5.1 | MIT | 1 packaged; 0 pinned upstream |
| ordered-stream | 0.2.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| os_pipe | 1.2.3 | MIT | 1 packaged; 0 pinned upstream |
| owned_ttf_parser | 0.25.1 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| parking | 2.2.1 | Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| parlance | 0.1.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| peniko | 0.6.1 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| petgraph | 0.8.3 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| phf | 0.14.0 | MIT | 1 packaged; 0 pinned upstream |
| phf_generator | 0.14.0 | MIT | 1 packaged; 0 pinned upstream |
| phf_macros | 0.14.0 | MIT | 1 packaged; 0 pinned upstream |
| phf_shared | 0.14.0 | MIT | 1 packaged; 0 pinned upstream |
| pin-project | 1.1.13 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| pin-project-internal | 1.1.13 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| piper | 0.2.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| plain | 0.2.3 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| png | 0.18.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| poll-promise | 0.3.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| polling | 3.11.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| pollster | 0.4.0 | Apache-2.0/MIT | 2 packaged; 0 pinned upstream |
| pollster | 1.0.1 | Apache-2.0/MIT | 2 packaged; 0 pinned upstream |
| polycool | 0.4.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| potential_utf | 0.1.6 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| presser | 0.3.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| profiling | 1.0.18 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| quick-error | 2.0.1 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| quick-xml | 0.41.0 | MIT | 1 packaged; 0 pinned upstream |
| quote | 1.0.47 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 1 packaged; 0 upstream |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 1 packaged; 0 upstream |
| rand | 0.10.3 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| rand_pcg | 0.10.2 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | 3 packaged; 0 pinned upstream |
| raw-window-metal | 1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| read-fonts | 0.41.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| redox_syscall | 0.4.1 | MIT | 1 packaged; 0 pinned upstream |
| redox_syscall | 0.5.18 | MIT | 1 packaged; 0 pinned upstream |
| redox_syscall | 0.9.5 | MIT | 1 packaged; 0 pinned upstream |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| rfd | 0.17.2 | MIT | 2 packaged; 0 pinned upstream |
| robust | 1.2.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| roxmltree | 0.21.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| rstar | 0.12.2 | MIT OR Apache-2.0 | 0 packaged; 2 pinned upstream |
| rustc-hash | 1.1.0 | Apache-2.0/MIT | 2 packaged; 0 pinned upstream |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 packaged; 0 pinned upstream |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 packaged; 0 pinned upstream |
| rustversion | 1.0.23 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| same-file | 1.0.6 | Unlicense/MIT | 2 packaged; 0 pinned upstream |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| sctk-adwaita | 0.10.1 | MIT | 1 packaged; 0 pinned upstream |
| self_cell | 1.3.0 | Apache-2.0 OR GPL-2.0-only | 2 packaged; 0 pinned upstream |
| semver | 1.0.28 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| send_wrapper | 0.6.0 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| serde | 1.0.229 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| serde_repr | 0.1.21 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| shlex | 2.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| sif-itree | 0.4.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| simd-adler32 | 0.3.10 | MIT | 1 packaged; 0 pinned upstream |
| simd_cesu8 | 1.2.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| siphasher | 1.0.4 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| skrifa | 0.44.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| slab | 0.4.12 | MIT | 1 packaged; 0 pinned upstream |
| slotmap | 1.1.1 | Zlib | 1 packaged; 0 pinned upstream |
| smallvec | 1.16.4 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| smithay-client-toolkit | 0.19.2 | MIT | 1 packaged; 0 pinned upstream |
| smithay-client-toolkit | 0.20.0 | MIT | 1 packaged; 0 pinned upstream |
| smithay-clipboard | 0.7.3 | MIT | 1 packaged; 0 pinned upstream |
| smol_str | 0.2.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 | 0 packaged; 1 pinned upstream |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| strict-num | 0.1.1 | MIT | 1 packaged; 0 pinned upstream |
| syn | 2.0.119 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| syn | 3.0.9 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| synstructure | 0.14.0 | MIT | 1 packaged; 0 pinned upstream |
| sys-locale | 0.3.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| termcolor | 1.4.1 | Unlicense OR MIT | 2 packaged; 0 pinned upstream |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| tiff | 0.11.3 | MIT | 2 packaged; 0 pinned upstream |
| tiny-skia | 0.11.4 | BSD-3-Clause | 1 packaged; 0 pinned upstream |
| tiny-skia-path | 0.11.4 | BSD-3-Clause | 1 packaged; 0 pinned upstream |
| tinystr | 0.8.5 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| tokio | 1.54.1 | MIT | 1 packaged; 0 pinned upstream |
| toml_datetime | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| toml_edit | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| tracing | 0.1.44 | MIT | 1 packaged; 0 pinned upstream |
| tracing-attributes | 0.1.31 | MIT | 1 packaged; 0 pinned upstream |
| tracing-core | 0.1.36 | MIT | 2 packaged; 0 pinned upstream |
| tree_magic_mini | 3.2.2 | MIT | 1 packaged; 0 pinned upstream |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| type-map | 0.5.1 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| uds_windows | 1.2.1 | MIT | 1 packaged; 0 pinned upstream |
| unicode-general-category | 1.1.0 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | 3 packaged; 0 pinned upstream |
| unicode-properties | 0.1.4 | MIT/Apache-2.0 | 3 packaged; 0 pinned upstream |
| unicode-script | 0.5.8 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 | 3 packaged; 0 pinned upstream |
| url | 2.5.8 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| uuid | 1.29.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| vello_common | 0.2.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| vello_cpu | 0.2.0 | Apache-2.0 OR MIT | 2 packaged; 0 pinned upstream |
| version_check | 0.9.5 | MIT/Apache-2.0 | 2 packaged; 0 pinned upstream |
| walkdir | 2.5.0 | Unlicense/MIT | 2 packaged; 0 pinned upstream |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wasm-bindgen-futures | 0.4.79 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wayland-backend | 0.3.17 | MIT | 1 packaged; 0 pinned upstream |
| wayland-client | 0.31.15 | MIT | 1 packaged; 0 pinned upstream |
| wayland-csd-frame | 0.3.0 | MIT | 1 packaged; 0 pinned upstream |
| wayland-cursor | 0.31.14 | MIT | 1 packaged; 0 pinned upstream |
| wayland-protocols | 0.32.13 | MIT | 2 packaged; 0 pinned upstream |
| wayland-protocols-experimental | 20250721.0.1 | MIT | 1 packaged; 0 pinned upstream |
| wayland-protocols-misc | 0.3.12 | MIT | 1 packaged; 0 pinned upstream |
| wayland-protocols-plasma | 0.3.12 | MIT | 2 packaged; 0 pinned upstream |
| wayland-protocols-wlr | 0.3.12 | MIT | 1 packaged; 0 pinned upstream |
| wayland-scanner | 0.31.11 | MIT | 1 packaged; 0 pinned upstream |
| wayland-sys | 0.31.11 | MIT | 1 packaged; 0 pinned upstream |
| web-sys | 0.3.106 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| web-time | 1.1.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| webbrowser | 1.2.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| weezl | 0.1.12 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-core | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-core-deps-apple | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-core-deps-emscripten | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-core-deps-wasm | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-core-deps-windows-linux-android | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-hal | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-naga-bridge | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| wgpu-types | 30.0.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| winapi-util | 0.1.11 | Unlicense OR MIT | 2 packaged; 0 pinned upstream |
| windows | 0.62.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| winit | 0.30.13 | Apache-2.0 | 1 packaged; 0 pinned upstream |
| winnow | 1.0.4 | MIT | 1 packaged; 0 pinned upstream |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| wl-clipboard-rs | 0.9.5 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| writeable | 0.6.4 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| x11-dl | 2.21.0 | MIT | 1 packaged; 0 pinned upstream |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| x11rb-protocol | 0.13.2 | MIT OR Apache-2.0 | 2 packaged; 0 pinned upstream |
| xcursor | 0.3.11 | MIT | 1 packaged; 0 pinned upstream |
| xkbcommon-dl | 0.4.2 | MIT | 1 packaged; 0 pinned upstream |
| xkeysym | 0.2.1 | MIT OR Apache-2.0 OR Zlib | 3 packaged; 0 pinned upstream |
| xml-rs | 0.8.29 | MIT | 1 packaged; 0 pinned upstream |
| yeslogic-fontconfig-sys | 6.0.1 | MIT | 1 packaged; 0 pinned upstream |
| yoke | 0.8.4 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| yoke-derive | 0.8.4 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| zbus | 5.19.0 | MIT | 1 packaged; 0 pinned upstream |
| zbus-lockstep | 0.5.2 | MIT | 1 packaged; 0 pinned upstream |
| zbus-lockstep-macros | 0.5.2 | MIT | 1 packaged; 0 pinned upstream |
| zbus_macros | 5.19.0 | MIT | 1 packaged; 0 pinned upstream |
| zbus_names | 4.3.5 | MIT | 1 packaged; 0 pinned upstream |
| zbus_xml | 5.2.1 | MIT | 1 packaged; 0 pinned upstream |
| zcheapstr | 1.1.0 | MIT | 1 packaged; 0 pinned upstream |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | 3 packaged; 0 pinned upstream |
| zerofrom | 0.1.8 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| zerotrie | 0.2.5 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| zerovec | 0.11.9 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| zerovec-derive | 0.11.7 | Unicode-3.0 | 1 packaged; 0 pinned upstream |
| zlib-rs | 0.6.8 | Zlib | 1 packaged; 0 pinned upstream |
| zmij | 1.0.23 | MIT | 1 packaged; 0 pinned upstream |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib | 3 packaged; 0 pinned upstream |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib | 3 packaged; 0 pinned upstream |
| zvariant | 5.15.1 | MIT | 1 packaged; 0 pinned upstream |
| zvariant_derive | 5.15.1 | MIT | 1 packaged; 0 pinned upstream |
| zvariant_utils | 4.2.0 | MIT | 1 packaged; 0 pinned upstream |

## Native audio playback additions

Original available declarations are included in RUST-NOTICES.txt and audio/. Each new crate archive SHA-256 was checked against Cargo.lock. The dasp_sample source files were compared byte-for-byte against its recorded upstream revision. The objc2 notices were recovered from the revision declared by each wrapper in .cargo_vcs_info.json; generated framework sources are not present in that upstream git archive and were not compared. The objc2 upstream LICENSE.md describes its Apple SDK derivation limitation and does not include a standalone license grant text. That limitation is retained in the included original declaration. Existing verification limits elsewhere in this inventory remain applicable.

- alsa 0.9.1 — Apache-2.0/MIT
- alsa-sys 0.3.1 — MIT
- cesu8 1.1.0 — Apache-2.0/MIT
- coreaudio-rs 0.13.0 — MIT/Apache-2.0
- cpal 0.16.0 — Apache-2.0
- dasp_sample 0.11.0 — MIT OR Apache-2.0
- jni 0.21.1 — MIT/Apache-2.0
- mach2 0.4.3 — BSD-2-Clause OR MIT OR Apache-2.0
- num-bigint 0.4.8 — MIT OR Apache-2.0
- num-derive 0.4.2 — MIT OR Apache-2.0
- num-integer 0.1.47 — MIT OR Apache-2.0
- num-rational 0.4.2 — MIT OR Apache-2.0
- objc2-audio-toolbox 0.3.2 — Zlib OR Apache-2.0 OR MIT
- objc2-core-audio 0.3.2 — Zlib OR Apache-2.0 OR MIT
- objc2-core-audio-types 0.3.2 — Zlib OR Apache-2.0 OR MIT
- rodio 0.21.1 — MIT OR Apache-2.0
- windows 0.54.0 — MIT OR Apache-2.0
- windows-core 0.54.0 — MIT OR Apache-2.0
- windows-result 0.1.2 — MIT OR Apache-2.0
- windows-sys 0.45.0 — MIT OR Apache-2.0
- windows-targets 0.42.2 — MIT OR Apache-2.0
- windows_aarch64_gnullvm 0.42.2 — MIT OR Apache-2.0
- windows_aarch64_msvc 0.42.2 — MIT OR Apache-2.0
- windows_i686_gnu 0.42.2 — MIT OR Apache-2.0
- windows_i686_msvc 0.42.2 — MIT OR Apache-2.0
- windows_x86_64_gnu 0.42.2 — MIT OR Apache-2.0
- windows_x86_64_gnullvm 0.42.2 — MIT OR Apache-2.0
- windows_x86_64_msvc 0.42.2 — MIT OR Apache-2.0
