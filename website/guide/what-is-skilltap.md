---
description: A personal control plane for local agent harness environments.
---

# What is skilltap?

skilltap gives agents an easy way to help you look after the tooling on your
computer. It adopts, reconciles, and updates native marketplaces, plugins,
complete skill directories, MCP declarations, and shared instruction files
across a typed registry of local agent harnesses.

The harnesses remain authoritative for their own formats. skilltap uses native
lifecycle commands when they exist, records desired state separately from
observed state, and reports when a resource cannot move faithfully between
harnesses.

That cross-harness handoff is a core part of skilltap: a marketplace or plugin
built for Claude can still provide compatible skills and components in Codex,
and vice versa. When a plugin is published natively for both harnesses,
skilltap tracks both native installations instead. Otherwise it materializes
through documented target paths and calls out anything that needs approval
instead of silently dropping behavior.

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

1. Enable the registered harness adapters you want skilltap to manage.
2. Adopt existing native configuration into skilltap's inventory.
3. Review a clear plan.
4. Synchronize safe, faithful changes.
5. Use status to inspect verified health, declared-but-unverified state, drift,
   updates, and required decisions.

Support is component- and scope-specific. Verified capabilities reconcile
normally. Declaration-managed file surfaces require explicit foreground
acknowledgment and remain effective-unverified. Observe-only targets never gain
mutation authority.

See the [Harness Support Matrix](../reference/harnesses) and start with
[Getting Started](./getting-started).
