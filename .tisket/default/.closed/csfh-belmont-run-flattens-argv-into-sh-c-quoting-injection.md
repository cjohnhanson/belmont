---
title: "belmont run flattens argv into sh -c (quoting/injection)"
status: done
priority: 2
assignee:
labels: [bug]
depends_on: []
created: 2026-08-13T02:06:36Z
updated: "2026-08-13T17:58:13Z"
---

src/runner.rs:29-31 builds sh -c with command.join(' '), so arguments with spaces or shell metacharacters are re-split and re-interpreted. Fix: exec the command directly (Command::new(argv[0]).args(argv[1..])) without a shell.

## Scratch Notes

WONTFIX / by-design: the sh -c is intentional. belmont injects secrets as env vars and the command references them via $VAR; the shell must expand them before belmont scrubs the expanded value from output. The runner unit tests (echoed_secret_is_scrubbed, multiple_secrets_scrubbed) and the missouri run assertion all depend on this expansion. Removing the shell breaks belmont's core feature. belmont's threat model is accidental secret leakage, not command injection from the command the user themselves passes — the user already controls that argv. The review finding was a false positive.
