---
name: skilltap
description: Use skilltap when setting up, inspecting, reconciling, or troubleshooting the local Codex and Claude Code environment.
---

# Operate the local agent environment

skilltap is a non-interactive control plane for one person's local Codex and
Claude Code setup. It manages explicitly selected resources, shared
instructions, native marketplaces/plugins, complete skill directories, and
drift between desired and observed state. It does not search, rank, browse, or
recommend skills, plugins, or marketplaces.

## Start with the user's intent

- First-time setup: first check whether `skilltap` is available. If it is
  missing, explain that the plugin does not bundle an executable and use the
  verified installer from `https://skilltap.dev/install.sh` (or Homebrew) only
  with the user's authorization. After the binary exists, run `skilltap
  bootstrap --help`, then `skilltap bootstrap`. Treat the binary result and
  each harness result as separate facts; Codex may report an unsupported
  native plugin-install path.
- “What is wrong?” or a health check: run `skilltap status --help`, then
  `skilltap status --json` when structured output is useful.
- Existing native resources should become managed: use `skilltap adopt` after
  reviewing status. Adoption observes and records; it does not push changes.
- “What would change?”: run `skilltap plan` and explain its operations,
  compatibility, ownership, and next actions before mutating anything.
- Apply an accepted plan: run `skilltap sync`. Use `--target` to narrow
  Codex/Claude and the scope flags to choose global or a project.
- Explicit marketplace/plugin/skill lifecycle work: use the matching
  `marketplace`, `plugin`, or `skill` command family. Instructions use
  `instructions`; automatic safe updates use the optional `daemon`.

Always ask the executable for exact syntax at the point of use:
`skilltap --help`, `skilltap <group> --help`, and
`skilltap <group> <command> --help` are authoritative. Do not copy a full flag
table into this skill.

## Scope and safety

Commands are global by default. `--project` selects the current project,
`--project <path>` selects another project, and `--all-scopes` covers global
plus managed projects. `--target codex`, `--target claude`, or `--target all`
select harnesses independently of scope. Use `--json` when passing a stable
result to another agent or script.

Plans disclose partial or destructive consequences. `--yes` acknowledges a
reported partial foreground operation; it does not bypass invalid input,
missing dependencies, unsupported required components, drift, or native
policy. If output names an operation-scoped consequence or asks for a user
decision, stop and convey that decision instead of inventing a bypass.

The canonical global instruction file is `~/AGENTS.md`. skilltap may manage
native Codex/Claude bridges, but existing divergent files, overrides, broken
links, and ownership conflicts are health findings—not permission to overwrite
user content. A skill is its complete directory with a top-level `SKILL.md`;
never reduce it to that file when references or scripts are present.

For configuration paths and instruction precedence, read
[configuration](references/configuration.md) and
[instructions](references/instructions.md). For interpreting results, updates,
daemon behavior, and recovery decisions, read
[diagnostics](references/diagnostics.md).
