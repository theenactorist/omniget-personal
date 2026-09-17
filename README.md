# OmniGet Personal

Personal macOS build of [OmniGet](https://github.com/tonhowtf/omniget), based on
version 0.9.2, commit `8600b91f4246848bac346874daa9e61c1fc5677a`.
This repository is intended to remain private and is a standalone source snapshot.
Upstream credits and GPL-3.0 licensing are preserved in [README.upstream.md](README.upstream.md)
and [LICENSE](LICENSE).

## Personal changes

- Separate app identity and private data directory: `local.omniget.personal`.
- Main app updates disabled; update this build by reviewing and rebuilding source.
- Plugin startup loads the installed set; Marketplace installs and updates are manual.
- Browser extension requires manual token pairing; unauthenticated token discovery is disabled.
- Background browser cookie collection removed; explicit export/download actions remain available.
- Extension credential requests are restricted to HTTP loopback and reject redirects.
- Discord activity sharing is off by default.
- Local Apple Metal dependency compatibility patch retained under `vendor/apple-metal`.

## Source and plugins

This includes the application source, browser extensions, lockfiles, tests, and
vendored dependency source. Courses, Telegram, Convert, and Study were installed
and startup-tested separately on the original Mac. Their precompiled binaries,
user data, cookies, accounts, and build caches are not stored in this repository.
Study is proprietary; its source code is not available in the upstream distribution.

## Build and verification

See [PERSONAL-BUILD.md](PERSONAL-BUILD.md) for prerequisites, build notes, security
limits, and the completed verification. Rust is pinned to 1.97.0 because native
plugins require the host's exact compiler fingerprint.

The original build passed 104 frontend tests, 426 extension tests, and four native
regression tests. The installed app opened and loaded all four plugins. The
browser bridge rejected unauthenticated queue access and automatic token pairing.

Upstream GitHub Actions are archived in `.github/upstream-workflows/` for reference;
there are no active workflows. Review them before enabling any automation.

## Provenance

`provenance/SOURCE.json` records the unauthenticated source archive and its hash.
`provenance/personal-changes.patch` and `provenance/PERSONAL-CHANGES.json` record the
local application changes against that archive, including the vendored dependency.
Those records precede this repository's README, ignore rules, and workflow relocation.

The original download used no fork or authenticated GitHub account. Storing this
copy in a private GitHub repository associates it with the repository owner's
account inside GitHub; it is not an anonymity mechanism.
