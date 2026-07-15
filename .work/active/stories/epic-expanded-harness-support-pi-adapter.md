---
id: epic-expanded-harness-support-pi-adapter
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-pi
depends_on: [epic-expanded-harness-support-pi-profile]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/analysis/campaigns/pi-claude-hook-compatibility/parent.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Implement Pi Core and Companion Observation

## Checkpoint

Implement an unregistered `PiAdapter` plus `PiConditionalProfile` that observes
Pi core, `pi-mcp-adapter`, and `@hsingjui/pi-hooks` as separate facts at global
and project scope. Pin the current exact tuple as known but mutation-unsupported.

## Native contract

- Core: executable `pi`, direct argv `--version`, exact attested version
  `0.80.6`, Pi home `~/.pi/agent`.
- MCP companion: npm identity `pi-mcp-adapter`, exact attested version `2.11.0`,
  documented `mcp.json` family, compatible declaration mapping but
  non-interactive activation health unverified.
- Hook companion: npm identity `@hsingjui/pi-hooks`, exact attested version
  `0.0.2`, entrypoint `./src/pi-hooks.ts`, `settings.json` `hooks` key,
  semantic compatibility always partial for this version. Missing hooks is
  inert; configured hooks remain activation-unverified.
- Existing package/config artifacts are `Ownership::Harness` and non-adoptable.
- Global/project package precedence and global-then-project hook concatenation
  remain separate rules.

## Paths and ports

- Add validated Pi/package roots, including supported `PI_PACKAGE_DIR`, through
  `PlatformPaths` and root-confined no-follow reads.
- `PiSkillProjection` returns `~/.agents/skills` globally and project
  `.agents/skills`; Pi-native `.pi/skills` is observed only.
- Observe settings, portable/Pi skill roots, and documented MCP declaration
  files with static surface labels.
- Do not parse `pi list` human output; it provides weaker settings-derived
  presence/path evidence and no version or health.
- Do not expose native lifecycle, managed projection, package update/removal,
  instruction bridge, or cache mutation.

## Exact profile

Compiled tuple id:
`pi-0-80-6-mcp-2-11-0-hooks-0-0-2`.

It supports precise core/skill observation, narrows MCP effectiveness to
unverified, marks hook compatibility unsupported, and sets every `skill.*`,
`plugin.*`, and `marketplace.*` mutation capability unsupported in both scopes.
Missing, malformed, or unknown component tuples are observe-only and cannot
fall back to core-only mutation.

## Files

- `crates/core/src/runtime/error.rs`
- `crates/core/src/runtime/paths.rs`
- `crates/harnesses/src/adapters/pi.rs` (new)
- `crates/harnesses/src/adapters/pi_profile.rs` (new)
- `crates/harnesses/src/adapters/pi_settings.rs` (new)
- `crates/harnesses/src/adapters/mod.rs`
- `crates/harnesses/src/lib.rs`

## Acceptance evidence

- Exact version bytes/profile and neighboring unknown versions are pinned.
- Package settings declaration, manifest name/version/entrypoint, configuration,
  trust, activation, and compatibility remain separate.
- A configured hook never upgrades `0.0.2` from partial.
- Malformed/missing/unknown one companion preserves the sibling observation but
  blocks the aggregate.
- MCP cache and secrets never enter output/state and are never written.
- Project skill destination is canonical/no-link; `.pi/skills` remains an
  unmanaged observed sibling.

## Ordering

Depends on the conditional-profile contract. Adapter remains outside the
canonical registry until integration and output guards are ready.
