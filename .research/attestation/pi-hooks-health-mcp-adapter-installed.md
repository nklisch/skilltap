---
source_handle: pi-hooks-health-mcp-adapter-installed
fetched: 2026-07-12
source_path: /home/nathan/.pi/agent/npm/node_modules/pi-mcp-adapter/
provenance: source-direct
substrate_confidence: source-direct
---

# Installed `pi-mcp-adapter` artifact (compound-profile interaction)

The MCP adapter is the second half of a compound Pi profile that pairs Claude
hook compatibility (`pi-hooks`) with MCP support (`pi-mcp-adapter`). This
attestation records the installed adapter's identity and, critically, the
configuration files it owns — which are disjoint from the `settings.json`
`hooks` key that `pi-hooks` reads. The two extensions coexist without
file-config overlap.

## Anchored excerpts

**`package.json` (installed), identity block:**

```json
{
  "name": "pi-mcp-adapter",
  "version": "2.11.0",
  "description": "MCP (Model Context Protocol) adapter extension for Pi coding agent",
  "license": "MIT",
  "author": "Nico Bailon",
  "bin": { "pi-mcp-adapter": "cli.js" },
  "repository": { "url": "git+https://github.com/nicobailon/pi-mcp-adapter.git" },
  "pi": { "extensions": ["./index.ts"] },
  "peerDependencies": { "zod": "^3.25.0 || ^4.0.0" },
  "devDependencies": { "@earendil-works/pi-coding-agent": "^0.79.1" }
}
```

**`config.ts`, configuration-file names referenced (grep of the installed
source):**

```text
mcp.json
.pi/mcp.json
```

The adapter reads MCP server configuration from `mcp.json` family files
(`~/.config/mcp/mcp.json` user-global, `~/.pi/agent/mcp.json` Pi override,
project `.mcp.json`, project `.pi/mcp.json`, per the package documentation
attested separately as `[pi-mcp-adapter]{55}`). It does NOT read or write the
`hooks` key of `~/.pi/agent/settings.json` or `.pi/settings.json`.

**Live state check (observational cache exists, Pi-global mcp config absent):**

```text
ls ~/.pi/agent/mcp-cache.json    ->  -rw-------  370552 bytes  (exists)
ls ~/.pi/agent/mcp.json          ->  (absent)
```

`mcp-cache.json` is the adapter's observational metadata cache (per
`[pi-mcp-adapter]{55}`, "observational rather than a configuration write
target"). `~/.pi/agent/mcp.json` does not exist, so the adapter currently
relies on its default precedence chain rather than a Pi-global override.

## Key passages and anchors

- **Identity:** `pi-mcp-adapter@2.11.0`; MIT; author Nico Bailon; repo
  `git+https://github.com/nicobailon/pi-mcp-adapter.git`; registers via
  `pi.extensions: ["./index.ts"]`; ships a `cli.js` binary.
- **Configuration files owned:** `mcp.json` and `.pi/mcp.json` (plus the
  user-global `~/.config/mcp/mcp.json` and project `.mcp.json` per the doc
  attestation). Disjoint from `pi-hooks`'s `settings.json` `hooks` key.
- **No `settings.json` writes:** the adapter's config layer touches the
  `mcp.json` family only; there is no path-overlap with `pi-hooks`.
- **Observational cache:** `~/.pi/agent/mcp-cache.json` (370 KB) is an
  observation surface, not a configuration write target.
- **Independence from `pi-hooks`:** separate npm package, separate version,
  separate repo, separate config files. `pi remove npm:@hsingjui/pi-hooks`
  removes only the hooks package; the adapter remains, and vice versa.

## Structural metadata

- Publisher: Nico Bailon (`nicobailon` on npm/GitHub)
- Document type: installed npm package artifact (TypeScript extension + CLI)
- Surface: MCP configuration file ownership and compound-profile coexistence
- Retrieval depth: installed `package.json`, `config.ts` filename scan, live
  cache/config file presence check
