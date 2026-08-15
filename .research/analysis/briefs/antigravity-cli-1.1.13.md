---
provenance: agent-synthesis
status: current
updated: 2026-08-15
temporal_contract: ttl-bounded
revisit_after: 2026-11-15
---

# Current Antigravity CLI adapter contract

## Decision

{inferred: architecture decision} skilltap must replace its deprecated Gemini
CLI target with a distinct `agy` target. The products use different executable
identities, version streams, configuration roots, MCP schemas, and plugin
lifecycles. Treating AGY as a Gemini profile could produce writes that
Antigravity does not load. [antigravity-customizations-1-1-13]{56}
[antigravity-cli-lifecycle-1-1-13]{57}

## Supported replacement surface

The attested AGY release documents complete skills under a customization root. The global
root is `~/.gemini/config/`, and the primary workspace root is `.agents/`.
{inferred: adapter mapping} skilltap can therefore publish complete skills to
`~/.gemini/config/skills/` globally and `.agents/skills/` in a project.
[antigravity-customizations-1-1-13]{56}

The detailed MCP contract documents global and plugin-bundled declarations. It
defines stdio servers and SSE servers, but it does not establish a standalone
project MCP file or a non-interactive effective-state observer.
{inferred: authority boundary} skilltap can manage the global MCP document as
an acknowledged declaration while leaving project MCP unsupported and effective
state unverified. [antigravity-customizations-1-1-13]{56}

The native CLI can validate, install, list, enable, disable, and remove a local
plugin in global configuration. Its observed install input is a plugin
directory, and its list output omits revision and scope identity.
{inferred: lifecycle decision} skilltap should not dispatch native plugin
installation until its execution contract can bind the resolved checkout path
and its marketplace/update identity is attested.
[antigravity-cli-lifecycle-1-1-13]{57}

## Compatibility consequences

AGY plugins may bundle rules, hooks, skills, and MCP servers. This replacement
can transfer compatible skills and documented global MCP declarations through
existing managed projection. It cannot yet claim faithful transfer for rule,
hook, enablement, declared-path, or full plugin-bundle behavior.
[antigravity-customizations-1-1-13]{56}

Existing `gemini` policy and inventory refer to an unregistered target after the
replacement. {inferred: product decision} skilltap should report that state
through its ordinary target validation and should not retain an alias or rewrite
user-owned state.

## Contradictions and qualifications

### Standalone rules — `contradiction`

The installed overview lists `.agents/rules/*.md` and trigger-based progressive
disclosure. The detailed rules page documents only hierarchical `AGENTS.md` and
`GEMINI.md` files and says those files do not support frontmatter. The profile
must not grant mutation authority for standalone rule files until this conflict
is resolved. [antigravity-customizations-1-1-13]{56}

### Project MCP — `qualification`

The overview describes workspace customization roots generically, but the MCP
page names only the global file and plugin-bundled files. The detailed component
contract is narrower, so project standalone MCP remains unsupported.
[antigravity-customizations-1-1-13]{56}

## Disconfirming evidence

The isolated lifecycle test confirms more native plugin behavior than the
installed documentation explains. This weakens a claim that AGY has no usable
native lifecycle, but it does not establish the checkout binding, project scope,
marketplace lineage, revision, or update postconditions required by skilltap's
native execution port. [antigravity-cli-lifecycle-1-1-13]{57}

No fetched source establishes that AGY loads Gemini CLI's
`~/.gemini/settings.json`, `.gemini/settings.json`, or `gemini mcp list`
contracts. The replacement must delete those assumptions rather than carry them
forward.

## Revisit triggers

Refresh this contract when AGY changes its customization paths or schemas,
documents structured plugin lifecycle output and update identity, adds a
non-interactive effective-state observer, or resolves the rule and project-MCP
ambiguities.
