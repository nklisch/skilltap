---
source_handle: zcode-mcp
fetched: 2026-07-12
source_url: https://zcode.z.ai/en/docs/mcp-services
provenance: source-direct
---

# ZCode MCP

ZCode supports user and workspace MCP scope, stdio/HTTP/SSE transports, JSON
import, and per-server enablement. It can import Claude, Codex, OpenCode, and
generic `.agents` definitions into a `.zcode` configuration at the selected
scope. The English page names the storage family but not the exact native file
path, which leaves a direct-write adapter boundary to verify empirically.

## Key passages

- “Create An MCP Server” offers User and Workspace scope.
- The form and full configuration modes support stdio, HTTP, SSE, headers, and environment.
- “Import From An External Agent” documents source files and target scope.
- Imported servers remain editable, enableable, disableable, and removable.
