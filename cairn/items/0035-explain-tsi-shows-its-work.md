---
id: 35
title: '--explain: tsi shows its work'
type: feature
status: backlog
milestone: v0.9
depends_on:
- 31
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: l
area: output
---

## Problem

tsi's concept doc promises it is "Educational: shows its work." Today it prints answers.

## Proposal

`--explain` on `calculate` and `optimize` prints the derivation with this problem's numbers substituted:
the rocket equation per stage, why the Δv split landed where it did (the Lagrange condition), which
constraint was binding, and what would move the answer. The library produces a structured
`Explanation` (steps with formula, substitution and result), and the CLI renders it. Other front
ends (JSON, a future TUI) can too.

## Acceptance criteria

- [ ] Every number in the explanation is traceable to a field in the solution
- [ ] Explains the binding constraint in plain language
- [ ] Snapshot tests for two canonical problems
- [ ] Available as `"explanation"` in JSON output
