# Vision

skilltap is a personal control plane for agent development environments.

It gives one person a consistent way to understand and manage the skills, plugins, marketplaces, MCP servers, and instruction files used by Codex and Claude Code. It preserves each harness's native behavior while making the overall environment observable, reproducible on the same computer, and easier for agents to operate safely.

## Problem

Agent harnesses provide increasingly capable extension systems, but each harness represents and manages those extensions differently.

A person using multiple harnesses must understand several marketplace formats, plugin layouts, configuration files, installation scopes, instruction conventions, and lifecycle commands. The same skill may work everywhere, while a plugin may contain hooks, agents, apps, or servers that exist in only one harness. Configuration drifts as tools install or update resources independently.

This leaves both people and agents with basic unanswered questions:

- Which harnesses are configured on this computer?
- Which marketplaces and plugins are installed?
- Which resources are native, adopted, or managed by another tool?
- Can a resource be transferred faithfully to another harness?
- Are `AGENTS.md` and `CLAUDE.md` carrying the same instructions?
- What changed outside the desired configuration?
- What actions are safe to apply automatically?
- What decision requires the user?

## Core Idea

skilltap maintains a normalized, machine-wide description of one person's agent environment.

It can adopt existing Codex and Claude Code configuration into that state, compare the state with the current native environments, and synchronize changes back through each harness's supported mechanisms.

The normalized state is a control plane, not a replacement plugin format. Native harness behavior remains authoritative at the integration boundary.

## Native First

skilltap uses a harness's native marketplace, plugin, and configuration mechanisms whenever they exist.

A resource crosses harness boundaries only when skilltap can represent it faithfully. When a native plugin is unavailable for a target harness, skilltap may materialize compatible components into that harness's supported locations.

Partial or lossy materialization is visible and blocked by default. The user can approve the proposed result as a whole or choose components individually. Unsupported behavior is never silently discarded.

## Agent Forward

Every command is deterministic and non-interactive.

Commands produce concise output that an agent can interpret and convey to the user. Inspection and planning operations provide structured JSON when useful. When an operation requires judgment, skilltap explains:

- What it observed.
- What it proposes.
- What cannot be preserved.
- What decision the user needs to make.
- How to proceed after that decision.

Agents do not need a separate operating mode. The same command contract serves people, agents, scripts, and automation.

## Instructions as Shared Infrastructure

`AGENTS.md` is the canonical instruction format managed by skilltap. Global instructions live at `~/AGENTS.md`.

Each harness consumes the canonical content directly or through a managed native-location bridge. Claude Code uses a managed `CLAUDE.md` symlink or import. Existing project and nested instruction files remain in their natural locations.

skilltap detects missing links, conflicting files, unexpected ownership, and instruction drift without silently overwriting user-authored content.

## Audience

skilltap serves individual developers who use more than one agent harness and want one trustworthy view of their local environment.

Its state describes one computer. Repositories and collaborators do not need to adopt skilltap. Project-specific resources can still be observed and managed locally without introducing a shared skilltap manifest into the repository.

## Principles

1. **Native before normalized.** Use supported harness behavior rather than bypassing it.
2. **Faithfulness before portability.** Do not claim two components are equivalent when their behavior differs materially.
3. **Plan before mutation.** Show what reconciliation intends to change and why.
4. **Explicit loss.** Partial results require visible user approval.
5. **Observable ownership.** Track whether a resource is native, adopted, or materialized by skilltap.
6. **Idempotent reconciliation.** Synchronizing an unchanged environment produces no changes.
7. **No hidden decisions.** Drift and conflicts are reported for resolution rather than silently merged.
8. **Agent-readable operation.** Output makes the next safe action clear.
9. **Deep support over broad claims.** A harness is supported only when skilltap can model and operate it faithfully.

## Success

skilltap succeeds when a person or agent can quickly determine the health of the computer's agent environment and safely bring managed harnesses into the intended state.

A healthy environment has:

- Known and reachable harnesses.
- Traceable marketplaces and plugins.
- Consistent shared instructions.
- No unexplained drift.
- No ambiguous ownership.
- No silently omitted plugin behavior.
- A zero-change reconciliation plan when native state matches desired state.

## Non-Goals

skilltap does not:

- Discover, rank, recommend, or index skills and plugins.
- Host a marketplace.
- Define a universal plugin format.
- Reduce harnesses to a lowest-common-denominator abstraction.
- Scan extension content for security issues.
- Replace native authentication or secret storage.
- Require project collaborators to install skilltap.
- Act as a general-purpose dotfiles manager.
- Run a background service by default or require one for normal operation. An
  optional user-level update daemon may be explicitly enabled; it runs without
  elevated privileges and never bypasses acknowledgment, drift, or conflict
  safeguards.
- Provide an interactive dashboard or setup wizard.
- Claim support for a harness through file copying alone.

Codex and Claude Code are the supported harnesses. Additional harnesses belong only when their native systems can participate in the same faithfulness and reconciliation model.
