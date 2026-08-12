---
title: "missouri suite: belmont.yml fixture trailing newline mismatch vs serde_yml output"
status: todo
priority: 3
assignee:
labels: [tests]
depends_on: []
created: "2026-08-12T21:15:24Z"
updated: "2026-08-12T21:15:24Z"
---

The suite was unrunnable until the bin shim path fix (stale extraction path, now corrected). With the shim fixed, the single path fails: the fixture belmont.yml ends with a blank line and the serde_yml output does not. Regenerate the fixture from the real command output.
