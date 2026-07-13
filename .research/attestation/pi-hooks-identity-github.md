---
source_handle: pi-hooks-identity-github
fetched: 2026-07-12
source_url: https://github.com/hsingjui/pi-hooks
provenance: source-direct
substrate_confidence: source-direct
---

# GitHub repository — `hsingjui/pi-hooks`

Source-direct attestation of the GitHub repository backing the npm package, via
the GitHub REST API (`api.github.com/repos/hsingjui/pi-hooks`) plus the
contents, commits, tags, and releases endpoints. All values read off fetched
JSON.

## Repository metadata

- `full_name`: `hsingjui/pi-hooks`
- `html_url`: `https://github.com/hsingjui/pi-hooks`
- `clone_url`: `https://github.com/hsingjui/pi-hooks.git`
- `description`: `Claude Code-compatible command hooks for the Pi coding agent`
  (identical wording to the npm `description`).
- `homepage`: empty string (no separate site).
- `default_branch`: `main`
- `owner.login`: `hsingjui`; `owner.type`: `User`
- `license`: **`None`** — the GitHub repo metadata reports no SPDX license.
  (The npm package.json declares `MIT`; that declaration is attested from the
  npm packument and the installed manifest, not from this repo metadata.)
- `fork`: `False`; `parent`: none.
- `archived`: `False`; `disabled`: `False`.
- `stargazers_count`: `11`
- `open_issues_count`: `1`
- `topics`: `claude-code`, `coding-agent`, `extension`, `pi`
- `created_at`: `2026-04-02T12:04:47Z`
- `updated_at`: `2026-07-07T18:45:37Z`
- `pushed_at`: `2026-05-08T02:29:13Z`

## Tags and releases

- **Git tags:** none. The tags endpoint returns an empty list.
- **GitHub releases:** none. The releases endpoint returns an empty list.

The npm registry is therefore the only versioned release channel; there is no
git tag or GitHub release corresponding to `0.0.1` or `0.0.2`.

## Commit history (chronological, `main`)

| sha (short) | date (UTC) | message (first line) |
|---|---|---|
| `b89c3b6ce8` | 2026-04-02T12:17:14Z | `feat: add Claude Code-compatible hooks support for Pi` |
| `18e06e62fb` | 2026-04-02T12:29:26Z | `docs: add Quick Setup and update npm package metadata` |
| `e0ad1cd8db` | 2026-04-03T06:15:40Z | `docs: rewrite README in English and add README.zh-CN.md` |
| `8250a856d4` | 2026-05-08T02:27:43Z | `migrate to @earendil-works pi package` |

HEAD of `main` is `8250a856d4f892f0a8a640ac2f1241d1a000701b` (2026-05-08T02:27:43Z).

## Provenance tension between repo HEAD and npm `0.0.2`

This is a load-bearing observation. The npm `0.0.2` version records
`gitHead: 8250a856d4f892f0a8a640ac2f1241d1a000701b` — i.e., the published
`0.0.2` tarball cites the current `main` HEAD as its source commit. However,
the raw `package.json` at that same commit (fetched from
`raw.githubusercontent.com/hsingjui/pi-hooks/main/package.json`) still declares
`"version": "0.0.1"`, while its `peerDependencies` was already migrated to
`@earendil-works/pi-coding-agent`.

So at the commit npm cites as `0.0.2`'s source:

- the `version` field reads `0.0.1` (stale relative to the published `0.0.2`);
- the `peerDependencies` field reads `@earendil-works/pi-coding-agent: "*"`
  (matching the published `0.0.2`, not the published `0.0.1`).

The consistent reading: the maintainer published `0.0.2` by bumping the version
at publish time without committing the version-field bump back to `main`
(`npm version`/`npm publish` can rewrite the version in the packed tarball).
Combined with the absence of tags and releases, **the in-repo `package.json`
`version` field is not authoritative for what npm publishes**, and `main`'s
state under-reports the published version. Anyone reconciling repo state to npm
state must treat the npm packument's `version` as the source of truth and must
not trust `main`'s version field.

## File tree at HEAD (`main`)

Repository root:

- `.gitignore` (69 B)
- `README.md` (14954 B)
- `README.zh-CN.md` (14407 B)
- `package.json` (925 B)
- `pnpm-lock.yaml` (82646 B)
- `tsconfig.json` (245 B)
- `src/` (directory)

`src/`:

- `config.ts` (4013 B)
- `executor.ts` (3669 B)
- `helpers.ts` (1859 B)
- `hook-context.ts` (3152 B)
- `pi-hooks.ts` (859 B)
- `types.ts` (3309 B)
- `hooks/` (directory)

`src/hooks/`:

- `compact-hooks.ts` (1736 B)
- `prompt-hooks.ts` (3885 B)
- `session-hooks.ts` (1822 B)
- `shared.ts` (8056 B)
- `stop-hooks.ts` (3980 B)
- `tool-hooks.ts` (11664 B)

The repo uses `pnpm` (presence of `pnpm-lock.yaml`); the published package
ships only `src` and `README.md` per the manifest `files` allowlist (attested
from the installed copy).

## Open issue

- Issue #2: `fix: gracefully handle stale ctx after session replacement`,
  state `open`, `0` comments. (Issue numbering implies a prior #1; only open
  issues were enumerated. The issue title concerns internal context handling,
  not package identity.)
