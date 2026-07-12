---
id: epic-instruction-reconciliation-repair
kind: feature
stage: done
tags: []
parent: epic-instruction-reconciliation
depends_on: [epic-instruction-reconciliation-global, epic-instruction-reconciliation-project]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Repair Instruction Bridges Safely

Plan canonical-wins repairs, exact acknowledgment, symlink/import mode, and
recoverable backups through the existing lock and filesystem boundaries.

## Acceptance

No repair overwrites divergent or unmanaged content without named approval;
repeated repair is a no-op and failed publication leaves safe residual evidence.

## Implementation notes

Added explicit instruction repair planning: missing bridges create, managed
bridges no-op, divergent/duplicate bridges block without acknowledgment, and
approved divergence requires a recoverable backup before repair. Filesystem
publication remains behind the existing managed filesystem ports.

## Review

### Verdict

Approve with comments.

### Findings

- Command composition must map the operation-scoped acknowledgment and bridge
  mode to concrete symlink/import publication.

### Verification

Instruction repair model and native-config preservation tests pass under strict
clippy.
