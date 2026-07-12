---
description: Install skilltap and adopt an existing Codex or Claude environment.
---

# Getting Started

## Install

The plugin is skilltap's primary distribution surface. Add the marketplace,
then install or enable the plugin in the harness you already use:

```bash
# Claude Code
claude plugin marketplace add https://github.com/nklisch/skilltap/tree/main/plugin --scope user
claude plugin install skilltap@skilltap --scope user

# Codex (marketplace registration)
codex plugin marketplace add https://github.com/nklisch/skilltap.git --ref main --sparse plugin
```

Claude can install the plugin natively after registration. Codex can expose the
complete skill from its marketplace; open `/plugins` in Codex to install or
enable the plugin when that native flow is available.

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

## Inspect before configuring

`status` works before skilltap configuration exists and does not mutate the
machine:

```bash
skilltap status
```

When `config.toml` is absent, neither Codex nor Claude is considered enabled.
`status` may report what is installed, but it does not infer management policy,
create the skilltap configuration directory, or write any harness file.

## Enable native harnesses

Enable only the harnesses you want skilltap to manage:

```bash
skilltap harness enable codex
skilltap harness enable claude
skilltap harness list
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

Bare scoped commands operate globally. `--target codex`, `--target claude`, or
`--target all` independently selects the harnesses involved.

Each resource has a stable logical ID and one concrete global or project scope.
Together they form its exact resource key. Equal logical IDs in global scope
and different projects are separate managed instances.

Next, see [Managing Environments](./managing-environments) and
[Shared Instructions](./instructions).
