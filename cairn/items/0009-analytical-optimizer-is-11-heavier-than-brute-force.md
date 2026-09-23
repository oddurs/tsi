---
id: 9
title: Analytical optimizer is 11% heavier than brute force
type: bug
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
depends_on:
- 8
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p0
effort: l
area: optimizer
---

## What happens

`tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2` returns 205.4 t.
With `--optimizer brute-force` it returns 182.8 t. The "optimal" solver loses.

Three causes:
1. The 2% hidden margin (see the margin item).
2. The equal-Δv split is only optimal when every stage has the same Isp and structural
   fraction *and no fixed mass*. Engine dry mass is fixed per engine, which breaks it.
   The true optimum is near 4,370 / 5,030 m/s, not 4,700 / 4,700.
3. Stage 1 gets 2 engines when 1 meets min TWR (1.26 ≥ 1.2).

The engine-count `loop`s at `analytical.rs:251` and `:292` also have no iteration
cap and can oscillate forever.

## What should happen

Solve the staging problem properly: Lagrange multiplier condition for N stages with
differing Isp and ε, solved numerically (1-D root find on the multiplier), then search
the discrete engine count around the continuous optimum. Document the derivation in
rustdoc. It is the most educational code in the crate and should read that way.

## Acceptance criteria

- [x] Analytical ≤ brute force + 1% on the README example and on the property test below
- [x] Engine-count search is bounded; no `loop` without an exit
- [x] Works for N stages, not just 2
- [x] Rustdoc derivation with the textbook reference (Curtis, *Orbital Mechanics for Engineering Students*, §11.6)

## 2026-09-22

Lagrange multiplier split (Curtis §11.6) as the start, then pairwise scan + golden-section refinement against the exact mass model with engine mass. Engine counts come from a closed-form TWR bound (src/optimizer/sizing.rs), so no engine-count loops remain. Handles any stage count and every engine-to-stage assignment. README example: 188.9 t, split 4,407/4,993 m/s under the new ascent-averaged booster Isp. Stress-tested against brute force over 300 random cases: analytical never more than 1% heavier.

## 2026-09-22

Found while validating on Saturn V: with no floor on first-stage delta-v, the optimum degenerates to a propellant-free 'booster' that supplies liftoff thrust while vacuum-only upper stages do everything from sea level. Added Constraints::min_booster_delta_v (2,000 m/s, Earth launches with 2+ stages only).
