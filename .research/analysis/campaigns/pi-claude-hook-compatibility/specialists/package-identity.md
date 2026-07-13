---
provenance: agent-synthesis
updated: 2026-07-12
facet: package-identity
temporal_contract: supersedes-prior
campaign: pi-claude-hook-compatibility
specialist: 1 of N
---

# Package identity — the active Pi package implementing Claude Code hook compatibility

This specialist resolves facet 1 of the `pi-claude-hook-compatibility`
campaign: the exact active Pi package implementing Claude Code hook
compatibility, attested by source/repository/npm identity, current version,
Pi package manifest, install scopes, resource entrypoints, and
official/source-direct claims. It actively seeks alternatives and
disconfirming evidence.

The load-bearing finding: **`@hsingjui/pi-hooks@0.0.2` is the unique active
package that carries both `pi-package` and `claude-code-hooks` identity
markers and whose declared surface is "Claude Code-compatible command hooks
for the Pi coding agent".** No competing package shares both identity
markers; the nearest neighbours either read a different configuration file,
bridge in the opposite direction, or are not hook packages at all. The
package is community-published, sole-maintainer, low-traffic, and has no
Pi-core endorsement — but its identity claim is unambiguous and uncontested.

## Source map

Per-facet numbered bibliography. `{N}` in citations resolves through this
table to a source-direct attestation under `.research/attestation/`.

| Number | Handle | Source |
|---:|---|---|
| 1 | `pi-hooks-identity-npm` | npm packument — `https://registry.npmjs.org/@hsingjui/pi-hooks` |
| 2 | `pi-hooks-identity-github` | GitHub repo — `https://github.com/hsingjui/pi-hooks` |
| 3 | `pi-hooks-identity-installed` | local installed package — `~/.pi/agent/npm/node_modules/@hsingjui/pi-hooks` |
| 4 | `pi-hooks-identity-alternatives` | npm registry — multi-package lookups of alternative claimants |
| 5 | `pi-hooks-identity-pi-docs` | Pi official docs — `https://pi.dev/docs/latest/packages` (and siblings) |

All five handles are source-direct attestations authored before this
synthesis. No analytical-tier artifact is cited as a source.

## npm identity

- **Scoped name:** `@hsingjui/pi-hooks`. [pi-hooks-identity-npm]{1}
- **Declared description (npm and GitHub identical):** "Claude Code-compatible command hooks for the Pi coding agent". [pi-hooks-identity-npm]{1} [pi-hooks-identity-github]{2}
- **License declared in package metadata:** `MIT`. The GitHub repository metadata, by contrast, reports **no SPDX license** (`license: None` in the repo API response); the MIT declaration lives in the npm packument and the installed manifest, not in the GitHub repo metadata. [pi-hooks-identity-npm]{1} [pi-hooks-identity-github]{2} [pi-hooks-identity-installed]{3}
- **Sole maintainer/publisher:** `hsingjui` (`hsingjui@outlook.com`), the `_npmUser` of both published versions and the only entry in `maintainers`. [pi-hooks-identity-npm]{1}
- **Repository binding:** `git+https://github.com/hsingjui/pi-hooks.git`, owned by GitHub user `hsingjui` (type `User`, not an organization). Not a fork. [pi-hooks-identity-npm]{1} [pi-hooks-identity-github]{2}

## Current version and version state

- **Latest published version:** `0.0.2` (`dist-tags.latest`). The package has exactly two versions: `0.0.1` (published 2026-04-02) and `0.0.2` (published 2026-05-08). [pi-hooks-identity-npm]{1}
- **The locally installed copy is `0.0.2`**, matching latest. [pi-hooks-identity-installed]{3}
- **No git tags, no GitHub releases.** The npm registry is the only versioned release channel. [pi-hooks-identity-github]{2}

### Provenance tension (load-bearing for reconciliation)

The npm `0.0.2` version records `gitHead: 8250a856d4f892f0a8a640ac2f1241d1a000701b`, which is the current HEAD of `main` (commit message "migrate to @earendil-works pi package", 2026-05-08). But the raw `package.json` at that exact commit still declares `"version": "0.0.1"`, even though its `peerDependencies` was already migrated to `@earendil-works/pi-coding-agent`. [pi-hooks-identity-github]{2}

The consistent reading: the maintainer published `0.0.2` by bumping the version at publish time without committing the version-field bump back to `main`. Combined with the absence of tags and releases, **the in-repo `package.json` `version` field is not authoritative for what npm publishes**, and `main`'s state under-reports the published version. Any adapter that reconciles repo state to npm state must treat the npm packument's `version` as the source of truth and must not trust `main`'s `version` field. This also means git-SHA-based update detection against `main` HEAD will see no movement between the 0.0.2 publish and any future commit until a real new commit lands. [pi-hooks-identity-github]{2}

## Pi package manifest

- **Pi extension entry point:** a single extension at `./src/pi-hooks.ts`, declared via the `pi.extensions` field, identical in both published versions. [pi-hooks-identity-npm]{1} [pi-hooks-identity-installed]{3}
- **Module type:** `module` (ESM). [pi-hooks-identity-installed]{3}
- **Peer dependency (current, `0.0.2`):** `@earendil-works/pi-coding-agent: "*"`. The `0.0.1` release declared a different peer: `@mariozechner/pi-coding-agent: "*"`. The 0.0.1→0.0.2 jump is exactly the Pi package rename/migration — Pi's npm scope moved from `@mariozechner/pi-coding-agent` to `@earendil-works/pi-coding-agent`. [pi-hooks-identity-npm]{1}
- **Node exports:** `{"./package.json": "./package.json"}` only — manifest-only; runtime entry is Pi-discovered through `pi.extensions`, not Node subpath resolution. [pi-hooks-identity-npm]{1} [pi-hooks-identity-installed]{3}
- **Published file set:** `["src", "README.md"]` per the manifest `files` allowlist; the published tarball is 15 files, 78290 bytes unpacked. [pi-hooks-identity-npm]{1} [pi-hooks-identity-installed]{3}

### Resource entrypoints (runtime composition)

The entry-point module imports the `ExtensionAPI` type from `@earendil-works/pi-coding-agent` and registers five hook families through a shared context: session, compact, prompt, stop, and tool. The shipped source is `src/pi-hooks.ts` plus `src/{config,executor,helpers,hook-context,types}.ts` and `src/hooks/{compact,prompt,session,shared,stop,tool}-hooks.ts`. [pi-hooks-identity-installed]{3}

## Install scopes and configuration surface (source-direct claims)

These are the package's own README claims about how it integrates; they are attested as claims, with the facet-2/3 specialists owning semantic-equivalence verification:

- **Install command:** `pi install npm:@hsingjui/pi-hooks`. [pi-hooks-identity-installed]{3}
- **Configuration surface:** a top-level `hooks` key in `~/.pi/agent/settings.json` (global) or `.pi/settings.json` (project). This adopts Claude Code's hook configuration *shape* under a Pi settings file, rather than reading Claude Code's own `.claude/settings.json`. [pi-hooks-identity-installed]{3}

The official Pi packages documentation confirms `npm` as a first-class documented package source and that global installs write `~/.pi/agent/settings.json` while project installs use `.pi/settings.json` — so the package's claimed install path and scope targets are Pi-supported. [pi-hooks-identity-pi-docs]{5} [pi-hooks-identity-installed]{3}

## Source-direct and official claims

- **Pi core documents no native hook system.** `https://pi.dev/docs/latest/hooks` returns the site's generic "Page Not Found". Compatibility is therefore always with *Claude Code's* hook format bridged onto Pi extension events; there is no Pi-core hook contract for the package to be native to. [pi-hooks-identity-pi-docs]{5}
- **No Pi-core endorsement of any hook-compatibility package.** The official packages page mentions no `hooks`, `Claude`, `hsingjui`, or recommended community package. The community gallery at `pi.dev/packages` does not surface `@hsingjui/pi-hooks`, `pi-hooks`, `claude-code-hooks`, or `@fyeeme/pi-hooks` on its first page. Selection is a consumer decision; Pi core vouchsafes no specific bridge. [pi-hooks-identity-pi-docs]{5}
- **The package is low-traffic and early.** npm downloads API reports 126 downloads in the trailing month (2026-06-12 → 2026-07-11) and 20 in the trailing week. The GitHub repo has 11 stars and 1 open issue as of fetch. These are usage/maturity signals, not identity claims. [pi-hooks-identity-npm]{1} [pi-hooks-identity-github]{2}

## Disconfirming analysis

Each row tests a load-bearing proposition against the attested alternatives.

| Load-bearing proposition tested | Disconfirming search | Outcome |
|---|---|---|
| `@hsingjui/pi-hooks` is the unique package carrying both `pi-package` and `claude-code-hooks` identity keywords | npm search `keywords:claude-code-hooks` AND `keywords:pi-package` returns exactly one result. | Confirmed unique on identity keywords; no other package carries both. [pi-hooks-identity-alternatives]{4} |
| Some other package is the "real" or "official" Claude hook-compat package | Pi core has no hooks doc page and endorses no community hook package; the packages doc and gallery name none. | Confirmed: there is no Pi-core canonical alternative. [pi-hooks-identity-pi-docs]{5} |
| `@fyeeme/pi-hooks` is a drop-in competitor | It reads `.pi/hooks.json` (a Pi-local file), not Claude Code's `.claude/settings.json`, and maps only `SessionStart`, `PreToolUse`, `Stop`. | Rejected as same-identity: its "Claude Code-compatible" denotes event-name familiarity, not config-file compatibility. Different surface. [pi-hooks-identity-alternatives]{4} |
| `@ryan_nookpi/pi-extension-claude-hooks-bridge` is an alias | It bridges Claude's `.claude/settings.json` *into* Pi as source-of-truth — opposite direction from `@hsingjui/pi-hooks`, which exposes a Pi `hooks` key adopting Claude's shape. | Rejected as alias: complementary in spirit, distinct in surface and ownership model. [pi-hooks-identity-alternatives]{4} |
| `@vanillagreen/pi-claude-bridge` is a hook-compat package | Its description is a *provider* bridge running Claude Code via the Claude Agent SDK; peer-deps include `@earendil-works/pi-ai`. | Rejected: incommensurable surface (model routing, not hooks). Frequent false-positive on `claude`+`pi` searches. [pi-hooks-identity-alternatives]{4} |
| The unscoped `pi-hooks` is the same package | It is an unrelated bundle (checkpoint, lsp, permission, ralph-loop, repeat) by maintainer `prateekmedia`. | Rejected: lexical collision only; fully scoped `@hsingjui/pi-hooks` is required to disambiguate. [pi-hooks-identity-alternatives]{4} |
| The GitHub repo `package.json` version field reliably reports the published version | At the commit npm cites as `0.0.2`'s `gitHead`, the repo `package.json` still reads `version: 0.0.1`. | Rejected: repo `version` is not authoritative; npm packument `version` is the source of truth. [pi-hooks-identity-github]{2} |
| The repo's license is settled | GitHub repo metadata reports `license: None`; the MIT claim lives only in npm/manifest metadata. | Qualifies: MIT is declared by the publisher, but the repo itself sets no GitHub-recognized license file/metadata. [pi-hooks-identity-github]{2} [pi-hooks-identity-installed]{3} |

## Contradictions

Two structural tensions survive the pass; neither is smoothed over.

**Tension A — repo HEAD vs. published version (relationship: `contradicts`).**
The npm `0.0.2` publish cites commit `8250a856…` as its `gitHead`, yet the
`package.json` at that commit reads `version: 0.0.1`. The two records cannot
both be authoritative about "what version is at this commit". Resolution
demands a choice of authority: the npm packument is the publish-of-record and
its `version` (`0.0.2`) governs; the repo's stale field is a maintainer
hygiene gap. Handles: `pi-hooks-identity-npm`, `pi-hooks-identity-github`.
[pi-hooks-identity-npm]{1} [pi-hooks-identity-github]{2}

**Tension B — declared license vs. repo license metadata (relationship:
`qualifies`).** The npm packument and installed manifest declare `MIT`; the
GitHub repository API reports `license: None` (no SPDX license recognized by
GitHub, which typically means no committed `LICENSE` file GitHub can parse,
or no license metadata in the repo). The MIT declaration is the publisher's
assertion; the repo has not made that declaration machine-resolvable. This
does not contradict the npm claim, but it qualifies how strongly a downstream
consumer can rely on the license without inspecting the repo's actual
`LICENSE` content (which this facet did not fetch). Handles:
`pi-hooks-identity-npm`, `pi-hooks-identity-github`, `pi-hooks-identity-installed`.
[pi-hooks-identity-npm]{1} [pi-hooks-identity-github]{2} [pi-hooks-identity-installed]{3}

## Acquisition candidates

The identity facet is well-resolved with fetched sources; the candidates below
are proactive enrichment, not blocking gaps. Each is grounded in a fetched
source that names or implies it.

- **Repo `LICENSE` file content.** The GitHub repo metadata reports no SPDX
  license, while npm/manifest declare MIT. Fetching the repo's actual
  `LICENSE` file (or confirming its absence) would resolve Tension B. Grounded
  in: the repo API response reports `license: None`.
  [pi-hooks-identity-github]{2}
- **Closed issues / full issue history.** Only open issues were enumerated
  (one open, `#2`). The closed-issue history would reveal whether
  semantic-equivalence or compatibility defects have been reported and
  resolved — directly relevant to the adjacent hook-semantic-equivalence
  facet. Grounded in: the repo reports `open_issues_count: 1` and an open
  issue numbered `#2`, implying a prior `#1`.
  [pi-hooks-identity-github]{2}
- **`pi.dev/packages` gallery pagination / search API.** The gallery's first
  page did not surface `@hsingjui/pi-hooks`, but the gallery is paginated and
  download-ordered; a search/pagination API would confirm whether the package
  is listed at all and capture its gallery-side download/maintainer metadata
  for triangulation against the npm downloads API. Grounded in: the gallery
  exists and renders ~101 entries with per-package download counts.
  [pi-hooks-identity-pi-docs]{5}
- **`@ryan_nookpi/pi-extension-claude-hooks-bridge` and `@fyeeme/pi-hooks`
  README/source.** This facet established their *identity divergence* from
  registry metadata alone. Reading each package's README would let the
  adjacent semantic-equivalence facet authoritatively bound what each claims
  to map, rather than relying on the registry `description` field. Grounded
  in: both packages' packuments name repos
  (`Jonghakseo/pi-extension`, `fyeeme/pi-packages`).
  [pi-hooks-identity-alternatives]{4}

## Unknowns

- {ambiguous: license-file-presence} The MIT license is declared in npm and
  manifest metadata but not in GitHub repo metadata. The repo's actual
  `LICENSE` file was not fetched; absence vs. unparsed-presence is unresolved.
- {ambiguous: closed-issues} Issue `#2` is open; the existence and content of
  closed issue/PR `#1` and any others was not enumerated. Defect history
  could bear on compatibility classification in the adjacent facets.
- {ambiguous: gallery-listing} Whether `@hsingjui/pi-hooks` appears anywhere
  in the `pi.dev/packages` gallery (beyond the first page) is unresolved.

## Revisit if

- A new npm package begins carrying both `pi-package` and `claude-code-hooks`
  keywords, or `@hsingjui/pi-hooks` is renamed, deprecated, or ownership
  transfers.
- The maintainer publishes a version bump that *does* commit the
  `version` field back to `main`, or begins tagging releases — either
  changes the reconciliation rule that npm is the sole authoritative channel.
- Pi core introduces a native hooks documentation page or officially
  endorses a hook-compatibility package, which would change the
  "no Pi-core endorsement" finding.
- The peer-dependency scope changes again (e.g., away from
  `@earendil-works/pi-coding-agent`), which would signal another Pi rename
  and a required republish to remain installable.
- The GitHub repo gains a recognized `LICENSE` file or formally declares
  non-MIT licensing, resolving or reversing Tension B.
- The package crosses a maturity threshold (maintainership, download volume,
  stars) that changes the weight a consumer should place on its sole-
  maintainer, low-traffic profile when classifying companion health.

## Acquisition candidates (summary)

See the "Acquisition candidates" section above; none are blocking for the
identity verdict. The facet's load-bearing claims — that
`@hsingjui/pi-hooks@0.0.2` is the unique active package carrying both
identity keywords, that its install path and scope targets are Pi-supported,
that Pi core endorses no competitor, and that the npm packument is the
authoritative version source — are all source-attested.
