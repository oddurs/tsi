---
id: 45
title: Public API review and freeze
type: chore
status: backlog
milestone: v1.0
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: m
area: docs
breaking: 'true'
---

Read every public item as a stranger would.

## Acceptance criteria

- [ ] `#![warn(missing_docs)]` with zero warnings
- [ ] Each module's rustdoc opens with the physical idea before the code
- [ ] `[package.metadata.docs.rs]` set; all features documented with `doc_cfg`
- [ ] The `breaking` view is empty
