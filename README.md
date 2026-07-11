# skilltap

A personal control plane for Codex and Claude Code.

skilltap manages the marketplaces, plugins, skills, and instruction files used by your local agent harnesses. It adopts existing configuration, keeps a normalized machine-wide inventory, and reconciles that inventory through each harness's native mechanisms.

It does not search for skills or recommend plugins. You tell skilltap what you want; skilltap helps install it correctly, keep it updated, and show whether your environment is healthy.

## What It Manages

- Codex and Claude Code harness configuration.
- Native plugin marketplaces.
- Native and materialized plugins.
- Complete standalone skill directories.
- Git-backed skill updates.
- Global and project instructions.
- Cross-harness compatibility.
- Configuration drift and ownership.
- Optional safe automatic updates.

## Native First

skilltap uses native harness commands whenever they exist.

A Claude plugin stays a Claude plugin. A Codex plugin stays a Codex plugin. When the same plugin exists natively for both harnesses, skilltap installs both native distributions.

When no native target exists, skilltap can materialize compatible components into the target harness. Partial or lossy results are reported and require explicit approval.

## Agent Forward

skilltap is deterministic and non-interactive.

Human-readable output is concise enough for an agent transcript. Inspection and planning commands also support `--json`. When a user decision is required, output explains the incompatibility and gives the exact next action.

There is no TUI, setup wizard, or separate agent mode.

## Quick Start

Enable the harnesses you want skilltap to manage:

```bash
skilltap harness enable codex
skilltap harness enable claude
```

Adopt their existing global configuration:

```bash
skilltap adopt
```

Establish shared global instructions:

```bash
skilltap instructions setup
```

This uses `~/AGENTS.md` as the canonical global file and creates the required native harness bridges.

Inspect the complete managed computer:

```bash
skilltap status --all-scopes
```

Preview and reconcile desired state:

```bash
skilltap plan --all-scopes
skilltap sync --all-scopes
```

## Marketplaces and Plugins

Register an explicit marketplace:

```bash
skilltap marketplace add example/agent-plugins
```

Install an exact plugin:

```bash
skilltap plugin install formatter@example-plugins
```

Target one harness when needed:

```bash
skilltap plugin install formatter@example-plugins --target claude
```

skilltap lists registered marketplaces and installed plugins. It does not browse or search marketplace catalogs.

## Standalone Skills

A skill is a complete directory with `SKILL.md` at its root:

```text
commit-helper/
├── SKILL.md
├── scripts/
├── references/
└── templates/
```

Install a top-level skill directory:

```bash
skilltap skill install ./commit-helper
```

Install a skill from an explicit repository subdirectory:

```bash
skilltap skill install https://github.com/example/agent-tools \
  --path skills/commit-helper
```

The whole directory is installed and tracked. For Git-backed skills, skilltap records the resolved commit SHA and detects updates when that SHA changes.

## Global and Project Scope

Commands operate globally unless project scope is requested:

```bash
# Global
skilltap plugin install formatter@example-plugins

# Project containing the current directory
skilltap plugin install formatter@example-plugins --project

# Another project
skilltap plugin install formatter@example-plugins --project ~/src/my-app

# Every managed scope
skilltap status --all-scopes
```

`--target` selects harnesses. Scope selects where the resource applies. They can be combined:

```bash
skilltap sync --project --target claude
```

skilltap stores its own state outside repositories. It writes project files only when they are native harness resources or instruction files named in the reconciliation plan.

## Instructions

Global instructions use:

```text
~/AGENTS.md
```

Native global bridges normally use:

```text
~/.codex/AGENTS.md -> ~/AGENTS.md
~/.claude/CLAUDE.md -> ~/AGENTS.md
```

Project instructions use:

```text
<project>/AGENTS.md
<project>/CLAUDE.md -> AGENTS.md
```

Existing user-authored files are preserved. Conflicting content is reported for review rather than overwritten.

## Compatibility

Resources are classified as:

- Native.
- Faithfully equivalent.
- Materializable.
- Partial.
- Unsupported.

Safe native and faithful changes can proceed normally. A partial foreground operation requires `--yes`:

```bash
skilltap plugin install deploy@claude-tools --target codex --yes
```

Optional `--include` and `--exclude` selectors control individual components.

`--yes` acknowledges the reported partial result. It does not override missing required components, local drift, or invalid configuration.

## Updates

Check or apply plugin and skill updates:

```bash
skilltap plugin update --all-scopes
skilltap skill update --all-scopes
```

Git-backed skills update when their requested ref resolves to a new commit SHA. Pinned commits remain fixed. Local edits block automatic replacement.

Safe automatic updates are optional:

```bash
skilltap daemon enable --interval 6h
skilltap daemon status
```

The daemon updates managed plugins and skills across all scopes. It never approves partial updates, overwrites local drift, or resolves conflicts.

## Supported Harnesses

- Codex
- Claude Code

Additional harnesses belong only when skilltap can observe and operate their native systems faithfully. Filesystem copying alone does not count as support.

## Documentation

- [Vision](docs/VISION.md)
- [Specification](docs/SPEC.md)
- [Architecture](docs/ARCH.md)
- [Command-line experience](docs/UX.md)
- [Harness contracts](docs/HARNESS-CONTRACTS.md)

## License

See [LICENSE](LICENSE).
