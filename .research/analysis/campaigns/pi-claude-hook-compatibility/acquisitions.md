---
provenance: agent-synthesis
updated: 2026-07-12
campaign: pi-claude-hook-compatibility
temporal_contract: extend-on-source-rev
---

# Acquisition candidates

No blocking acquisition remains. The following source-bound candidates would
enrich later refreshes but do not hold the current decision.

| Source | Urgency | Class | Web availability | Completes |
|---|---|---|---|---|
| `hsingjui/pi-hooks` repository LICENSE or confirmed absence | enriching | primary-doc | public repository; exact file not fetched | resolves manifest-MIT versus repository-no-license qualification |
| Closed issue/PR history for `hsingjui/pi-hooks` | enriching | portal | public GitHub history | defect and semantic-compatibility history |
| Pi built-in tool input schemas and units | enriching | primary-doc | named by installed Pi SDK documentation | exact Claude-to-Pi tool-input translation matrix |
| Full settings schema written by `pi config` | enriching | primary-doc | Pi settings documentation / installed source | non-interactive observation of per-resource enable state |
| npm registry document for `pi-mcp-adapter` | enriching | ingestible | public npm registry | independent adapter-half release currency |
| Source and README for `@fyeeme/pi-hooks` and `@ryan_nookpi/pi-extension-claude-hooks-bridge` | enriching | portal | public npm/Git repositories | alternative-package semantic comparison |

Autonomous policy: these candidates remain research-side. Promotion to the
standing `.work/` acquisition queue requires an operator-confirmed research
handoff and was not performed by autopilot.
