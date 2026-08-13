---
title: "belmont run flattens argv into sh -c (quoting/injection)"
status: todo
priority: 2
assignee:
labels: [bug]
depends_on: []
created: "2026-08-13T02:06:36Z"
updated: "2026-08-13T02:06:36Z"
---

src/runner.rs:29-31 builds sh -c with command.join(' '), so arguments with spaces or shell metacharacters are re-split and re-interpreted. Fix: exec the command directly (Command::new(argv[0]).args(argv[1..])) without a shell.
