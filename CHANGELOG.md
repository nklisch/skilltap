# Changelog

## v3.0.3

### Fixes

- Make managed-project lifecycle tests honor their disabled-observation fixture
  mode, eliminating accidental dependence on a developer-installed Codex binary
  while preserving production post-mutation observation.

## v3.0.2

### Features

- Complete real-harness recovery for managed Codex project projection, native
  lifecycle postconditions, instruction repair, and actionable diagnostics.
- Show people how to delegate high-level environment management to an agent
  while skilltap supplies plans, structured output, and explicit decisions.

### Fixes

- Prevent blocked or unsupported plugin plans from publishing inventory or
  state before a faithful operation exists.
- Recover exact managed projection manifests after interrupted journal writes
  without duplicate publication, including Git SHA updates.
- Report whether managed-skill rollback restored the prior tree or left an
  exact residual destination requiring recovery.
- Require successful instruction repair and safe native remove retries to
  complete with truthful repeat-no-op results.

### Security

- Confine managed project reads, writes, removals, and rollback beneath a
  descriptor-bound root so hostile ancestor symlinks cannot redirect them.
- Enforce depth, entry, per-file, total-byte, and document limits during both
  planning and locked execution, including post-plan hostile growth.

### Documentation

- Clarify plugin-first installation, warm the website language, and add
  agent-directed examples such as “Use skilltap to sync…” across the landing
  page and guides.
