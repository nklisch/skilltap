---
source_handle: zcode-mcp
fetched: 2026-07-15
source_url: https://zcode.z.ai/cn/docs/mcp-services
corroborating_url: https://zcode.z.ai/en/docs/mcp-services
provenance: source-direct
---

# ZCode MCP

ZCode supports user and workspace MCP scope, stdio/HTTP/SSE transports, JSON
import, direct configuration-file editing, and per-server enablement. The
current official Chinese page names the native user file as
`~/.zcode/cli/config.json` and the native workspace file as
`<project-root>/.zcode/config.json`; both use `mcp.servers`. It also documents
compatible `.agents` fallback files at `~/.agents/mcp.json` and
`<project-root>/.agents/mcp.json`, using `mcpServers`.

Within one scope, any MCP service in the native `.zcode` file causes the whole
`.agents/mcp.json` file to be skipped rather than merged. Workspace and user
scopes are both loaded when a workspace is open, with workspace read first.
`"enable": false` disables a server; an absent field means enabled. Settings
writes always target the native `.zcode` file and do not change the `.agents`
source. The English localization still names only the `.zcode` configuration
family and does not include these exact path and precedence details.

## Key passages

- “配置文件与默认读取路径” names both native files, both `.agents` fallback
  files, and their schema keys, and explicitly permits direct manual editing.
- “读取规则与优先级” documents workspace/user loading and native `.zcode`
  precedence over `.agents` within one scope without merging.
- Enablement is represented by `"enable": false`; omission means enabled.
- Settings-created or edited servers are written to the selected scope's
  native `.zcode` file, leaving `.agents` files unchanged.
