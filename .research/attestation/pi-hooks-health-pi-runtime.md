---
source_handle: pi-hooks-health-pi-runtime
fetched: 2026-07-12
source_path: /home/nathan/.pi/agent/settings.json
provenance: source-direct
substrate_confidence: source-direct
---

# Pi runtime observation: package presence, version, and effective health

This attestation records observations from the running Pi binary and the live
global settings file (`~/.pi/agent/settings.json`). The binary is `pi`
version `0.80.6`; the `pi list` and `pi --help` outputs were captured at the
same time as the settings file read. The load-bearing observation: Pi's
package-list surface reports presence and a resolved path, but neither version,
nor health, nor per-resource enable state. Effective health is therefore a
separate fact from package presence.

## Anchored excerpts

**`pi list` output (verbatim, the two relevant entries):**

```text
npm:pi-mcp-adapter
    /home/nathan/.pi/agent/npm/node_modules/pi-mcp-adapter
npm:@hsingjui/pi-hooks
    /home/nathan/.pi/agent/npm/node_modules/@hsingjui/pi-hooks
```

`pi list` prints one source identifier and one resolved checkout path per
package. It does not print version, dependency/peer health, load status, or
per-resource enable/disable state.

**`pi --help` package-command surface (verbatim):**

```text
pi install <source> [-l]     Install extension source and add to settings
pi remove <source> [-l]      Remove extension source from settings
pi uninstall <source> [-l]   Alias for remove
pi update [source|self|pi]   Update pi (use --all for pi and extensions)
pi list                      List installed extensions from settings
pi config [-l]               Open TUI to enable/disable package resources (Tab switches scope)
```

The help text states explicitly that `pi list` reads "installed extensions
from settings," that install/remove write to settings (default user, `-l`
project), and that enable/disable of package resources is a TUI-only surface
(`pi config`).

**Live `~/.pi/agent/settings.json`, package list (verbatim, the two relevant
entries among fourteen):**

```json
{
  "packages": [
    "npm:pi-mcp-adapter",
    "npm:@hsingjui/pi-hooks"
  ]
}
```

**Live `~/.pi/agent/settings.json`, hooks key (programmatic check):**

```text
has hooks key: False
hooks value: (absent)
packages count: 14
pi-hooks present: True
mcp-adapter present: True
```

The settings file lists `npm:@hsingjui/pi-hooks` as an installed package but
contains NO `hooks` key. `loadSettings()` in the extension therefore returns
`{ settings: undefined }`, and every registered hook callback finds zero
groups. The extension's code is loaded into every session; its effect is null
until a `hooks` block is configured.

**`pi` binary version:** `pi --version` → `0.80.6`.

## Key passages and anchors

- **`pi list` reports presence + path only:** no version, no health, no
  enable state. The resolved path is the npm checkout root under
  `~/.pi/agent/npm/node_modules/<pkg>`.
- **`pi list` is settings-derived:** the help text confirms it lists
  "installed extensions from settings," not a registry/cache probe.
- **Install/remove write settings; default user scope, `-l` project:** the
  settings file is the mutation surface, not an opaque cache.
- **Enable/disable is TUI-only:** `pi config` is the documented resource
  enable/disable surface; no non-interactive flag is documented in `--help`.
- **Presence ≠ health:** `npm:@hsingjui/pi-hooks` is present in `packages`
  but `hooks` is absent, so the extension is loaded yet inert. Version +
  health must be observed from the resolved checkout path and the
  `hooks` key respectively.
- **Pi version:** 0.80.6; installed `pi-hooks` peer-depends on
  `@earendil-works/pi-coding-agent: *` (unbounded).

## Structural metadata

- Publisher: Pi / Earendil (binary); local machine state (settings)
- Document type: live runtime observation (binary output + config file)
- Surface: package presence, version, scope, health, enable
- Retrieval depth: full `pi list` output, `pi --help`, parsed settings file,
  `pi --version`
