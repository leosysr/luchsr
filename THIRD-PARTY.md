# Fremdbestandteile

Luchsr selbst steht unter MIT (siehe `LICENSE`). Diese Datei listet, was
mit ausgeliefert wird, und unter welcher Lizenz.

**Diese Datei wird erzeugt.** Nicht von Hand pflegen — `scripts/third-party.ps1`
liest die tatsächlichen Abhängigkeitsgraphen. Die Lizenz-Volltexte liegen
unter `licenses/`.

## Schriften

Lokal eingebettet (Entscheidung D4: kein Laufzeit-Netzzugriff). Die SIL Open
Font License verlangt, dass ihr Text die Schriftdateien begleitet — deshalb
liegt er unter `licenses/`.

| Schrift | Lizenz | Volltext |
|---|---|---|
| Manrope | SIL Open Font License 1.1 | `licenses/OFL-1.1-Manrope.txt` |
| IBM Plex Mono | SIL Open Font License 1.1 | `licenses/OFL-1.1-IBM-Plex.txt` |

## Schwaches Copyleft — MPL-2.0

Fünf Crates aus dem Tauri-Unterbau stehen unter MPL-2.0. Das ist mit MIT
vereinbar: die MPL ist dateiweise und erlaubt ausdrücklich die Einbettung in
ein „Larger Work" unter anderer Lizenz (Abschnitt 3.3). Zwei Pflichten
bleiben, und beide sind hier erfüllt: der Lizenztext liegt bei
(`licenses/MPL-2.0.txt`), und der Quelltext der betroffenen Dateien ist
verfügbar — sie sind unverändert übernommen und über crates.io abrufbar.

| Crate | Quelle |
|---|---|
| `cssparser v0.36.0` | https://crates.io/crates/cssparser |
| `cssparser-macros v0.6.1 (proc-macro)` | https://crates.io/crates/cssparser-macros |
| `dtoa-short v0.3.5` | https://crates.io/crates/dtoa-short |
| `option-ext v0.2.0` | https://crates.io/crates/option-ext |
| `selectors v0.36.1` | https://crates.io/crates/selectors |

## Rust-Crates

Im Windows-Build (`x86_64-pc-windows-msvc`) enthalten: **294** Crates.
Cargo.lock erfasst zusätzlich Crates anderer Plattformen; die stehen hier
absichtlich nicht.

### Verteilung

| Anzahl | Lizenz |
|---|---|
| 143 | MIT OR Apache-2.0 |
| 58 | MIT |
| 31 | Apache-2.0 OR MIT |
| 18 | Unicode-3.0 |
| 12 | MIT/Apache-2.0 |
| 5 | MPL-2.0 |
| 5 | Unlicense OR MIT |
| 2 | Unlicense/MIT |
| 2 | BSD-3-Clause OR Apache-2.0 |
| 2 | BSD-3-Clause |
| 2 | Apache-2.0 |
| 1 | BSD-2-Clause OR Apache-2.0 OR MIT |
| 1 | MIT OR Zlib OR Apache-2.0 |
| 1 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| 1 | Apache-2.0 OR BSL-1.0 |
| 1 | MIT OR Apache-2.0 OR Zlib |
| 1 | BSD-3-Clause/MIT |
| 1 | Zlib OR Apache-2.0 OR MIT |
| 1 | 0BSD OR MIT OR Apache-2.0 |
| 1 | BSD-3-Clause AND MIT |
| 1 | Apache-2.0 AND MIT |
| 1 | Apache-2.0 / MIT |
| 1 | Zlib |
| 1 | CC0-1.0 OR MIT-0 OR Apache-2.0 |
| 1 | (Apache-2.0 OR MIT) AND BSD-3-Clause |

### Vollständige Liste

| Crate | Lizenz |
|---|---|
| `adler2 v2.0.1` | 0BSD OR MIT OR Apache-2.0 |
| `aho-corasick v1.1.5` | Unlicense OR MIT |
| `alloc-no-stdlib v2.0.4` | BSD-3-Clause |
| `alloc-stdlib v0.2.4` | BSD-3-Clause |
| `anyhow v1.0.104` | MIT OR Apache-2.0 |
| `async-compression v0.4.43` | MIT OR Apache-2.0 |
| `atomic-waker v1.1.2` | Apache-2.0 OR MIT |
| `auto-launch v0.5.0` | MIT |
| `base64 v0.22.1` | MIT OR Apache-2.0 |
| `bitflags v1.3.2` | MIT/Apache-2.0 |
| `bitflags v2.13.1` | MIT OR Apache-2.0 |
| `bit-set v0.8.0` | Apache-2.0 OR MIT |
| `bit-vec v0.8.0` | Apache-2.0 OR MIT |
| `block-buffer v0.10.4` | MIT OR Apache-2.0 |
| `brotli v8.0.4` | BSD-3-Clause AND MIT |
| `brotli-decompressor v5.0.3` | BSD-3-Clause/MIT |
| `bytemuck v1.25.2` | Zlib OR Apache-2.0 OR MIT |
| `byteorder v1.5.0` | Unlicense OR MIT |
| `byteorder-lite v0.1.0` | Unlicense OR MIT |
| `bytes v1.12.1` | MIT |
| `camino v1.2.5` | MIT OR Apache-2.0 |
| `cargo_metadata v0.19.2` | MIT |
| `cargo-platform v0.1.9` | MIT OR Apache-2.0 |
| `cfb v0.7.3` | MIT |
| `cfg-if v1.0.4` | MIT OR Apache-2.0 |
| `chrono v0.4.45` | MIT OR Apache-2.0 |
| `compression-codecs v0.4.38` | MIT OR Apache-2.0 |
| `compression-core v0.4.32` | MIT OR Apache-2.0 |
| `cookie v0.18.2` | MIT OR Apache-2.0 |
| `cpufeatures v0.2.17` | MIT OR Apache-2.0 |
| `crc32fast v1.5.0` | MIT OR Apache-2.0 |
| `crossbeam-channel v0.5.16` | MIT OR Apache-2.0 |
| `crossbeam-utils v0.8.22` | MIT OR Apache-2.0 |
| `crypto-common v0.1.7` | MIT OR Apache-2.0 |
| `cssparser v0.36.0` | MPL-2.0 |
| `cssparser-macros v0.6.1 (proc-macro)` | MPL-2.0 |
| `ctor v0.8.0` | Apache-2.0 OR MIT |
| `ctor-proc-macro v0.0.7 (proc-macro)` | Apache-2.0 OR MIT |
| `darling v0.23.0` | MIT |
| `darling_core v0.23.0` | MIT |
| `darling_macro v0.23.0 (proc-macro)` | MIT |
| `deranged v0.5.8` | MIT OR Apache-2.0 |
| `derive_more v2.1.1` | MIT |
| `derive_more-impl v2.1.1 (proc-macro)` | MIT |
| `digest v0.10.7` | MIT OR Apache-2.0 |
| `dirs v6.0.0` | MIT OR Apache-2.0 |
| `dirs-sys v0.5.0` | MIT OR Apache-2.0 |
| `displaydoc v0.2.7 (proc-macro)` | MIT OR Apache-2.0 |
| `dom_query v0.27.0` | MIT |
| `dpi v0.1.2` | Apache-2.0 AND MIT |
| `dtoa v1.0.11` | MIT OR Apache-2.0 |
| `dtoa-short v0.3.5` | MPL-2.0 |
| `dunce v1.0.5` | CC0-1.0 OR MIT-0 OR Apache-2.0 |
| `dyn-clone v1.0.20` | MIT OR Apache-2.0 |
| `encoding_rs v0.8.35` | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| `equivalent v1.0.2` | Apache-2.0 OR MIT |
| `erased-serde v0.4.10` | MIT OR Apache-2.0 |
| `fastrand v2.5.0` | Apache-2.0 OR MIT |
| `fdeflate v0.3.7` | MIT OR Apache-2.0 |
| `fern v0.7.1` | MIT |
| `flate2 v1.1.9` | MIT OR Apache-2.0 |
| `fnv v1.0.7` | Apache-2.0 / MIT |
| `foldhash v0.2.0` | Zlib |
| `form_urlencoded v1.2.2` | MIT OR Apache-2.0 |
| `futures-channel v0.3.34` | MIT OR Apache-2.0 |
| `futures-core v0.3.34` | MIT OR Apache-2.0 |
| `futures-sink v0.3.34` | MIT OR Apache-2.0 |
| `futures-task v0.3.34` | MIT OR Apache-2.0 |
| `futures-util v0.3.34` | MIT OR Apache-2.0 |
| `generic-array v0.14.7` | MIT |
| `getrandom v0.3.4` | MIT OR Apache-2.0 |
| `getrandom v0.4.3` | MIT OR Apache-2.0 |
| `glob v0.3.4` | MIT OR Apache-2.0 |
| `h2 v0.4.18` | MIT |
| `hashbrown v0.12.3` | MIT OR Apache-2.0 |
| `hashbrown v0.17.1` | MIT OR Apache-2.0 |
| `heck v0.5.0` | MIT OR Apache-2.0 |
| `html5ever v0.38.0` | MIT OR Apache-2.0 |
| `http v1.5.0` | MIT OR Apache-2.0 |
| `httparse v1.10.1` | MIT OR Apache-2.0 |
| `http-body v1.1.0` | MIT |
| `http-body-util v0.1.5` | MIT |
| `hyper v1.11.0` | MIT |
| `hyper-tls v0.6.0` | MIT/Apache-2.0 |
| `hyper-util v0.1.20` | MIT |
| `ico v0.5.0` | MIT |
| `icu_collections v2.3.0` | Unicode-3.0 |
| `icu_locale_core v2.3.0` | Unicode-3.0 |
| `icu_normalizer v2.3.0` | Unicode-3.0 |
| `icu_normalizer_data v2.3.0` | Unicode-3.0 |
| `icu_properties v2.3.0` | Unicode-3.0 |
| `icu_properties_data v2.3.0` | Unicode-3.0 |
| `icu_provider v2.3.1` | Unicode-3.0 |
| `ident_case v1.0.1` | MIT/Apache-2.0 |
| `idna v1.1.0` | MIT OR Apache-2.0 |
| `idna_adapter v1.2.2` | Apache-2.0 OR MIT |
| `image v0.25.10` | MIT OR Apache-2.0 |
| `indexmap v1.9.3` | Apache-2.0 OR MIT |
| `indexmap v2.14.0` | Apache-2.0 OR MIT |
| `infer v0.19.0` | MIT |
| `ipnet v2.12.1` | MIT OR Apache-2.0 |
| `itoa v1.0.18` | MIT OR Apache-2.0 |
| `json-patch v3.0.1` | MIT/Apache-2.0 |
| `jsonptr v0.6.3` | MIT OR Apache-2.0 |
| `keyboard-types v0.7.0` | MIT OR Apache-2.0 |
| `keyring v4.1.6` | MIT OR Apache-2.0 |
| `keyring-core v1.0.0` | MIT OR Apache-2.0 |
| `libc v0.2.189` | MIT OR Apache-2.0 |
| `litemap v0.8.3` | Unicode-3.0 |
| `lock_api v0.4.14` | MIT OR Apache-2.0 |
| `log v0.4.33` | MIT OR Apache-2.0 |
| `luchsr v0.0.0 (C:\code\Luchsr\src-tauri)` | MIT |
| `markup5ever v0.38.0` | MIT OR Apache-2.0 |
| `memchr v2.8.3` | Unlicense OR MIT |
| `mime v0.3.17` | MIT OR Apache-2.0 |
| `miniz_oxide v0.8.9` | MIT OR Zlib OR Apache-2.0 |
| `mio v1.2.2` | MIT |
| `moxcms v0.8.1` | BSD-3-Clause OR Apache-2.0 |
| `muda v0.19.3` | Apache-2.0 OR MIT |
| `native-tls v0.2.18` | MIT OR Apache-2.0 |
| `new_debug_unreachable v1.0.6` | MIT |
| `notify-rust v4.18.0` | MIT OR Apache-2.0 |
| `num-conv v0.2.2` | MIT OR Apache-2.0 |
| `num-traits v0.2.19` | MIT OR Apache-2.0 |
| `once_cell v1.21.4` | MIT OR Apache-2.0 |
| `open v5.4.1` | MIT |
| `option-ext v0.2.0` | MPL-2.0 |
| `parking_lot v0.12.5` | MIT OR Apache-2.0 |
| `parking_lot_core v0.9.12` | MIT OR Apache-2.0 |
| `percent-encoding v2.3.2` | MIT OR Apache-2.0 |
| `phf v0.13.1` | MIT |
| `phf_generator v0.13.1` | MIT |
| `phf_macros v0.13.1 (proc-macro)` | MIT |
| `phf_shared v0.13.1` | MIT |
| `pin-project-lite v0.2.17` | Apache-2.0 OR MIT |
| `plist v1.10.0` | MIT |
| `png v0.17.16` | MIT OR Apache-2.0 |
| `png v0.18.1` | MIT OR Apache-2.0 |
| `potential_utf v0.1.6` | Unicode-3.0 |
| `powerfmt v0.2.0` | MIT OR Apache-2.0 |
| `ppv-lite86 v0.2.21` | MIT OR Apache-2.0 |
| `precomputed-hash v0.1.1` | MIT |
| `proc-macro2 v1.0.107` | MIT OR Apache-2.0 |
| `pxfm v0.1.30` | BSD-3-Clause OR Apache-2.0 |
| `quick-xml v0.41.0` | MIT |
| `quote v1.0.47` | MIT OR Apache-2.0 |
| `rand v0.9.5` | MIT OR Apache-2.0 |
| `rand_chacha v0.9.0` | MIT OR Apache-2.0 |
| `rand_core v0.9.5` | MIT OR Apache-2.0 |
| `raw-window-handle v0.6.2` | MIT OR Apache-2.0 OR Zlib |
| `regex v1.13.1` | MIT OR Apache-2.0 |
| `regex-automata v0.4.18` | MIT OR Apache-2.0 |
| `regex-syntax v0.8.11` | MIT OR Apache-2.0 |
| `reqwest v0.13.4` | MIT OR Apache-2.0 |
| `rfd v0.16.0` | MIT |
| `rustc-hash v2.1.3` | Apache-2.0 OR MIT |
| `rustls-pki-types v1.15.1` | MIT OR Apache-2.0 |
| `ryu v1.0.23` | Apache-2.0 OR BSL-1.0 |
| `same-file v1.0.6` | Unlicense/MIT |
| `schannel v0.1.29` | MIT |
| `schemars v0.8.22` | MIT |
| `schemars_derive v0.8.22 (proc-macro)` | MIT |
| `scopeguard v1.2.0` | MIT OR Apache-2.0 |
| `selectors v0.36.1` | MPL-2.0 |
| `semver v1.0.28` | MIT OR Apache-2.0 |
| `serde v1.0.229` | MIT OR Apache-2.0 |
| `serde_core v1.0.229` | MIT OR Apache-2.0 |
| `serde_derive v1.0.229 (proc-macro)` | MIT OR Apache-2.0 |
| `serde_derive_internals v0.29.1` | MIT OR Apache-2.0 |
| `serde_json v1.0.151` | MIT OR Apache-2.0 |
| `serde_repr v0.1.21 (proc-macro)` | MIT OR Apache-2.0 |
| `serde_spanned v1.1.1` | MIT OR Apache-2.0 |
| `serde_urlencoded v0.7.1` | MIT/Apache-2.0 |
| `serde_with v3.22.0` | MIT OR Apache-2.0 |
| `serde_with_macros v3.22.0 (proc-macro)` | MIT OR Apache-2.0 |
| `serde-untagged v0.1.9` | MIT OR Apache-2.0 |
| `serialize-to-javascript v0.1.2` | MIT OR Apache-2.0 |
| `serialize-to-javascript-impl v0.1.2 (proc-macro)` | MIT OR Apache-2.0 |
| `servo_arc v0.4.3` | MIT OR Apache-2.0 |
| `sha2 v0.10.9` | MIT OR Apache-2.0 |
| `simd-adler32 v0.3.10` | MIT |
| `siphasher v1.0.3` | MIT/Apache-2.0 |
| `slab v0.4.12` | MIT |
| `smallvec v1.15.2` | MIT OR Apache-2.0 |
| `socket2 v0.6.5` | MIT OR Apache-2.0 |
| `softbuffer v0.4.8` | MIT OR Apache-2.0 |
| `stable_deref_trait v1.2.1` | MIT OR Apache-2.0 |
| `string_cache v0.9.0` | MIT OR Apache-2.0 |
| `strsim v0.11.1` | MIT |
| `syn v2.0.119` | MIT OR Apache-2.0 |
| `syn v3.0.3` | MIT OR Apache-2.0 |
| `sync_wrapper v1.0.2` | Apache-2.0 |
| `synstructure v0.13.2` | MIT |
| `tao v0.35.3` | Apache-2.0 |
| `tauri v2.11.5` | Apache-2.0 OR MIT |
| `tauri-codegen v2.6.3` | Apache-2.0 OR MIT |
| `tauri-macros v2.6.3 (proc-macro)` | Apache-2.0 OR MIT |
| `tauri-plugin-autostart v2.5.1` | Apache-2.0 OR MIT |
| `tauri-plugin-dialog v2.7.2` | Apache-2.0 OR MIT |
| `tauri-plugin-fs v2.5.1` | Apache-2.0 OR MIT |
| `tauri-plugin-log v2.9.0` | Apache-2.0 OR MIT |
| `tauri-plugin-notification v2.3.3` | Apache-2.0 OR MIT |
| `tauri-plugin-opener v2.5.4` | Apache-2.0 OR MIT |
| `tauri-plugin-single-instance v2.4.3` | Apache-2.0 OR MIT |
| `tauri-runtime v2.11.3` | Apache-2.0 OR MIT |
| `tauri-runtime-wry v2.11.4` | Apache-2.0 OR MIT |
| `tauri-utils v2.9.3` | Apache-2.0 OR MIT |
| `tauri-winrt-notification v0.7.3` | MIT OR Apache-2.0 |
| `tendril v0.5.1` | MIT OR Apache-2.0 |
| `thiserror v1.0.69` | MIT OR Apache-2.0 |
| `thiserror v2.0.20` | MIT OR Apache-2.0 |
| `thiserror-impl v1.0.69 (proc-macro)` | MIT OR Apache-2.0 |
| `thiserror-impl v2.0.20 (proc-macro)` | MIT OR Apache-2.0 |
| `time v0.3.55` | MIT OR Apache-2.0 |
| `time-core v0.1.9` | MIT OR Apache-2.0 |
| `time-macros v0.2.32 (proc-macro)` | MIT OR Apache-2.0 |
| `tinystr v0.8.4` | Unicode-3.0 |
| `tokio v1.53.1` | MIT |
| `tokio-macros v2.7.2 (proc-macro)` | MIT |
| `tokio-native-tls v0.3.1` | MIT |
| `tokio-util v0.7.19` | MIT |
| `toml v1.1.4+spec-1.1.0` | MIT OR Apache-2.0 |
| `toml_datetime v1.1.1+spec-1.1.0` | MIT OR Apache-2.0 |
| `toml_parser v1.1.3+spec-1.1.0` | MIT OR Apache-2.0 |
| `toml_writer v1.1.2+spec-1.1.0` | MIT OR Apache-2.0 |
| `tower v0.5.3` | MIT |
| `tower-http v0.6.11` | MIT |
| `tower-layer v0.3.3` | MIT |
| `tower-service v0.3.3` | MIT |
| `tracing v0.1.44` | MIT |
| `tracing-attributes v0.1.31 (proc-macro)` | MIT |
| `tracing-core v0.1.36` | MIT |
| `tray-icon v0.24.2` | MIT OR Apache-2.0 |
| `try-lock v0.2.5` | MIT |
| `typeid v1.0.3` | MIT OR Apache-2.0 |
| `typenum v1.20.1` | MIT OR Apache-2.0 |
| `unic-char-property v0.9.0` | MIT/Apache-2.0 |
| `unic-char-range v0.9.0` | MIT/Apache-2.0 |
| `unic-common v0.9.0` | MIT/Apache-2.0 |
| `unicode-ident v1.0.24` | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `unicode-segmentation v1.13.3` | MIT OR Apache-2.0 |
| `unic-ucd-ident v0.9.0` | MIT/Apache-2.0 |
| `unic-ucd-version v0.9.0` | MIT/Apache-2.0 |
| `url v2.5.8` | MIT OR Apache-2.0 |
| `urlpattern v0.3.0` | MIT |
| `utf8_iter v1.0.4` | Apache-2.0 OR MIT |
| `uuid v1.24.1` | Apache-2.0 OR MIT |
| `walkdir v2.5.0` | Unlicense/MIT |
| `want v0.3.1` | MIT |
| `web_atoms v0.2.6` | MIT OR Apache-2.0 |
| `webview2-com v0.38.2` | MIT |
| `webview2-com-macros v0.8.1 (proc-macro)` | MIT |
| `webview2-com-sys v0.38.2` | MIT |
| `winapi v0.3.9` | MIT/Apache-2.0 |
| `winapi-util v0.1.11` | Unlicense OR MIT |
| `windows v0.61.3` | MIT OR Apache-2.0 |
| `windows_x86_64_msvc v0.52.6` | MIT OR Apache-2.0 |
| `windows_x86_64_msvc v0.53.1` | MIT OR Apache-2.0 |
| `windows-collections v0.2.0` | MIT OR Apache-2.0 |
| `windows-core v0.61.2` | MIT OR Apache-2.0 |
| `windows-future v0.2.1` | MIT OR Apache-2.0 |
| `windows-implement v0.60.2 (proc-macro)` | MIT OR Apache-2.0 |
| `windows-interface v0.59.3 (proc-macro)` | MIT OR Apache-2.0 |
| `windows-link v0.1.3` | MIT OR Apache-2.0 |
| `windows-link v0.2.1` | MIT OR Apache-2.0 |
| `windows-native-keyring-store v1.1.0` | MIT OR Apache-2.0 |
| `windows-numerics v0.2.0` | MIT OR Apache-2.0 |
| `windows-registry v0.6.1` | MIT OR Apache-2.0 |
| `windows-result v0.3.4` | MIT OR Apache-2.0 |
| `windows-result v0.4.1` | MIT OR Apache-2.0 |
| `windows-strings v0.4.2` | MIT OR Apache-2.0 |
| `windows-strings v0.5.1` | MIT OR Apache-2.0 |
| `windows-sys v0.59.0` | MIT OR Apache-2.0 |
| `windows-sys v0.60.2` | MIT OR Apache-2.0 |
| `windows-sys v0.61.2` | MIT OR Apache-2.0 |
| `windows-targets v0.52.6` | MIT OR Apache-2.0 |
| `windows-targets v0.53.5` | MIT OR Apache-2.0 |
| `windows-threading v0.1.0` | MIT OR Apache-2.0 |
| `windows-version v0.1.7` | MIT OR Apache-2.0 |
| `window-vibrancy v0.6.0` | Apache-2.0 OR MIT |
| `winnow v1.0.4` | MIT |
| `winreg v0.10.1` | MIT |
| `writeable v0.6.4` | Unicode-3.0 |
| `wry v0.55.1` | Apache-2.0 OR MIT |
| `yoke v0.8.3` | Unicode-3.0 |
| `yoke-derive v0.8.2 (proc-macro)` | Unicode-3.0 |
| `zerocopy v0.8.56` | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zerofrom v0.1.8` | Unicode-3.0 |
| `zerofrom-derive v0.1.7 (proc-macro)` | Unicode-3.0 |
| `zeroize v1.9.0` | Apache-2.0 OR MIT |
| `zerotrie v0.2.5` | Unicode-3.0 |
| `zerovec v0.11.8` | Unicode-3.0 |
| `zerovec-derive v0.11.6 (proc-macro)` | Unicode-3.0 |
| `zmij v1.0.23` | MIT |

## npm-Laufzeitabhängigkeiten

Gebündelt in `dist/`: **11** Pakete. Devabhängigkeiten
(Vite, TypeScript, vitest, Tailwind) werden nicht ausgeliefert.

| Paket | Lizenz |
|---|---|
| `@tanstack/react-virtual@3.14.10` | MIT |
| `@tanstack/virtual-core@3.17.8` | MIT |
| `@tauri-apps/api@2.11.1` | Apache-2.0 OR MIT |
| `@tauri-apps/plugin-autostart@2.5.1` | MIT OR Apache-2.0 |
| `@tauri-apps/plugin-log@2.9.0` | MIT OR Apache-2.0 |
| `@tauri-apps/plugin-notification@2.3.3` | MIT OR Apache-2.0 |
| `@tauri-apps/plugin-opener@2.5.4` | MIT OR Apache-2.0 |
| `lucide-react@1.33.0` | ISC |
| `react@19.2.8` | MIT |
| `react-dom@19.2.8` | MIT |
| `scheduler@0.27.0` | MIT |

