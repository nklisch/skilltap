# Configuration and state layout

skilltap describes one person's managed computer. It does not create a
repository manifest for collaborators and it does not replace native harness
configuration formats.

## Machine-wide state

The state directory is `${XDG_CONFIG_HOME:-$HOME/.config}/skilltap/`:

```text
config.toml       operating policy and enabled harnesses
inventory.toml    desired resources and their scopes
state.json        machine-written provenance and apply observations
managed/          skilltap-owned complete skill/plugin trees and backups
```

The first state-changing command creates what it needs. Read-only `status`
does not synthesize configuration. `config.toml` records enabled harnesses,
instruction bridge policy, update mode, and daemon interval. `inventory.toml`
is the human-readable desired state: explicit marketplaces, plugins, skills,
targets, scopes, sources, and update choices. `state.json` records provenance,
resolved Git SHAs, fingerprints, native versions, and apply history; do not
edit it as desired configuration. Authentication material and secrets never
belong in any skilltap file.

Keep three observations distinct when explaining a result:

1. Desired inventory says what skilltap intends.
2. Native declared state says what a harness configuration declares.
3. Effective installed state says what a harness can load or has cached.

A marketplace refresh is not proof that every plugin updated. A project
declaration is not proof that every user installed or trusted the plugin.

## Scope and targets

Commands are global by default. `--project` selects the Git project containing
the current directory; `--project <path>` selects the project containing that
path. `--all-scopes` includes global plus every project already recorded in
inventory. `--project` and `--all-scopes` cannot be combined.

Use `skilltap harness list` for registered target ids. `--target <id>` selects
one harness and `--target all` selects every enabled harness independently of
scope. Omitted targets use the enabled harness policy. An installed binary is
not enabled until `harness enable` records that policy.

Skills are complete directories with a top-level `SKILL.md`; references,
scripts, and assets remain siblings in the same managed tree. Compatible
standalone skills use `.agents/skills/` as the canonical managed form. Native
links or copies are adapter projections only when a target requires them.

For exact flags, output fields, and exit codes, ask the binary for help rather
than relying on this conceptual layout.
