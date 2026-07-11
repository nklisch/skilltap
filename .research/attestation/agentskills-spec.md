---
source_handle: agentskills-spec
fetched: 2026-07-10
source_url: https://agentskills.io/specification
provenance: source-direct
substrate_confidence: source-direct
---

# Agent Skills specification

## Structural metadata

- Publisher: Agent Skills project
- Document type: normative format specification
- Page title: Specification
- Scope: skill directory structure, `SKILL.md`, optional resources, progressive disclosure, and validation

## Attested summary

The specification defines a skill as a directory whose minimum content is a
top-level file named exactly `SKILL.md`. The directory may also contain
scripts, references, assets, and other files or directories. `SKILL.md` is YAML
frontmatter followed by Markdown instructions.

The required frontmatter fields are `name` and `description`. `name` is one to
64 characters, uses lowercase ASCII letters, digits, and hyphens, cannot start
or end with a hyphen or contain consecutive hyphens, and must equal the parent
directory name. `description` is one to 1024 characters and should say both
what the skill does and when it applies.

Optional fields are `license`, `compatibility`, `metadata`, and
`allowed-tools`. `compatibility`, when present, is one to 500 characters and
describes environmental requirements such as an intended product, required
software, or network access. `metadata` is an arbitrary string-to-string map.
`allowed-tools` is a space-separated string, is experimental, and may not be
supported consistently by clients.

The Markdown body has no prescribed section format. The specification names
`scripts/`, `references/`, and `assets/` as conventional optional directories,
while also allowing additional files or directories. Referenced paths are
relative to the skill root. Supporting resources are intended to load only as
needed rather than with the initial catalog metadata.

The disclosure model has three levels: clients initially load `name` and
`description`; load the full `SKILL.md` after activation; and load individual
resources only when required. The page recommends keeping the instructions
below 5,000 tokens and 500 lines and avoiding deep chains of references. It
points to the project's `skills-ref` library for validation.

## Key passages by source-internal anchor

> “A skill is a directory containing, at minimum, a `SKILL.md` file.” — Directory structure

- **Directory structure:** the minimum unit is a directory containing
  `SKILL.md`; the illustrated skill includes optional `scripts/`,
  `references/`, and `assets/`, plus arbitrary additional content.
- **SKILL.md format / Frontmatter:** YAML frontmatter precedes Markdown;
  `name` and `description` are required, and the optional fields and their
  types and limits are tabulated.
- **name field:** the name grammar, length, no-edge-hyphen,
  no-consecutive-hyphen, and parent-directory equality rules are explicit.
- **compatibility field:** this field describes product and environmental
  requirements and is usually unnecessary for otherwise portable skills.
- **allowed-tools field:** tool pre-approval is explicitly experimental and
  client support may vary.
- **Optional directories:** scripts are executable support code, references
  are on-demand documentation, and assets are static resources.
- **Progressive disclosure:** metadata, complete instructions, and resources
  are three successively loaded layers.
- **File references:** supporting paths are resolved relative to the skill
  root.
