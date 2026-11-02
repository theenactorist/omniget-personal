# Origo Downloads: personal desktop build

Current source version: **0.9.6**. The verified platform for this project is
Apple Silicon macOS. This document describes the desktop source; it does not
include private installation records or downloaded media.

## Prerequisites and build entry points

Use Node.js, npm, Rust/rustup, and an Apple SDK/toolchain able to compile the Swift
dependencies. Rust remains pinned to **1.97.0** in `rust-toolchain.toml` for
reproducibility. Native plugins remain disabled even though their host interfaces
and dependency source are retained.

From the repository root:

```sh
npm ci --ignore-scripts
npm run check
npm test
node --test browser-extension/chrome/tests/*.test.mjs
npm run tauri -- build --bundles app --no-sign --target aarch64-apple-darwin -- --locked
codesign --force --deep --sign - --entitlements src-tauri/Entitlements.plist \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Origo Downloads.app"
```

The output lives under
`src-tauri/target/aarch64-apple-darwin/release/bundle/macos/`.
These are build entry points, not a claim that every clean machine has been tested.
Ad-hoc signing is for a personal local build; it is not Apple notarization.

The source-level Apple Metal compatibility fix is retained in
`vendor/apple-metal`: SDK detection guards macOS 26 sampler properties when an
older SDK is used. Upstream vendor licenses are retained. The original build
environment also used local Swift module-cache/manifest workarounds and a Swift
compatibility-library search path. These machine-specific caches and toolchain
copies are not part of this repository; do not copy another machine's caches.
The local release uses matching Command Line Tools Swift 6.1.2 and macOS SDK
15.5, selected through process-local `DEVELOPER_DIR` and `SDKROOT`. If using a
compiler wrapper, keep its SDK on the same toolchain; a newer global Xcode
selection must not supply an incompatible SDK. Global Xcode settings need not
be changed.

## Preserve application identity during installation

The visible name changed from OmniGet Personal to Origo Downloads. Keep these
internal values unchanged when rebuilding or installing an update:

- Bundle ID and data-directory identity: `local.omniget.personal`.
- Executable and Cargo package: `omniget`; library: `omniget_lib`.
- Deep-link scheme: `omniget-personal://`.
- Existing browser-extension IDs, protocol identifiers, and pairing keys.
- Autostart's legacy internal name: `OmniGet Personal`.

The normal macOS data directory is
`~/Library/Application Support/local.omniget.personal`. Renaming the app should
not copy, reset, or delete that directory or its WebKit state. If an existing
LaunchAgent is configured, its executable path must follow the new installed
bundle path; do not enable autostart as a side effect of the rename. Renaming the
bundled extension does not reload or alter an existing browser profile.

## Included behavior and disabled components

Instagram downloads, retained downloaders, saved-location feedback, and local
video compression are kept. The WhatsApp preset targets an output below 16 MB
while preserving the clip's duration. Compression quality depends on duration
and source complexity; it does not promise the original resolution or quality.

The 0.9.4 privacy changes remain active in 0.9.6:

- Native plugin loading, installation, updates, commands, and frontend access are
  blocked. Precompiled Courses, Telegram, Convert, and Study binaries are absent.
- Telegram, Courses, Library, Music, League, chat, P2P, and Marketplace routes and
  entry points are removed or blocked. Voice workers do not initialize.
- Discord activity sharing and author-service startup connections are disabled.
- Known author endpoints are blocked in reviewed URL/redirect paths; app updates
  are disabled and release notes are bundled locally.
- Browser-extension pairing is manual, and credential-bearing bridge requests
  stay on HTTP loopback without redirects. Background cookie collection is off.

Downloads can still use explicitly configured helpers such as yt-dlp, FFmpeg,
and FFprobe. Helpers, optional model downloads, and their binaries are not
included here. Existing native-helper paths must not be redirected into disabled
plugin directories. Do not restore plugin loading to resolve a dependency error.

## Source verification and limitations

The current source passed 149 frontend tests and 426 extension tests. The
Svelte/type check found zero errors and 88 existing warnings. Focused regressions
cover privacy gates, manual pairing, download-location parsing, completion-toast
deduplication, and reveal failures. The Apple Silicon release build, local code
signature, installed name/version, existing History display, local bridge
authentication and disabled-plugin checks also passed. The checked persisted
state remained byte-identical through installation and launch. This repository
does not include private runtime records. Instagram and compression were not
rerun for the display-name change; their implementations remain unchanged.

These controls are not a universal firewall over third-party extractor requests.
Instagram downloads contact Instagram and its media hosts; local compression
does not need a remote backend. The upstream disabled CSP and broad local-file
asset scope remain limitations.

Imported cookies, bridge tokens, proxy credentials, and some download headers
are stored locally under filesystem permissions. The extension retains pairing
and download state in browser storage. Software running as the same OS user can
access those files. Never commit those values, settings, histories, or downloaded
media. This is a limited source review and hardening effort, not a security
certification.
