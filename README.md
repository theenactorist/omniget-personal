# Origo Downloads

Personal macOS downloader, currently version **0.9.6**, derived from OmniGet
0.9.2 at commit `8600b91f4246848bac346874daa9e61c1fc5677a`.
This repository is intended to remain private. Upstream credits and GPL-3.0
licensing are preserved in [README.upstream.md](README.upstream.md) and
[LICENSE](LICENSE).

## Current desktop behavior

- Instagram and the retained media downloaders remain available.
- Local video compression includes a WhatsApp preset that preserves the whole
  clip and targets a file below 16 MB.
- Completed downloads and History show the recorded filename and location, with
  a **Show in folder** button. New completion toasts provide the same action.
- The visible app and bundled browser-extension names are **Origo Downloads**.
  The bundle ID `local.omniget.personal`, existing data directory, executable,
  browser pairing, and `omniget-personal://` scheme are retained for continuity.
- Main-app automatic updates remain disabled; updates require reviewed source
  and a local rebuild.

## Privacy changes

Native plugins are disabled pending review. Telegram, Courses, Library, Music,
League, chat, P2P sharing, and Marketplace entry points are removed or blocked.
The corresponding native startup paths and commands are disabled, including
Discord activity sharing. Known author-service endpoints are blocked in the
reviewed URL paths; release notes are bundled locally.

The browser extension uses explicit token pairing. Automatic token discovery
and background cookie collection are disabled. Credential-bearing extension
requests are restricted to HTTP loopback and reject redirects. Renaming the
bundled extension does not modify an extension already installed in a browser.

This is source-level hardening, not a universal network firewall or security
certification. Downloads contact the requested service and its media hosts.
See [PERSONAL-BUILD.md](PERSONAL-BUILD.md) for limitations and build notes.

## Source and verification

This repository contains source, browser extensions, lockfiles, tests, and
vendored dependency source. It excludes app installers, native plugin binaries,
downloaded helpers, cookies, tokens, personal settings, media, and build caches.

The 0.9.6 source passed **149 frontend tests** and **426 browser-extension tests**.
The Svelte/type check reported zero errors and 88 existing warnings. The native
Apple Silicon release build, local signing, installed app and persisted-state
checks also passed; private runtime records are excluded from this repository.
The Instagram and WhatsApp implementations remain byte-identical to the reviewed
0.9.5 versions. The rename preserves the prior privacy controls.

Upstream GitHub Actions remain archived in `.github/upstream-workflows/`; no
active workflow is enabled. The separate local Android branch is not part of
this desktop update.

## Provenance

`provenance/SOURCE.json` records the original unauthenticated source archive and
its hash. `provenance/personal-changes.patch` and
`provenance/PERSONAL-CHANGES.json` describe the initial personal build; later
desktop changes are recorded in Git history. Those initial records are not a
manifest of the current release.

The original upstream import used no fork or authenticated GitHub account.
Keeping this copy in a private GitHub repository associates it with the owner's
GitHub account; private storage is not an anonymity mechanism.
