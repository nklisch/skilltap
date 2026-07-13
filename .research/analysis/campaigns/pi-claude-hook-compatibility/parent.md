---
provenance: agent-synthesis
status: current
updated: 2026-07-12
temporal_contract: re-engage-on-trigger
scope: Pi compound target Claude Code hook compatibility
---

# Pi Claude-hook compatibility capability brief

## Decision

`@hsingjui/pi-hooks@0.0.2` is the currently identifiable Pi package whose
primary purpose is Claude Code command-hook compatibility. Its npm identity,
Pi extension entrypoint, global/project package scopes, settings paths, and
independent lifecycle are observable through supported Pi and npm surfaces.
[pi-hooks-identity-npm]{4} [pi-hooks-identity-installed]{6}
[pi-hooks-health-pi-runtime]{11}

{inferred: admission decision} **The current package does not satisfy
skilltap's hook-equivalence contract and must not grant mutation authority to
the Pi compound target.** It is a partial, best-effort shim: it implements nine
of roughly thirty Claude hook events, supports only command hooks, and diverges
on load-bearing timing, payload, blocking, timeout, async, matcher, and safety
semantics. [pi-hooks-source]{1} [claude-hooks-reference]{3}

{inferred: adapter consequence} The Pi adapter may implement observation and
status for Pi core, `pi-mcp-adapter`, and `@hsingjui/pi-hooks` as three distinct
facts. A compiled Pi profile remains observe-only when this hook package is the
available companion. Status should report the package as present/absent,
configured/inert, version-current/unknown, and **semantically partial**, rather
than describing it as healthy merely because `pi list` returns a path.
[pi-hooks-health-pi-runtime]{11} [pi-hooks-health-installed-package]{9}

This decision does not erase the independently attested Pi skills and MCP load
surfaces. It applies to the epic's chosen **compound-target mutation gate**:
that gate requires both companions to be healthy and compatible, and the hook
companion fails the semantic half of that gate.

## Source map

| Number | Handle | Source role |
|---:|---|---|
| 1 | `pi-hooks-source` | installed package source, v0.0.2 |
| 2 | `pi-extension-events` | installed Pi extension-event reference, v0.80.6 |
| 3 | `claude-hooks-reference` | current Anthropic hooks reference |
| 4 | `pi-hooks-identity-npm` | npm release and identity metadata |
| 5 | `pi-hooks-identity-github` | repository history, tags, and release state |
| 6 | `pi-hooks-identity-installed` | installed manifest and README |
| 7 | `pi-hooks-identity-alternatives` | alternative-package disconfirmation |
| 8 | `pi-hooks-identity-pi-docs` | official Pi package documentation/catalog |
| 9 | `pi-hooks-health-installed-package` | installed health/config source inspection |
| 10 | `pi-hooks-health-npm-registry` | npm update identity |
| 11 | `pi-hooks-health-pi-runtime` | live `pi list`, help, version, and settings observation |
| 12 | `pi-hooks-health-mcp-adapter-installed` | installed MCP-adapter ownership surface |
| 13 | `pi-hooks-health-github-source` | repository-side version evidence |

All handles resolve to source-direct attestations. Specialist analyses were used
as composition lenses and are not citation targets. Citation lint, adversarial
semantic-chain review, isolated evaluation, and lead spot-check completed with
no unresolved material findings.

## Package and lifecycle contract

The exact package is `@hsingjui/pi-hooks`, latest and locally installed at
`0.0.2`. The published Pi manifest exposes `./src/pi-hooks.ts`; version `0.0.2`
changed the peer dependency to `@earendil-works/pi-coding-agent: "*"`.
[pi-hooks-identity-npm]{4} [pi-hooks-identity-installed]{6}

The npm packument is the release authority. The repository has no tags or
GitHub releases, and the commit recorded as npm `0.0.2` still declares
`version: 0.0.1` in its repository manifest. A profile must compare the
installed checkout manifest with npm `dist-tags.latest`; it must not derive npm
currency from repository HEAD. [pi-hooks-identity-github]{5}
[pi-hooks-health-npm-registry]{10}

Pi documents global and project package installation through
`~/.pi/agent/settings.json` and `.pi/settings.json`. Package identity and
resolved path are available through `pi list`, but that command does not expose
version, initialization health, or per-resource enable state. Resource
enable/disable is documented only through the interactive `pi config` surface.
[pi-hooks-health-pi-runtime]{11}

{inferred: ownership rule} Existing companion packages remain user-owned unless
explicitly adopted. Observation must not convert a `packages` entry into
skilltap provenance, and no compiled profile may infer health from installation
alone.

## Semantic compatibility

### Supported subset

The package binds `SessionStart`, `SessionEnd`, `UserPromptSubmit`,
`PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `Stop`, `PreCompact`, and
`PostCompact` onto Pi extension events. It executes command hooks with JSON on
stdin and can provide SessionStart context, block a PreToolUse call with exit
code 2, and patch Pi tool results through Pi's middleware return shape.
[pi-hooks-source]{1} [pi-extension-events]{2}

These narrow paths are useful compatibility evidence, but they do not establish
equivalence for an arbitrary Claude plugin hook set.

### Material incompatibilities

- The package supports only `command`; Claude also supports `http`, `mcp_tool`,
  `prompt`, and `agent` hook types on applicable events.
  [pi-hooks-source]{1} [claude-hooks-reference]{3}
- Its `async` field is declared but never read, so a Claude hook requesting
  background execution blocks in Pi instead. [pi-hooks-source]{1}
- `Stop` is bound to Pi `agent_end`, which may precede retries, compaction
  recovery, or queued continuations; Claude Stop is a logical response boundary
  with interrupt/failure distinctions. [pi-extension-events]{2}
  [claude-hooks-reference]{3}
- The package has no Claude-equivalent eight-block Stop safety cap.
  [pi-hooks-source]{1} [claude-hooks-reference]{3}
- PreToolUse `updatedInput` merges rather than replaces, and PostToolUse reads
  `updatedToolResult` rather than Claude's `updatedToolOutput`.
  [pi-hooks-source]{1} [claude-hooks-reference]{3}
- Exit code 2 blocks only PreToolUse; Claude defines event-specific blocking or
  feedback behavior for additional events. [pi-hooks-source]{1}
  [claude-hooks-reference]{3}
- SessionEnd collapses every reason to `other` and uses a 60-second default
  instead of Claude's 1.5-second SessionEnd budget. [pi-hooks-source]{1}
  [claude-hooks-reference]{3}
- Claude's capitalized tool names and tool-input fields are forwarded as Pi's
  native lowercase names and Pi-native fields, so existing matchers and scripts
  can silently miss or read different input. [pi-hooks-source]{1}
  [claude-hooks-reference]{3}

{inferred: compatibility classification} A plugin containing any hook must be
analyzed against this subset. A hook using an unsupported event/type or a
materially divergent field is partial or blocked under the normal required-vs-
optional component rules; package presence cannot upgrade that classification.

## Health and compound-profile observation

The observed machine illustrates why presence and health must be separate.
`pi list` reports both `npm:@hsingjui/pi-hooks` and `npm:pi-mcp-adapter`, but the
global settings file has no `hooks` key. The hook extension therefore loads but
all callbacks are inert. [pi-hooks-health-pi-runtime]{11}
[pi-hooks-health-installed-package]{9}

The two companions have independent package identities and disjoint persistent
configuration surfaces: pi-hooks reads the `hooks` key in Pi settings, while
pi-mcp-adapter reads its `mcp.json` family. They can be updated or removed
independently. [pi-hooks-health-installed-package]{9}
[pi-hooks-health-mcp-adapter-installed]{12}

They still interact behaviorally. A configured `PreToolUse` matcher for
`mcp__.*` can observe, deny, or rewrite MCP-adapter tool calls. {extends} Status
and plan output should therefore identify hook policy as a possible cause when
MCP configuration exists but MCP tools are blocked. [pi-hooks-source]{1}

## Required Pi adapter evidence

A future mutation-capable Pi profile must establish all of the following before
replacing the observe-only result:

1. Pi core version is a verified compiled version; runtime probes may only
   narrow it.
2. `pi-mcp-adapter` has a separately attested supported version, documented
   global/project config surfaces, and effective MCP observation.
3. The hook companion has stable package/version identity and observable
   configured/effective health, not only package presence.
4. Event/type/payload/timing/blocking semantics satisfy the hook subset that
   skilltap promises. An adapter may narrow its promise, but it may not call a
   partial shim broadly Claude-compatible.
5. Project trust and resource enablement are observable enough to avoid false
   health claims.
6. Existing package/config ownership is preserved until explicit adoption.
7. Immediate-repeat mutation produces no changes and post-observation verifies
   both companions independently.

The current package clears parts of items 3 and 6 at the identity/lifecycle
level, but fails item 4 and leaves enablement/initialization gaps in item 5.

## Contradictions

### Package claim versus semantic contract — `qualifies`

The package accurately describes itself as a Claude-Code-shaped, best-effort
command-hook adapter. Its name and description do not establish full contract
equivalence. Source comparison shows the implementation is a strict and
behaviorally divergent subset. [pi-hooks-identity-npm]{4}
[pi-hooks-source]{1} [claude-hooks-reference]{3}

### npm release versus repository version — `contradicts`

npm and the installed package identify `0.0.2`; the repository commit npm names
as its source still identifies `0.0.1`, and no release tags exist. Both cannot
be the version authority for the same npm installation. npm governs the
published package; repository HEAD remains non-authoritative evidence.
[pi-hooks-identity-npm]{4} [pi-hooks-identity-github]{5}

### Pi package scope versus hook merge scope — `incommensurable`

Pi's package identity rule gives a project package entry precedence over its
global counterpart, while pi-hooks concatenates global and project hook groups.
The rules govern different keys and cannot be reduced to one precedence model.
A controller must preserve package precedence and hook concatenation
separately. [pi-hooks-health-installed-package]{9}

### Installed versus active — `tension`

Pi's package list truthfully reports that the extension is installed. The
extension is nevertheless behaviorally inert when no `hooks` key exists. The
facts are compatible only when status distinguishes presence from effective
health. [pi-hooks-health-pi-runtime]{11}

## Disconfirming analysis

- Searches across npm alternatives found no package with the same complete
  identity markers; the closest packages use different configuration sources,
  narrower event sets, reverse bridge direction, or unrelated provider
  behavior. [pi-hooks-identity-alternatives]{7}
- Pi's package docs/catalog provide no official hook package endorsement; this
  package is community-supplied, not Pi-core authority. [pi-hooks-identity-pi-docs]{8}
- Source comparison rejected full event coverage, working async behavior,
  general exit-2 blocking, faithful Stop timing, and faithful update fields.
  [pi-hooks-source]{1} [pi-extension-events]{2}
  [claude-hooks-reference]{3}
- Live observation rejected the proposition that installation implies active
  hook behavior. [pi-hooks-health-pi-runtime]{11}
- Repository inspection rejected git HEAD as reliable npm release currency.
  [pi-hooks-identity-github]{5} [pi-hooks-health-npm-registry]{10}

No disconfirming source established a current hook companion that clears the
compound-profile semantic gate.

## Revisit if

- `@hsingjui/pi-hooks` adds the missing event/type coverage or fixes the
  load-bearing async, Stop, exit-code, timeout, matcher, and update semantics.
- Another package establishes a stronger source-direct Claude hook equivalence
  contract and supported Pi package lifecycle.
- Pi core adds native Claude-compatible hooks or a stable equivalent event
  layer.
- Pi adds non-interactive, structured version/health/enablement observation.
- The product deliberately narrows the Pi target contract to a named faithful
  subset of Claude hooks and rolls the foundation docs forward accordingly.
- Claude or Pi changes the event contracts used in this comparison.

## Acquisition candidates

No load-bearing claim is acquisition-blocked. Enriching candidates are
consolidated in `acquisitions.md`; they do not alter the current observe-only
verdict.
