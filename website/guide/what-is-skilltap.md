---
description: A personal control plane for Codex and Claude Code environments.
---

# What is skilltap?

skilltap manages the agent tooling already chosen for one computer. It adopts,
reconciles, and updates native marketplaces, plugins, complete skill
directories, and shared instruction files across Codex and Claude Code.

The harnesses remain authoritative for their own formats. skilltap uses native
lifecycle commands when they exist, records desired state separately from
observed state, and reports when a resource cannot move faithfully between
harnesses.

## What it manages

- Marketplace registrations and native plugins.
- Standalone skills, where the complete directory containing top-level
  `SKILL.md` is the resource.
- Global and project instruction files, with `AGENTS.md` canonical.
- Source revisions and updates for resources managed by skilltap.
- Optional user-level background update checks.

## What it does not do

skilltap does not search marketplace catalogs, recommend extensions, provide a
public registry, run a terminal dashboard, or support older skilltap state.
Callers provide explicit marketplace, plugin, skill, and project identities.

## The operating loop

1. Enable the Codex and Claude adapters you use.
2. Adopt existing native configuration into skilltap's inventory.
3. Review a deterministic plan.
4. Synchronize safe, faithful changes.
5. Use status to inspect health, drift, updates, and required decisions.

Start with [Getting Started](./getting-started).
