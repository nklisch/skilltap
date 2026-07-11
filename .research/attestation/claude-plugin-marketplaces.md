---
source_handle: claude-plugin-marketplaces
fetched: 2026-07-10
source_url: https://code.claude.com/docs/en/plugin-marketplaces
provenance: source-direct
substrate_confidence: source-direct
---

# Claude Code plugin marketplaces

## Summary

Anthropic defines a marketplace as a catalog rooted at `.claude-plugin/marketplace.json`. The marketplace schema requires `name`, `owner`, and `plugins`; each plugin entry requires `name` and `source`. Supported sources include relative paths, GitHub repositories, other git repositories, subdirectories, npm packages, and inline settings declarations, with source-specific constraints.

Claude Code exposes non-interactive commands for adding, listing, updating, and removing marketplaces. Marketplace declarations support user, project, and local scope. Removing a marketplace also removes plugins installed from it. Version resolution and automatic updates are native lifecycle behavior rather than a cache-editing contract.

## Anchored excerpts

**Create the marketplace file, line 234:**

> Each plugin entry needs at minimum a `name` and a `source`.

**Manage marketplaces from the CLI, line 853:**

> Claude Code provides non-interactive `claude plugin marketplace` subcommands for scripting and automation.

## Key passages and anchors

- **Create the marketplace file, lines 232-259:** `.claude-plugin/marketplace.json` sits under the marketplace root and lists plugins; each entry minimally needs a name and source.
- **Marketplace schema, lines 263-320:** marketplace `name`, `owner`, and `plugins` are required; plugin entries require a kebab-case name and a source; optional metadata, strictness, and rename history are specified.
- **Relative paths, lines 349-360:** relative plugin sources resolve from the marketplace root, defined as the directory containing `.claude-plugin/`, and cannot traverse above it.
- **Version resolution, lines 727-735:** resolved version comes from `plugin.json`, marketplace entry, or git commit SHA; identical resolved versions cause update and auto-update to skip.
- **Validation and CLI management, lines 830-908:** `claude plugin validate` validates marketplace/plugin material; `claude plugin marketplace add` accepts GitHub shorthand, git URLs, remote JSON URLs, and local paths plus scope; `list --json` returns structured source data.
- **Removal/update behavior, marketplace management section:** removing a marketplace uninstalls plugins installed from it; updating refreshes catalog data; seed-managed marketplaces are read-only and cannot be updated or removed.
- **Pre-populated plugin seeds, lines 640-653:** seed directories mirror the native plugin cache layout, remain read-only, and take precedence over matching primary marketplace configuration.
- **Managed restrictions, lines 656 onward:** allow/block policies are checked before network or filesystem mutation and apply to add, install, update, refresh, and auto-update.

## Structural metadata

- Publisher: Anthropic
- Document type: normative product guide and schema reference
- Surface: Claude Code marketplace distribution and lifecycle
- Retrieval depth: full page with targeted line reads
