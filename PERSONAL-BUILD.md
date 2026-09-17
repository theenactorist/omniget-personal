# Personal build notes

## Prerequisites

The verified target is Apple Silicon macOS. The original build used Node 22.17.1,
Rust 1.97.0 (pinned in `rust-toolchain.toml`), and Apple's Command Line Tools. Native
plugin compatibility requires the same Rust compiler fingerprint as the plugin.
Use an Apple SDK/toolchain that can build the Swift dependencies.

With Node, npm, Rust/rustup and Apple build tools available, the source's build
entry points are:

```sh
npm ci --ignore-scripts
npm run check
npm test
npm run tauri -- build --bundles app --no-sign --target aarch64-apple-darwin -- --locked
codesign --force --deep --sign - --entitlements src-tauri/Entitlements.plist \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/OmniGet Personal.app"
```

The application bundle is written under
`src-tauri/target/aarch64-apple-darwin/release/bundle/macos/`.
These commands are entry points, not a claim of a clean-machine rebuild test.

## Apple toolchain compatibility

The original Mac had Swift 6.1.2 and SDK 15.5. Its Command Line Tools included stale
private Swift interfaces and a duplicate obsolete Swift module map. The successful
local build used project-local copies/overlays to omit those stale files, with
`SWIFTPM_CUSTOM_LIBS_DIR`, `CLANG_MODULE_CACHE_PATH`, and
`SWIFTPM_MODULECACHE_OVERRIDE` pointing to local build caches. The final linker
also needed `LIBRARY_PATH` to include
`/Library/Developer/CommandLineTools/usr/lib/swift/macosx`, because a dependency
assumed a full Xcode directory layout. None of the system toolchain files were modified.
Those machine-specific cache copies are not included in this repository.

The source-level fix is retained in `vendor/apple-metal`: the build detects the
SDK and compiles macOS 26 sampler properties only when available. OmniGet's current
features do not use those properties. Both upstream vendor licenses are retained.

## Plugins and helpers

The original local installation separately included Courses 1.1.0, Telegram 1.0.2,
Convert 1.0.3, and Study 0.3.6. All four loaded successfully. These are upstream
precompiled native plugins; their source was not audited or rebuilt during packaging.
Study is proprietary. A fresh source checkout does not contain or automatically
install these plugins; use explicit Marketplace actions to install compatible versions.

The local app used verified official yt-dlp 2026.08.19 plus existing FFmpeg, FFprobe,
and Node installations. PDFium came from the Study package. Native helpers and
optional models may be installed when a feature is explicitly requested.

## Verification completed on the original Mac

- Apple Silicon release build and strict ad-hoc code-signature verification passed.
- Frontend: 104 tests passed; type/Svelte check had zero errors and 91 existing warnings.
- Browser extension: 426 tests passed.
- Targeted native regressions: four passed (plugin state, URI handling, token pairing).
- Installed app created its window, loaded all four plugins, and exited cleanly in smoke mode.
- Bridge health and authorized queue reads succeeded; unauthenticated queue access
  returned 401; automatic pairing returned 403 without disclosing a token.
- yt-dlp passed a localhost HTTP download with byte-identical output.
- The installed GUI opened on its setup screen.

Logged-in services, paid features, optional model downloads, and every plugin
operation were not exercised. No browser extension was installed into an existing
browser profile during packaging. Export and load the bundled extension manually,
then copy the endpoint and token from app Settings to its options.

## Remaining security limits

This is an ad-hoc signed personal build, not an Apple-notarized distribution.
The upstream disabled CSP and broad local-file asset scope remain. Native plugins
run with the user's permissions and retain their own dependency-management behavior.

Imported cookies, bridge tokens, proxy credentials, and some download headers are
stored in plaintext under local file permissions. The extension also retains
pairing and last-download data in browser storage. Software running as the same
user can access those files. Future Marketplace downloads do not verify signatures
or checksums; the original packaged asset checks covered only the pinned downloads.
This is a limited code review and hardening exercise, not a security certification.
