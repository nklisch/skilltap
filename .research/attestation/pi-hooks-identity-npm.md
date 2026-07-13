---
source_handle: pi-hooks-identity-npm
fetched: 2026-07-12
source_url: https://registry.npmjs.org/@hsingjui/pi-hooks
provenance: source-direct
substrate_confidence: source-direct
---

# npm registry packument — `@hsingjui/pi-hooks`

The canonical npm registry packument (JSON) for the scoped package
`@hsingjui/pi-hooks`, fetched from the public npm registry root endpoint. This
is the authoritative install source cited by the package's own README
(`pi install npm:@hsingjui/pi-hooks`).

## Key passages and source-internal values

All values below are read off the fetched packument JSON.

**Identity and description:**

- `name`: `@hsingjui/pi-hooks`
- `description`: `Claude Code-compatible command hooks for the Pi coding agent`
- `license`: `MIT`
- `homepage`: `https://github.com/hsingjui/pi-hooks#readme`
- `repository.url`: `git+https://github.com/hsingjui/pi-hooks.git`
- `bugs.url`: `https://github.com/hsingjui/pi-hooks/issues`
- `keywords`: `pi-package`, `pi`, `pi-coding-agent`, `extension`, `hooks`, `command-hooks`, `claude-code`, `claude-code-hooks`
- `publishConfig.access`: `public`

**Dist-tags and version timeline:**

- `dist-tags.latest`: `0.0.2`
- `versions`: `0.0.1`, `0.0.2`
- `time.created`: `2026-04-02T12:25:08.118Z`
- `time.modified`: `2026-05-08T02:32:20.630Z`
- `time.0.0.1`: `2026-04-02T12:25:08.358Z`
- `time.0.0.2`: `2026-05-08T02:32:20.514Z`

**Maintainership:**

- `maintainers`: a single entry — `{name: "hsingjui", email: "hsingjui@outlook.com"}`. The same sole maintainer is the `_npmUser` of both published versions.

**Pi package manifest (`pi` field) — both versions identical:**

- `pi.extensions`: `["./src/pi-hooks.ts"]`

**Peer-dependency shift between versions (load-bearing):**

- `0.0.1` `peerDependencies`: `{"@mariozechner/pi-coding-agent": "*"}`
- `0.0.2` `peerDependencies`: `{"@earendil-works/pi-coding-agent": "*"}`
- `devDependencies` (both): `{"typescript": "^6.0.2"}`

**Per-version build provenance:**

- `0.0.1`: `gitHead` `b89c3b6ce8e03354d341e4e3652de20dc093e66e`; `_npmVersion` `11.6.2`; `_nodeVersion` `24.12.0`; `dist.fileCount` `14`; `dist.unpackedSize` `63261`.
- `0.0.2`: `gitHead` `8250a856d4f892f0a8a640ac2f1241d1a000701b`; `_npmVersion` `11.11.1`; `_nodeVersion` `24.12.0`; `dist.fileCount` `15`; `dist.unpackedSize` `78290`; `dist.shasum` `ac0cb7c0aad7d27f28168e274e97952bf9cbc581`.

**Exports field (both versions):** `{"./package.json": "./package.json"}` — i.e., the package exposes its manifest but declares no runtime subpath exports; runtime entry is Pi-discovered via the `pi.extensions` field, not via Node resolution.

## Supplementary signal (same registry family)

npm downloads API (`api.npmjs.org/downloads`), fetched 2026-07-12:

- last-month (2026-06-12 → 2026-07-11): `126` downloads
- last-week (2026-07-05 → 2026-07-11): `20` downloads

These are usage-signal observations only; they are not identity claims and do
not attest package contents.

## Notes on what the packument does NOT contain

- The packument does not include a `files` allowlist field for either version
  (npm strips `files` from the published version metadata). The published
  fileset is attested from the local installed copy, not from this packument.
- The packument does not declare a `type` field on `0.0.1`'s displayed metadata
  block in the raw fetch; `0.0.2` declares `"type": "module"`. Both versions'
  source declares `"type": "module"` in the installed manifest.
