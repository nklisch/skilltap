---
source_handle: pi-hooks-health-github-source
fetched: 2026-07-12
source_url: https://github.com/hsingjui/pi-hooks
provenance: source-direct
substrate_confidence: source-direct
---

# GitHub source for `hsingjui/pi-hooks`

The GitHub repository is the documented source-of-record (`package.json`
`repository.url`) but is NOT the authoritative version surface: it has no
release tags, and its default-branch manifest reads a different version than
the published npm tarball. This divergence is load-bearing for any
identity/update check that reads the repository HEAD rather than npm.

## Anchored excerpts

**Repository metadata (GitHub API, parsed 2026-07-12):**

```json
{
  "default_branch": "main",
  "pushed_at": "2026-05-08T02:29:13Z",
  "archived": false,
  "disabled": false,
  "open_issues_count": 1,
  "license": null
}
```

**Default-branch `package.json` served by GitHub raw (parsed 2026-07-12):**

```json
{
  "name": "@hsingjui/pi-hooks",
  "version": "0.0.1",
  "pi": { "extensions": ["./src/pi-hooks.ts"] },
  "files": ["src", "README.md"],
  "peerDependencies": { "@earendil-works/pi-coding-agent": "*" }
}
```

The default-branch manifest reads `version: 0.0.1`, while the published npm
tarball and the locally installed copy both read `0.0.2`
(see `pi-hooks-health-npm-registry` and `pi-hooks-health-installed-package`).

**Latest commits on `main` (GitHub API):**

```text
8250a856d4f8 | 2026-05-08T02:27:43Z | migrate to @earendil-works pi package
e0ad1cd8db25 | 2026-04-03T06:15:40Z | docs: rewrite README in English and add README.zh-CN.md
18e06e62fb5f | 2026-04-02T12:29:26Z | docs: add Quick Setup and update npm package metadata
```

The most recent commit (`8250a856d4f8`, 2026-05-08) is dated ~4 minutes before
the `pushed_at` timestamp and ~5 minutes before the npm `0.0.2` publish time
(2026-05-08T02:32:20Z). The commit message ("migrate to @earendil-works pi
package") does not describe a version bump, and the manifest on `main` still
reads `0.0.1`.

**Tags (GitHub API):** the `/tags` endpoint returned an empty list. There are
no git release tags; npm is the sole versioned surface.

## Key passages and anchors

- **Default branch:** `main`; not archived, not disabled; 1 open issue.
- **License detection:** GitHub reports `license: null` — no LICENSE file is
  detected, even though the npm `package.json` declares `"license": "MIT"`.
  The `files` field (`["src", "README.md"]`) would exclude a LICENSE file from
  the npm tarball if one existed.
- **Version divergence:** `main` manifest = `0.0.1`; npm latest and installed =
  `0.0.2`. A HEAD-based identity check would falsely report drift against an
  installed `0.0.2`.
- **No tags:** `/tags` empty; only npm versions are citable release points.
- **Last commit timing:** `main` HEAD `8250a856d4f8` predates the npm `0.0.2`
  publish by ~5 minutes and does not itself bump the manifest version.

## Structural metadata

- Publisher: GitHub user `hsingjui`
- Document type: source repository (default branch + API metadata)
- Surface: `hsingjui/pi-hooks` source-of-record
- Retrieval depth: API metadata, default-branch `package.json`, recent commits,
  tags list
