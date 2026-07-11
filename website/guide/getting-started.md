---
description: Install skilltap and adopt an existing Codex or Claude environment.
---

# Getting Started

## Install

On macOS or Linux, install the standalone binary:

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

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

Next, see [Managing Environments](./managing-environments) and
[Shared Instructions](./instructions).
