# Changelog

## v3.3.0

- Remove exact harness and companion version whitelists. Supported commands
  and file schemas work across versions; runtime failures remain visible.
- Preserve shared global skills when removing only one harness binding.
- Report failed or partial skill publication instead of claiming success.
- Resolve relative local skill directories and retain their complete contents.
- Update only desired harness bindings, retain sequentially added targets, and
  preserve independent source repositories when updating across scopes.
- Show satisfied skills as no-change plans and compare global skill status.
- Allow declaration-managed installation into fresh configuration directories.

## v3.2.0

### Changes

- Replace the already-deprecated `gemini` target with `agy` for Antigravity CLI.
  Existing `gemini` configuration and inventory must be retargeted explicitly;
  this release provides no alias or automatic migration.

### Features

- Add an exact Antigravity CLI 1.1.13 profile with complete global and project
  skill publication.
- Add declaration-managed global MCP projection through
  `~/.gemini/config/mcp_config.json`, with preservation, drift detection,
  rollback, and repeat-idempotence guarantees.
- Keep project MCP, native plugin lifecycle, and unverified transports blocked
  until Antigravity documents contracts skilltap can bind and observe safely.

## v3.1.0

### Features

- Register the expanded target set with per-component support tiers: verified
  Codex, Claude Code, Factory Droid, Qwen Code, Gemini CLI, and OpenCode
  profiles; mixed GitHub Copilot CLI support; declaration-managed Kiro, Kimi,
  Mistral Vibe, Kilo Code, Junie, and Amp surfaces; and observe-only Pi,
  Cursor, Zoo Code, and ZCode profiles.
- Project managed plugin projection is target-agnostic for adapters with a safe
  complete-skill or MCP surface, while ownership, drift checks, rollback, and
  status remain bound to the exact selected target.
- Project skills use one canonical `.agents/skills/<name>/` tree and
  registry-derived target links or no-link projections where each harness
  requires them.
- Daemon update runs refresh registered marketplaces before resolving managed
  plugin versions and Git-backed skill revisions.

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
