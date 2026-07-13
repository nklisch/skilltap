---
source_handle: pi-hooks-identity-alternatives
fetched: 2026-07-12
source_url: https://registry.npmjs.org (multi-package registry lookups)
provenance: source-direct
substrate_confidence: source-direct
---

# Alternative claimants — npm packages touching "Pi" + "Claude Code hooks"

Source-direct attestation of nearby npm packages surfaced by registry search
(`registry.npmjs.org/-/v1/search`) and per-package registry lookups. Each
package's identity, declared surface, and divergence from
`@hsingjui/pi-hooks` is read off its packument. This attestation exists to
support disconfirmation: the question is whether any *other* active package
better fits the identity "Claude Code-compatible command hooks for Pi".

## Dispositive keyword intersection

The npm search query `keywords:claude-code-hooks` AND `keywords:pi-package`
returns **exactly one** result: `@hsingjui/pi-hooks@0.0.2`.

Read off the search response: the result set has one object whose
`package.name` is `@hsingjui/pi-hooks`, version `0.0.2`. No other package
carries both identity keywords. This is the strongest single piece of
disconfirming evidence that there is no direct competitor sharing both
declared identity markers.

## Package: `@fyeeme/pi-hooks`

- `dist-tags.latest`: `1.0.0`; only one published version.
- `description`: `Claude Code-compatible hooks runner for pi. Reads .pi/hooks.json and maps SessionStart, PreToolUse, and Stop events to pi lifecycle events.`
- `repository.url`: `git+https://github.com/fyeeme/pi-packages.git` (a
  monorepo, not a single-package repo).
- `homepage`: `https://github.com/fyeeme/pi-packages#readme`
- `license`: `MIT`; sole maintainer `fyeeme`.
- `time.created`: `2026-06-15T17:50:35.110Z`;
  `time.modified`: `2026-06-15T17:50:35.610Z` (single-day publish, no
  subsequent versions).
- `pi.extensions`: `["./index.ts"]`
- `peerDependencies` (1.0.0): `{"@earendil-works/pi-coding-agent": ">=0.77.0"}`
- `files`: not declared in the published version metadata.

**Divergence from `@hsingjui/pi-hooks`:** `@fyeeme/pi-hooks` reads
`.pi/hooks.json` — a Pi-local config file, **not** Claude Code's
`.claude/settings.json` hook format. Its claimed event coverage is narrower
(`SessionStart`, `PreToolUse`, `Stop` only). It does not present itself as
reusing existing Claude Code hook configurations unchanged; it requires its
own `.pi/hooks.json`. The shared phrase "Claude Code-compatible" in its
description denotes event-name familiarity, not config-file compatibility.

## Package: `@ryan_nookpi/pi-extension-claude-hooks-bridge`

- `dist-tags.latest`: `0.2.3`; versions `0.1.0`, `0.2.0`, `0.2.1`, `0.2.2`,
  `0.2.3` (four revision bumps).
- `description`: `Bridge Claude Code hooks (.claude/settings.json) into pi extension lifecycle events.`
- `repository.url`: `git+https://github.com/Jonghakseo/pi-extension.git`
  (monorepo under a different GitHub user/org than the publisher).
- `license`: `MIT`; sole maintainer `ryan_nookpi`; `keywords`: `["pi-package"]`.
- `time.created`: `2026-04-16T07:37:12.405Z`;
  `time.modified`: `2026-07-10T01:44:33.880Z` (still being revised as of the
  fetch date).
- `pi.extensions`: `["./index.ts"]`
- `peerDependencies` (0.2.3): `{"@earendil-works/pi-coding-agent": "*"}`

**Divergence from `@hsingjui/pi-hooks`:** this package reads Claude Code's
actual `.claude/settings.json` as its source-of-truth and bridges it *into*
Pi lifecycle events. The direction is "consume the existing Claude file from
inside Pi", whereas `@hsingjui/pi-hooks` exposes a `hooks` key in Pi settings
that *adopts Claude Code's configuration shape*. The two are complementary in
spirit but distinct in surface and ownership model; they are not aliases of
the same package.

## Package: `@vanillagreen/pi-claude-bridge`

- `dist-tags.latest`: `1.6.1`; 19 published versions.
- `description`: `Pi provider bridge that runs Claude Code through the Claude Agent SDK, with opt-in forwarding for Pi prompt context.`
- `repository.url`: `git+https://github.com/vanillagreencom/vstack.git`
- `license`: `MIT`; sole maintainer `vanillagreencom`.
- `time.created`: `2026-05-06T21:51:27.851Z`
- `pi.extensions`: `["./bundle/index.js"]` with an `image` field
- `peerDependencies` (1.6.1): `{"@earendil-works/pi-ai": "*", "@earendil-works/pi-coding-agent": "*"}`
- `keywords`: `pi-package`, `pi`, `coding-agent`, `claude-code`, `claude-agent-sdk`

**Divergence from `@hsingjui/pi-hooks`:** this is **not a hook-compatibility
package at all**. It is a *model/provider bridge* that runs Claude Code as a
backend through the Claude Agent SDK. Its surface is provider routing, not
hook lifecycle. It is named with the substring `claude-bridge`, which makes
it a frequent false-positive in `claude` + `pi` searches, but its declared
capability is incommensurable with command-hook compatibility.

## Package: `pi-hooks` (unscoped)

- `dist-tags.latest`: `1.0.5`.
- `description`: `Collection of pi extensions (checkpoint, lsp, permission, ralph-loop, repeat)`
- `maintainer`: `prateekmedia`; no `repository` declared.
- `pi.extensions`: `["./checkpoint/checkpoint.ts", "./lsp/lsp.ts", "./lsp/lsp-tool.ts", "./permission/permission.ts", "./ralph-loop/ralph-loop.ts", "./repeat/repeat.ts", "./token-rate/token-rate.ts"]`
- `keywords`: `pi-package`, `pi`, `pi-coding-agent`

**Name-collision risk only.** The unscoped name `pi-hooks` is occupied by an
unrelated bundle of Pi extensions (checkpointing, LSP, permission gating,
loop utilities). It has no Claude Code hook compatibility surface. The
collision is purely lexical; install commands and citations must use the
fully scoped `@hsingjui/pi-hooks` to avoid ambiguity.

## Broader search context (not individual packages)

A broad npm search for `pi-package` + `hooks` returns ~25 packages spanning
many unrelated capabilities: YAML hook automation (`pi-yaml-hooks`), memory
bridges (`pi-icarus-hook`, `pi-memorix`), permission guardrails
(`@thurstonsand/pi-permissions`, `@samfp/pi-steering-hooks`,
`@ssweens/pi-leash`), git hooks (`@artale/pi-git-hooks`), and others. None of
these claim Claude Code command-hook compatibility as their primary identity;
they implement Pi-native hook-like extensions under their own schemas. They
are catalogued here only to document that the search was performed and the
alternatives were considered, not because each is a viable identity claimant.
