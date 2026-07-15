---
source_handle: opencode-cli
fetched: 2026-07-14
source_url: https://dev.opencode.ai/docs/cli/
provenance: source-direct-plus-isolated-execution
substrate_confidence: source-direct-and-runtime
---

# OpenCode CLI

The current official CLI documents `opencode mcp list`/`ls`,
`opencode mcp auth`, `opencode mcp logout`, and `opencode mcp debug <name>`.
The documented plugin command remains the one-way `opencode plugin <module>`
installer with `--global` and `--force`; the command reference does not define a
complete native marketplace/plugin install-update-remove-list lifecycle.

The isolated 1.18.1 binary emitted exactly `1.18.1\n` for `opencode --version`.
The bounded status probe must use direct arguments `mcp`, `list`, with the
selected project as working directory for project scope. OpenCode's `debug`
command is a follow-up diagnostic only and is not a lifecycle operation.

## Key passages

- The command index describes `mcp list` as listing configured servers and
  connection status.
- The plugin command is described only as installing a plugin and updating
  configuration.
- Authentication and debug commands can initiate or inspect OAuth and are not
  invoked as part of ordinary skilltap reconciliation.
