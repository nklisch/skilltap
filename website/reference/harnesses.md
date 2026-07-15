---
description: Registered harnesses, support tiers, scopes, and verification boundaries.
---

# Harness Support

skilltap registers targets independently from the authority to mutate them.
Support is evaluated for the installed version, component, and concrete global
or project scope. Use `skilltap harness list`, `skilltap status`, and
`skilltap plan` for the machine-specific result.

## Support tiers

- **Verified** — skilltap can safely perform the operation and verify the native
  or managed result.
- **Declaration-managed** — an exact-version documented file surface can be
  owned, preserved, rolled back, removed, and repeated safely, but no safe native
  observer proves that the harness loaded it. Foreground `--yes` is required and
  status remains effective-unverified.
- **Observe-only** — skilltap can inspect safe documented surfaces but exposes no
  mutation authority.
- **Unsupported** — the component or scope has no safe contract. It remains
  blocked without preventing unrelated supported work.

Unknown versions never mutate. Native commands always require verified support.
The daemon never acknowledges declaration-managed operations.

## Current target matrix

| Target id | Harness | Current contract |
| --- | --- | --- |
| `codex` | Codex | Verified native and managed behavior for exact compiled profiles. |
| `claude` | Claude Code | Verified native marketplace/plugin behavior and managed compatibility. |
| `droid` | Factory Droid | Verified native/managed coexistence; project marketplace operations remain unsupported where the native lifecycle lacks them. |
| `qwen` | Qwen Code | Verified native extension conversion and managed skill/MCP projection. |
| `gemini` | Gemini CLI | Verified file-managed skills and MCP for the attested profile. |
| `opencode` | OpenCode | Verified file-managed skills and MCP for the attested profile. |
| `copilot` | GitHub Copilot CLI | Mixed: managed MCP has structured observation; complete skills are declaration-managed; incomplete native plugin lifecycle is unsupported. |
| `kiro` | Kiro CLI | Complete skills and MCP declarations for the attested profile; plugin/MCP projection is declaration-managed and never invokes login. |
| `kimi` | Kimi Code CLI | Global MCP declaration-managed; project MCP and unsupported auth/transports remain blocked. |
| `vibe` | Mistral Vibe | Global/trusted-project static MCP declaration-managed through syntax-preserving TOML edits; OAuth and interactive effective observation are unsupported. |
| `kilo` | Kilo Code | Valid global/project JSON/JSONC declarations are declaration-managed; side-effectful debug/list/auth probes are never invoked. |
| `junie` | Junie | Complete skills and MCP are declaration-managed; ambiguous cross-scope MCP names block and interactive `/mcp` is never invoked. |
| `amp` | Amp | Complete skills and MCP are declaration-managed; doctor, OAuth, login, browser, and trust approval are never invoked. |
| `pi` | Pi | Observe-only compound profile for the attested Pi, MCP companion, and hook companion tuple. |
| `cursor` | Cursor | Observe-only using documented `agent` detection and safe skill/MCP read surfaces; no mutation profile. |
| `zoo` | Zoo Code | File-only observe-only target; editor storage, installed identity, and effective reload remain unresolved. |
| `zcode` | ZCode | File-only observe-only target for documented global skill and MCP files; project skills and effective reload remain unresolved. |

Codex and Claude Code remain the only first-party bootstrap plugin targets.
Other registered harnesses participate through ordinary enablement, status,
planning, synchronization, and update behavior according to the matrix above.

## Declaration-managed safety

An acknowledged declaration-managed operation does **not** mean the harness
loaded or activated the resource. skilltap still guarantees:

- exact-version authority;
- explicit files and consequences in the plan;
- no overwrite of unmanaged or ambiguous same-name entries;
- preservation of unrelated documented fields and credential references;
- root-confined no-follow writes;
- lock-time executable and file revalidation;
- rollback and target-local ownership;
- immediate-repeat no change;
- no daemon execution, browser, authentication, trust approval, TUI, editor
  automation, or side-effectful status probe.
