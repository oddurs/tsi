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

## 2026-09-22

Post-release code review (2026-09-22) found 10 issues; all fixed on this branch before merge. (1) Monte Carlo and --show-losses judged liftoff TWR at g0 whatever the launch body: Rocket now stores surface gravity. (2,3) Vacuum-only engines offered or pinned for an Earth first stage gave misleading structural or 'engines too heavy' errors: Problem::is_valid rejects them by name. (4) Exceeding the engine-assignment cap at a high stage count threw away designs from lower counts. (5) --uncertainty none reported 1 build and used a different success tolerance. (6) Upper-stage TWR failures pointed at --min-twr; failures now record their stage, and diagnosis comes from the Lagrange starting split rather than whatever extreme split the scan hit last; brute force defers to the analytical optimizer for its diagnosis. (7) Pinned stage index cast with 'as u32'. (8) TWR slack rounded requirements just above an integer up to an extra engine. (9) Ascent-averaged Isp was a plain time average; the rocket equation weights by 1/m, so the mean pressure ratio is 0.21 not 0.31. Constant is now 0.2: Merlin 305 s, Raptor 345 s. README example is now 186.6 t; the Falcon 9 redesign is +1.9%; Saturn V is 29.5% lighter than the real vehicle. (10) Thrust-by-IspModel duplicated; now Engine::thrust_for.
