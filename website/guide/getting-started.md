---
description: Install skilltap and adopt existing local harness environments.
---

# Getting Started

## Install

The plugin is skilltap's primary distribution surface. Add the marketplace,
then install or enable the plugin in the harness you already use:

```bash
# Claude Code
claude plugin marketplace add nklisch/skilltap --scope user
claude plugin install skilltap@skilltap --scope user

# Codex (marketplace registration)
codex plugin marketplace add nklisch/skilltap
codex plugin add skilltap@skilltap
```

Both harnesses install the plugin natively after registration. Older Codex
builds without `plugin add` can install or enable it through `/plugins`.

### Install the standalone binary

After marketplace setup, use the online installer when you want the binary
directly or need to set up a harness without its plugin flow:

```bash
# macOS or Linux
curl -fsSL https://skilltap.dev/install.sh | sh

# Or Homebrew
brew install nklisch/skilltap/skilltap
```

The installer verifies the release checksum, installs the user-level binary,
and invokes `skilltap bootstrap` to detect Claude Code and Codex independently.
Harness absence or an unsupported native lifecycle is reported as attention;
the verified binary remains available. The executable is the authoritative
command discovery surface:

```console
skilltap bootstrap --help
```

Verify the binary:

```bash
skilltap --version
```

## Use skilltap through your agent

The easiest human workflow is to describe the result you want. Your Codex or
Claude agent can inspect the CLI, plan the work, and use skilltap on your
behalf. Try prompts such as:

> Use skilltap to check whether this computer's enabled harness environments
> are healthy, and distinguish verified from effective-unverified state.

> Use skilltap to adopt my existing Claude configuration and show what would
> change before syncing it to Codex.

> Use skilltap to sync my global plugins, skills, and shared instructions.
> Explain any drift or incompatibility and ask before accepting a partial
> result.

> Use skilltap to install `formatter@example-plugins` for this project in every
> selected target that can represent it safely.

Agents should use `skilltap --help` and leaf-command help for exact syntax,
start with `status` or `plan` when appropriate, and bring judgment calls back
to you. You can still run every command directly; there is no separate agent
mode.

## Inspect before configuring

`status` works before skilltap configuration exists and does not mutate the
machine:

```bash
skilltap status
```

When `config.toml` is absent, no harness is considered enabled. `status` may
report installed or file-only observe-only targets, but it does not infer
management policy, create the skilltap configuration directory, or write any
harness file.

## Enable native harnesses

Enable only the harnesses you want skilltap to manage:

```bash
skilltap harness list
skilltap harness enable codex
skilltap harness enable claude
# Other registered ids appear in `harness list` with their exact support tier.
```

Enabling an adapter records policy. It does not adopt or modify native
configuration.

## Adopt and synchronize

Adopt global resources from enabled harnesses, inspect the proposed changes,
then apply them:

```bash
skilltap adopt
skilltap plan
skilltap sync
```

Adoption changes skilltap inventory only. Synchronization performs the planned
native operations and verifies the result by observing the harnesses again.

For a project, add `--project`; to name another location, pass a path:

```bash
skilltap status --project
skilltap adopt --project ~/src/example
skilltap plan --project ~/src/example
```

Bare scoped commands operate globally. `--target <registered-id>` or
`--target all` independently selects the harnesses involved. Registration and
enablement do not imply mutation support; plans report component- and
scope-specific authority for the installed version.

Each resource has a stable logical ID and one concrete global or project scope.
Together they form its exact resource key. Equal logical IDs in global scope
and different projects are separate managed instances.

Next, see [Managing Environments](./managing-environments) and
[Shared Instructions](./instructions).
