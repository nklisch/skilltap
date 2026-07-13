---
source_handle: pi-hooks-health-npm-registry
fetched: 2026-07-12
source_url: https://registry.npmjs.org/@hsingjui%2Fpi-hooks
provenance: source-direct
substrate_confidence: source-direct
---

# npm registry metadata for `@hsingjui/pi-hooks`

The npm registry is the canonical versioned release surface for this package.
The GitHub repository carries no release tags (see
`pi-hooks-health-github-source`), so npm `dist-tags.latest` and the per-version
publish times are the only authoritative version-currency evidence.

## Anchored excerpts

**Registry document, identity and currency (parsed 2026-07-12):**

```json
{
  "name": "@hsingjui/pi-hooks",
  "dist-tags": { "latest": "0.0.2" },
  "versions": ["0.0.1", "0.0.2"],
  "time": {
    "created": "2026-04-02T12:25:08.118Z",
    "modified": "2026-05-08T02:32:20.630Z",
    "0.0.1": "2026-04-02T12:25:08.358Z",
    "0.0.2": "2026-05-08T02:32:20.514Z"
  },
  "repository": { "url": "git+https://github.com/hsingjui/pi-hooks.git" }
}
```

**Latest version peer dependencies:**

```json
{ "peerDependencies": { "@earendil-works/pi-coding-agent": "*" } }
```

The peer dependency on `@earendil-works/pi-coding-agent` is unbounded (`*`),
i.e. the package declares compatibility with any host version and provides no
peer range to test against.

## Key passages and anchors

- **`dist-tags.latest`:** `0.0.2` — matches the installed version exactly.
- **Version history:** only two versions exist; `0.0.1` (2026-04-02) and
  `0.0.2` (2026-05-08). The package is young and infrequently released.
- **`modified` vs latest publish:** registry `modified` equals the `0.0.2`
  publish time, i.e. no registry-side metadata change has occurred since the
  last release.
- **Peer dependency:** `@earendil-works/pi-coding-agent: *` — no bounded host
  range; compatibility is undeclared at the package-manifest level.

## Structural metadata

- Publisher: npm registry (canonical package source)
- Document type: registry metadata JSON
- Surface: `@hsingjui/pi-hooks` release identity
- Retrieval depth: full registry document parsed
