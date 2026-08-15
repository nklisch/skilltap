---
id: feature-replace-gemini-with-agy
kind: feature
stage: done
tags: []
parent: null
depends_on: []
release_binding: 4.0.0
research_refs:
  - .research/analysis/briefs/antigravity-cli-1.1.13.md
research_origin: operator-request-2026-08-15
gate_origin: null
created: 2026-08-15
updated: 2026-08-15
---

# Replace deprecated Gemini support with Antigravity CLI

Replace the registered `gemini` target with `agy`. Remove the Gemini adapter,
profile, fixtures, examples, and current-product claims. Do not retain an alias,
migration layer, or parallel Gemini support.

## Requirements

- Register target `agy` with display name `Antigravity CLI` and executable
  `agy`.
- Pin mutation authority to Antigravity CLI `1.1.13`. Unknown versions remain
  observe-only.
- Use Antigravity's documented global and project skill roots.
- Manage only MCP declarations whose scope and schema are documented.
- Keep declared state separate from effective state when no non-interactive
  observer exists.
- Preserve unknown native fields, detect ownership drift, and make repeated
  mutation a no-op.
- Remove `gemini` from every current registry, CLI, fixture, website, and
  foundation surface.
- Ship the breaking target replacement as `v4.0.0`.

## Design

**Primary lens:** Data, migration, or integration, with compatibility and testing overlays.

### Outcome and constraints

Antigravity CLI is a distinct native control plane with executable `agy`,
version stream `1.1.x`, and global customization root `~/.gemini/config/`.
The existing Gemini adapter targets executable `gemini` version `0.50.0`, uses
different skill and MCP paths, and invokes a command that AGY does not expose.
Reusing that adapter would create false reconciliation claims.

The replacement is intentionally breaking. Existing `gemini` configuration and
inventory remain ordinary unregistered-target input. skilltap does not rewrite
user state or preserve a deprecated target alias.

### Chosen approach

Create one exact-profile AGY adapter using the existing
configuration-constrained managed-projection machinery. The profile supports
complete skills at global `~/.gemini/config/skills/` and project
`.agents/skills/` roots. Global MCP declarations use
`~/.gemini/config/mcp_config.json`. Project standalone MCP is unsupported
because the installed detailed MCP contract documents only global and
plugin-bundled declarations.

AGY MCP writes are declaration-managed. The adapter preserves unowned servers
and unknown document fields, fingerprints owned entries, blocks drift, and
reports effective state as unverified. It maps documented stdio servers and SSE
servers with `serverUrl`. It rejects unproven transports and authentication
shapes.

The installed CLI exposes native plugin install, uninstall, enable, disable,
list, import, validate, and link commands. Isolated tests confirm global local
plugin installation and JSON list output when plugins exist. The current
lifecycle execution port cannot bind an install to the resolved plugin checkout,
and marketplace identity and update behavior remain undocumented. This release
therefore keeps native plugin lifecycle unsupported rather than weakening the
plan-to-execution binding. Managed projection still transfers compatible skills
and global MCP components.

Observe AGY's documented global and project customization roots and relevant
configuration files. Do not manage alternate `.agent`, `_agents`, or `_agent`
roots, rule frontmatter, hooks, declared search paths, or plugin enablement in
this replacement. Those surfaces lack either a required skilltap component
contract or sufficient effective-state evidence.

### Alternatives

Keeping `gemini` as an alias was rejected because the user requires AGY-only
support and the native contracts differ. Renaming the old adapter without
changing paths was rejected because it would write declarations AGY does not
load. Adding native plugin lifecycle now was rejected because the execution
port cannot bind AGY's directory install to the resolved checkout without a
broader lifecycle contract change.

### Implementation units

1. Add the AGY adapter, declaration profile, skill projection, MCP codec, and
   focused unit tests. Remove the Gemini adapter and projection.
2. Replace registry, acceptance, fake-binary, storage, and compiled-binary
   fixtures. Prove exact-profile mutation, unknown-version refusal, scope
   behavior, preservation, drift detection, and repeat idempotence.
3. Replace Gemini with AGY in foundation docs, CLI examples, website reference,
   generated documentation, plugin guidance, and release metadata.
4. Run focused and workspace verification, independent implementation review,
   release gates, and shipment checks.

### Verification

- AGY `1.1.13` is detected and receives only the documented capability set.
- Adjacent and unknown versions cannot write.
- Global and project skills reach the correct roots.
- Global MCP writes preserve unrelated data and repeat as no-ops.
- Project MCP remains unsupported and cannot write.
- `harness list` contains `agy` and not `gemini`.
- The registry, CLI, fixtures, examples, foundation docs, and website no longer
  name Gemini as a supported target. Historical changelog entries and Qwen's
  Gemini source-format reader remain intact.
- `docs/HARNESS-CONTRACTS.md` records the exact AGY `1.1.13` scope matrix.
  `docs/SPEC.md` and the website state the declaration-managed MCP tier.
- Pattern references point to a live managed-projection implementation.
- The full Rust, formatting, lint, package, website, and release checks pass.

### Risks and recovery

AGY has a fast release cadence. Exact profile pinning turns new versions into
observe-only targets until their contract is refreshed. AGY shares the
`~/.gemini` parent with other products, so the adapter confines all writes to
`~/.gemini/config/`. Removal of the target ID is reversible through Git, but no
runtime compatibility path is retained.
