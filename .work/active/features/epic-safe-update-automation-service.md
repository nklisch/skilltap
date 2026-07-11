---
id: epic-safe-update-automation-service
kind: feature
stage: drafting
tags: []
parent: epic-safe-update-automation
depends_on: [epic-safe-update-automation-foreground]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Integrate User Update Services

Install finite launchd/systemd-user timers that invoke one bounded daemon cycle
using the same update application service as foreground commands.
