---
description: Machine-wide policy, inventory, operational state, and managed artifacts.
---

# State and Configuration

All skilltap-owned machine configuration lives under:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/skilltap/
├── config.toml
├── inventory.toml
├── state.json
└── managed/
```

No skilltap metadata is written into managed projects.

## `config.toml`

Policy includes enabled harnesses, binary overrides, instruction bridge mode,
and update behavior. It does not list installed resources.

```toml
schema = 1

[harnesses.codex]
enabled = true
binary = "codex"

[harnesses.claude]
enabled = true
binary = "claude"

[instructions]
claude_mode = "symlink"

[updates]
mode = "apply-safe"
interval = "6h"
```

## `inventory.toml`

Inventory is the human-readable desired state for registered marketplaces,
plugins, standalone skills, instruction locations, target harnesses, project
paths, component selections, update policy, and pins. Entries use stable
skilltap identifiers and canonical absolute paths.

## `state.json`

State is machine-written provenance and observation: native identifiers,
fingerprints, versions, Git revisions, last checks, native harness versions,
and apply results. It is inspectable output, not desired configuration.

## `managed/`

Managed artifacts and recoverable backups live here when no faithful native
lifecycle exists. Authentication material and secrets never enter any skilltap
state file or managed artifact.
