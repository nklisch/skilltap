---
source_handle: pi-hooks-identity-installed
fetched: 2026-07-12
source_path: /home/nathan/.pi/agent/npm/node_modules/@hsingjui/pi-hooks
provenance: source-direct
substrate_confidence: source-direct
---

# Locally installed package — `@hsingjui/pi-hooks@0.0.2`

Source-direct attestation of the installed package on this machine, at
`/home/nathan/.pi/agent/npm/node_modules/@hsingjui/pi-hooks/`. This is the
concrete artifact Pi would load at runtime, read off the local filesystem.

## `package.json` (installed copy, `version: 0.0.2`)

All field values read off the local `package.json`:

- `name`: `@hsingjui/pi-hooks`
- `version`: `0.0.2`
- `description`: `Claude Code-compatible command hooks for the Pi coding agent`
- `type`: `module`
- `license`: `MIT`
- `keywords`: `pi-package`, `pi`, `pi-coding-agent`, `extension`, `hooks`,
  `command-hooks`, `claude-code`, `claude-code-hooks`
- `repository.url`: `git+https://github.com/hsingjui/pi-hooks.git`
- `bugs.url`: `https://github.com/hsingjui/pi-hooks/issues`
- `homepage`: `https://github.com/hsingjui/pi-hooks#readme`
- `publishConfig.access`: `public`
- `files`: `["src", "README.md"]` — the published tarball ships only the
  `src` tree and the English `README.md` (not the Chinese README, not the
  lockfile, not `tsconfig.json`).
- `exports`: `{"./package.json": "./package.json"}` — manifest-only export;
  no runtime subpath exports.
- `pi.extensions`: `["./src/pi-hooks.ts"]` — single Pi extension entry point.
- `peerDependencies`: `{"@earendil-works/pi-coding-agent": "*"}`
- `devDependencies`: `{"typescript": "^6.0.2"}`

## Observed file tree (installed)

- `README.md` (≈14.6K)
- `README.zh-CN.md` (≈14.1K) — note: the Chinese README is present in the
  installed copy despite the `files` allowlist naming only `README.md`; on
  case-sensitive filesystems this is a distinct file. (The installed copy
  reflects the published tarball contents; npm includes it because the
  tarball packing at publish time included it. The `files` allowlist in the
  committed manifest names only `README.md`.)
- `package.json` (925 B)
- `src/`: `config.ts`, `executor.ts`, `helpers.ts`, `hook-context.ts`,
  `pi-hooks.ts`, `types.ts`
- `src/hooks/`: `compact-hooks.ts`, `prompt-hooks.ts`, `session-hooks.ts`,
  `shared.ts`, `stop-hooks.ts`, `tool-hooks.ts`

## Entry-point behavior (`src/pi-hooks.ts`, read in full)

The default export is a function `export default function (pi: ExtensionAPI)`
that:

1. Imports the `ExtensionAPI` type from `@earendil-works/pi-coding-agent`.
2. Builds a shared hook context via `createHookContext(pi)` from
   `./hook-context`.
3. Calls five registration functions: `registerSessionHooks`,
   `registerCompactHooks`, `registerPromptHooks`, `registerStopHooks`,
   `registerToolHooks`.

This confirms the single declared `pi.extensions` entry point resolves and
that the extension's runtime composition is the union of those five
registrars.

## README claims worth attesting (load-bearing for identity)

These are claims the package's own README makes about itself; they are
attested here as *claims*, not as verified behavior:

- **Install instruction:** `pi install npm:@hsingjui/pi-hooks`.
- **Configuration surface:** `~/.pi/agent/settings.json` (global) or
  `.pi/settings.json` (project), under a top-level `hooks` key.
- **Supported events (claimed):** `SessionStart`, `SessionEnd`, `PreCompact`,
  `PostCompact`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`,
  `UserPromptSubmit`, `Stop`.
- **Explicit non-support (claimed):** hook handler types `http`, `prompt`,
  `agent` are not supported; only `type: "command"` is supported.
- **Event mapping (claimed):** `SessionStart`→`session_start`,
  `SessionStart.compact`→`session_compact`, `SessionEnd.other`→`session_shutdown`,
  `Stop`→`agent_end` (best-effort), `PreToolUse`→`tool_call`,
  `PostToolUse`→`tool_result`.

## Documentation vs. installed-tree discrepancy

The README's "Project Structure" section enumerates ten source files:
`src/pi-hooks.ts`, `src/config.ts`, `src/executor.ts`, `src/hooks/shared.ts`,
`src/hooks/session-hooks.ts`, `src/hooks/compact-hooks.ts`,
`src/hooks/prompt-hooks.ts`, `src/hooks/tool-hooks.ts`,
`src/hooks/stop-hooks.ts`, `src/types.ts`.

The installed tree contains two additional source files not listed in the
README's structure section: `src/helpers.ts` and `src/hook-context.ts`.
`src/hook-context.ts` is in fact imported by the entry point. The README's
structure section is therefore incomplete relative to the shipped source.
This is a documentation-completeness observation, not a behavioral claim.
