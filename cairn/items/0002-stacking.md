---
id: 2
key: v0.8
title: Stacking
type: milestone
status: backlog
depends_on:
- 1
created: 2026-09-22
updated: 2026-09-22
priority: p2
due: 2026-10-27
---

The vehicle is assembled: a library that can be depended on.

tsi becomes a library with a CLI rather than the reverse. The CLI is feature-gated,
the library never prints or panics on bad input, errors are typed, results
serialize, and the public surface is shaped to survive 1.0 semver. Almost every
breaking change lands here, on purpose, so v0.9 and v1.0 can be additive.

**Exit criteria:** `cargo add tsiolkovsky --no-default-features` pulls in no clap.
No reachable panic from public input. The `breaking` view is empty for this milestone.
