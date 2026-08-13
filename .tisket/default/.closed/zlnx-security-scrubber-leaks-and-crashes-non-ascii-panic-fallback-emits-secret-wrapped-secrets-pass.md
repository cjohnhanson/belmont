---
title: "SECURITY: scrubber leaks and crashes — non-ASCII panic, fallback emits secret, wrapped secrets pass"
status: done
priority: 1
assignee:
labels: [security, bug]
depends_on: []
created: 2026-08-13T02:06:36Z
updated: "2026-08-13T17:01:28Z"
---

src/scrub.rs: (1) buffer[emit_len..] slices on a byte index that can fall inside a multibyte char — non-ASCII output panics/truncates (confirmed: output truncated mid-stream on a préfix...ααα line). (2) The documented fallback (strip_suffix fails) emits the full scrubbed buffer but the boundary logic can emit an unscrubbed secret prefix. (3) scrub_text uses literal .replace(value), so a line-wrapped or chunk-split secret is never matched and passes through verbatim. For a secret-scrubbing tool these are the defects that matter most. Fix: operate on char boundaries; the boundary-buffer must hold back max_secret_len bytes and match across the boundary, not rely on contiguous literal replace.
