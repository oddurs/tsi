---
id: 1
key: v0.7
title: Static fire
type: milestone
status: done
created: 2026-09-22
updated: 2026-09-22
priority: p2
due: 2026-10-06
---

The engine lights on the test stand before anything is stacked on it.

Every number tsi prints must be right, and the build must prove it on every
push. That means fixing the optimizer and Monte Carlo bugs found in the v0.6
audit, adding the tests that would have caught them, and getting CI green.

**Exit criteria:** analytical and brute-force optimizers agree within 1%.
The optimizer reproduces Falcon 9 and Saturn V within stated tolerances.
CI enforces fmt, clippy, tests and `cairn check`.
