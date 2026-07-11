---
id: bug-cli-bare-help
kind: story
stage: backlog
tags: [bug, cli]
parent: null
depends_on: []
release_binding: null
gate_origin: tests
created: 2026-07-11
updated: 2026-07-11
---

# Bare CLI Omits Concise Help

Compiled-binary integration found that bare `skilltap` exits with the correct
input-error code `1` but emits only the normalized missing-command error. The
approved UX and CLI-shell acceptance require concise help/usage plus exit `1`.
