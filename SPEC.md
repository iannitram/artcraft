# ArtCraft (our fork): spec

**Approval:** 2026-10-26, the owner: "The specs/roadmaps were developed by someone else. Nomalize them to our system" (spec = upstream roadmap + our delta, R18).

## The spec is upstream's roadmap

ArtCraft is storytold's "IDE for artists": stage a scene by hand and generate from it with outside image, video and 3D models. We follow upstream's plan:

- 2026 goals: https://github.com/storytold/artcraft/blob/main/ROADMAP.md (product, architecture: "Remove dependence on ArtCraft hosted services", "More Rust, more speed", "Code cleanup, more/better tests, CI")
- The pure-Rust desktop app and its parity notes: https://github.com/storytold/artcraft/blob/main/artcraft-native/README.md

**1.0 for us**: the pure-Rust desktop app (`artcraft-native`) has every tool the Tauri app has, with no "coming soon" pages, and its tests run on Windows and macOS in CI.

## Our delta

| Item | Where | Disposition |
|---|---|---|
| The 95% feature list and a golden task per platform for the owner's real work | #3, #4 | keep on our fork (needs the owner) |
| Which generation services are in scope and what they may cost | #6 | the owner's decision |
| A Linux build | #5 | the owner's decision; upstream has Linux packaging in PR #2561 |
| Batch download never overwrites (sibling sweep) | #2 | upstream PR #2549 covers the same: check it before building ours |
| Tests in CI on Windows and macOS | build issue | send upstream (to confirm with the owner) |

Upstream PRs need the owner's yes, one by one; no credit beyond the username (pinned issue #7).
