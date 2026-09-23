---
id: 36
title: 'Mission targets: --target leo instead of raw m/s'
type: feature
status: backlog
milestone: v0.9
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: m
area: physics
---

## Proposal

`--target leo|sso|gto|gso|tli|mars` resolves to a documented Δv from a surface-to-destination budget,
still overridable with `--target-dv`. `tsi missions` prints the Δv map as a reference table with
sources, which is useful on its own.

## Acceptance criteria

- [ ] Δv map in `data/missions.toml`, embedded with `include_str!`
- [ ] Each entry cites a source
- [ ] Output names the mission alongside the number
