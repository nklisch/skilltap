---
source_handle: kilo-marketplace
fetched: 2026-07-12
source_url: https://kilo.ai/docs/customize/marketplace
provenance: source-direct
substrate_confidence: source-direct
---

# Kilo Code marketplace

Kilo's marketplace page describes browsing, installing, and removing skills, agents, and MCP entries in the Kilo sidebar. Installation writes files to Kilo-specific global/project paths. The docs do not establish a deterministic shell lifecycle or a cache/source state contract for skilltap to reconcile.

## Key passages

- Marketplace items are described as configuration/instruction files, not editor extensions.
- The destination table distinguishes `.kilo/skills` and `~/.kilo/skills`.
- The lifecycle instructions are UI actions.
