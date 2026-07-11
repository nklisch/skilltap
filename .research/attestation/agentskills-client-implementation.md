---
source_handle: agentskills-client-implementation
fetched: 2026-07-10
source_url: https://agentskills.io/client-implementation/adding-skills-support
provenance: source-direct
substrate_confidence: source-direct
---

# Agent Skills client implementation guide

## Structural metadata

- Publisher: Agent Skills project
- Document type: official implementation guidance for client authors
- Page title: How to add skills support to your agent
- Scope: discovery, parsing, catalog disclosure, activation, resource access, and context retention

## Attested summary

The guide describes implementation choices around a common three-stage
loading model: cataloging `name` and `description`, activating full
instructions, and accessing bundled resources individually. It separates the
format specification from client choices about filesystem scopes, discovery
mechanism, activation mechanism, and context management.

For local clients, the guide recommends project and user scopes and suggests
scanning both client-native locations and `.agents/skills/`. It explicitly
states that the format specification does not mandate where skills are stored;
`.agents/skills/` is described as a widely adopted interoperability
convention. Within a configured skill directory, a skill is a subdirectory
containing a file named exactly `SKILL.md`.

The guide recommends deterministic collision handling, with project scope
overriding user scope, and warning when one skill shadows another. It also
recommends trust gating for repository-provided project skills.

For parsing, the guide extracts required `name` and `description`, optional
frontmatter, and the Markdown body. Although the specification's authoring
rules are strict, the guide recommends that clients warn and continue for some
name violations, while skipping a skill if its description is absent or YAML
cannot be parsed. It identifies that approach as a deliberate compatibility
relaxation rather than a change to the strict format.

Clients may activate skills by letting the model read `SKILL.md` or through a
dedicated activation tool. The model can receive the whole file or only its
Markdown body. Supporting resources may be enumerated but should not be read
eagerly. Relative paths resolve from the directory containing `SKILL.md`.

The guide treats filtering, permissions, explicit user activation syntax,
deduplication, preservation through context compaction, and subagent execution
as client implementation choices rather than fields imposed by the core
format.

## Key passages by source-internal anchor

> “The Agent Skills specification does not mandate where skill directories live.” — Where to scan

- **The core principle: progressive disclosure:** catalog metadata, full
  instructions, and individual resources are the shared lifecycle.
- **Where to scan:** project and user locations are implementation choices;
  `.agents/skills/` is an interoperability convention, not a mandated format
  location.
- **What to scan for:** a direct child directory with exact `SKILL.md` is the
  discovery target.
- **Handling name collisions:** project precedence is presented as the common
  rule; same-scope precedence remains client-defined.
- **Lenient validation:** clients may load some strictly nonconforming skills
  with warnings, but missing descriptions or unparseable YAML prevent loading.
- **What to store:** the location of `SKILL.md` also supplies the skill root
  used for relative resources.
- **Model-driven activation / What the model receives:** activation may use a
  file read or dedicated tool, and clients may preserve or strip frontmatter.
- **Listing bundled resources:** clients can enumerate support files without
  eagerly loading them.
- **Subagent delegation:** executing a skill in a separate agent session is an
  optional client extension.
