---
provenance: agent-synthesis
updated: 2026-07-10
facet: agent-skills
temporal_contract: supersedes-prior
---

# Agent Skills format contract

## Status and source boundary

This facet treats the published specification as the format authority and the
official client implementation guide as non-normative interoperability
guidance. It does not use the imported Claude marketplace findings as
substrate.

## Canonical managed unit

A conforming skill is the **complete directory** whose root contains an exact
`SKILL.md`; scripts, references, assets, and other files can be part of that
same skill. `SKILL.md` alone is therefore not the full managed artifact when
the directory contains supporting content. Relative references are resolved
from the skill root. [agentskills-spec]{20}

`SKILL.md` must contain YAML frontmatter followed by Markdown. Its required
fields are `name` and `description`; the Markdown body itself has no mandated
section structure. [agentskills-spec]{20}

The strict `name` contract is one to 64 characters, lowercase ASCII letters,
digits, and hyphens only; it cannot begin or end with a hyphen or contain
consecutive hyphens, and it must equal the parent directory name. A
`description` is one to 1024 characters and should describe both capability
and activation conditions. [agentskills-spec]{20}

## Optional metadata and compatibility

The portable optional fields are `license`, `compatibility`, `metadata`, and
experimental `allowed-tools`. `compatibility` is descriptive environment
metadata rather than proof that two clients implement the same behavior; it
can name an intended product, software dependencies, or network needs.
`metadata` is an extensibility map with string keys and values.
[agentskills-spec]{20}

`allowed-tools` cannot be assumed portable because the specification labels it
experimental and explicitly says implementation support may vary.
[agentskills-spec]{20}

{inferred: portability boundary} A skill that uses only the required fields,
Markdown instructions, and resources supported by both target harnesses has a
faithful common representation. Product-specific frontmatter, tool names,
execution semantics, or environmental assumptions require adapter-level
compatibility analysis; directory-format conformance alone does not establish
behavioral equivalence. [agentskills-spec]{20}

## Discovery and loading expectations

Skills-compatible clients follow progressive disclosure: catalog `name` and
`description`, load full instructions when activated, and load individual
resources on demand. Clients may expose activation through ordinary file reads
or a dedicated tool, and may pass either the full file or only its body to the
model. [agentskills-client-implementation]{21}

The core format does not mandate an installation path. The official guide
describes project and user scopes and identifies `.agents/skills/` as a widely
adopted cross-client convention, while allowing client-native locations and
other deployment models. [agentskills-client-implementation]{21}

Within a configured skills location, the client looks for directories
containing exact `SKILL.md`; the directory remains necessary for resolving and
enumerating bundled resources. [agentskills-client-implementation]{21}

## What the standard does not define

{inferred: specification scope} The current format specification does not
define a marketplace, plugin manifest, registry, source provenance model,
installation protocol, update protocol, dependency resolver, lockfile, or
normative version-selection behavior. The optional `metadata` map can carry a
producer-defined version string, but the specification assigns it no update or
resolution semantics. [agentskills-spec]{20}

The standard also does not choose the client’s storage paths, same-scope
collision precedence, activation syntax, permission model, trust policy,
context-compaction strategy, or subagent behavior; the implementation guide
presents these as client decisions. [agentskills-client-implementation]{21}

## Adapter implications

The following are skilltap design implications, not source claims:

- Manage, hash, copy, link, and update the whole skill root, never only
  `SKILL.md`; require top-level `SKILL.md` before classifying a directory as a
  skill.
- Validate strict authoring conformance in state/adoption diagnostics, but
  preserve the harness adapter's observed ability to load nonconforming input
  as a separate fact.
- Use `.agents/skills/` as skilltap's portable canonical placement when both
  harness contracts support it, while treating every harness-native path as an
  adapter concern rather than part of Agent Skills itself.
- Preserve unknown files and directories inside the managed skill because the
  specification permits additional content and resources can be operationally
  significant.
- Parse `compatibility` and known extension fields when evaluating faithful
  transfer. Treat experimental or harness-specific fields as possible partial
  boundaries; never infer equivalent semantics from a matching field name.
- Keep source, revision, Git SHA, update policy, and ownership in skilltap
  state. Do not write those control-plane semantics into `SKILL.md` as though
  Agent Skills defined them.
- skilltap's prohibition on marketplace discovery is compatible with the
  format: skilltap can place already-selected skill directories where the
  harness discovers installed skills without browsing or recommending remote
  inventory.

## Disconfirming analysis

The strict specification was checked against the official client guide. The
guide deliberately recommends lenient consumption of some invalid names or
frontmatter values, so strict validity does not imply that every real client
will reject the same artifact. Conversely, client tolerance does not make the
artifact conforming. [agentskills-spec]{20}
[agentskills-client-implementation]{21}

The specification's optional `allowed-tools` field was checked for a uniform
execution guarantee; the source instead labels it experimental and warns that
support varies. No portable enforcement guarantee is attested.
[agentskills-spec]{20}

The official guide was checked for a mandated filesystem location; it
explicitly says the specification does not mandate one, despite recommending
`.agents/skills/` for interoperability. [agentskills-client-implementation]{21}

## Contradictions

### Strict authoring versus lenient consumption — `tension`

- The specification defines exact name and directory matching constraints for
  conforming skills. [agentskills-spec]{20}
- The client guide recommends warning and loading where possible for some
  violations, explicitly calling this a deliberate relaxation for
  cross-client compatibility. [agentskills-client-implementation]{21}

These positions address different layers. skilltap should report both
conformance and observed harness loadability rather than merging them into one
boolean.

## Unknowns

- Whether future clients converge on strict rejection or continued leniency is
  not established by these sources.
- The exact semantics that Codex and Claude assign to experimental or extended
  frontmatter belong to their native contracts, not this facet.
- Absence claims about distribution and lifecycle are temporal; revisit if the
  official specification adds package or provenance primitives.

## Revisit if

- `agentskills.io/specification` changes its required or optional fields,
  directory rules, or experimental status markers.
- The Agent Skills project publishes a normative package, distribution,
  dependency, versioning, or installation specification.
- Codex or Claude changes its support for `.agents/skills/`, resource
  directories, or extended frontmatter.

## Bibliography mapping

- `{20}` → `agentskills-spec` → https://agentskills.io/specification
- `{21}` → `agentskills-client-implementation` → https://agentskills.io/client-implementation/adding-skills-support
