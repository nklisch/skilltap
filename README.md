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

Read-only `status` works before configuration exists and creates nothing. A
missing `config.toml` means neither harness is enabled; skilltap never infers
management policy merely because a Codex or Claude executable is installed.

For normal human use, ask your agent for the outcome instead of translating it
into flags yourself. For example:

> Use skilltap to check the health of my Codex and Claude environment.

> Use skilltap to sync my global plugins and shared instructions. Show me the
> plan first, and ask before accepting any partial result.

> Use skilltap to install `formatter@example-plugins` in this project and tell
> me if any part will not work in both harnesses.

The agent can learn exact syntax from `skilltap --help`, run read-only status
and planning commands, and convey any required decision before mutation.

## Quick Start

The native plugin is the primary installation path. Add this repository as a
marketplace, then install or enable the plugin in the harness you use:

```bash
# Claude Code
claude plugin marketplace add nklisch/skilltap --scope user
claude plugin install skilltap@skilltap --scope user

# Codex marketplace
codex plugin marketplace add nklisch/skilltap
codex plugin add skilltap@skilltap
```

Let skilltap verify the binary and harness setup:

```bash
skilltap bootstrap
```

Need the standalone binary directly? Use the online installer after the plugin
instructions:

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

The plugin path and online installer report binary availability separately
from Claude/Codex harness setup. A missing harness or unsupported native
plugin lifecycle is an explicit next action, not a reason to write a native
cache. Homebrew installs the binary only (`brew install
nklisch/skilltap/skilltap`); run `skilltap bootstrap` afterward when you want
the plugin/skill setup.

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

Internally, every managed resource instance has a logical ID plus one concrete
scope. That exact pair is its `ResourceKey`, so the same plugin or skill can
exist independently in global scope and in multiple projects without
colliding. Component selectors resolve within the scopes selected by the
command and every resulting operation retains the exact scoped key.

## Instructions

Global instructions use:

```text
~/AGENTS.md
```

Native global bridges normally use:

```text
${CODEX_HOME:-$HOME/.codex}/AGENTS.md -> ~/AGENTS.md
~/.claude/CLAUDE.md -> ~/AGENTS.md
```

Project instructions use:

```text
<project>/AGENTS.md
<project>/CLAUDE.md -> AGENTS.md
```

Existing user-authored files are preserved. Conflicting content is reported for review rather than overwritten.

Project-shared Claude declarations remain visible to `status` because they can
affect the effective environment. They are not adoptable into skilltap's
personal project scope; skilltap does not turn a repository's shared
declaration into personal desired state.

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

Mutation support comes only from capability profiles compiled into skilltap
and matched to the exact observed harness executable and version. Runtime
probes may narrow compiled support, never grant it. Unknown harness versions
remain observable but receive no mutation authority.

Fresh declared/effective observations and health findings are ephemeral.
`status` and `adopt` do not persist those snapshots to `state.json`; state
retains provenance and successful apply history instead.

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
