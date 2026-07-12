---
description: Install skilltap and adopt an existing Codex or Claude environment.
---

# Getting Started

## Install

On macOS or Linux, install the standalone binary and run its first-party
bootstrap boundary:

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

The installer verifies the release checksum, installs the user-level binary,
and invokes `skilltap bootstrap` to detect Claude Code and Codex independently.
Harness absence or an unsupported native lifecycle is reported as attention;
the verified binary remains available. The executable is the authoritative
command discovery surface:

```console
skilltap bootstrap --help
```

If you installed the plugin through either native marketplace, use the same
command. Marketplace installation and the one-line installer are equivalent
first-party setup paths; neither path writes native harness caches.

Or use Homebrew:

```bash
brew install nklisch/skilltap/skilltap
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
