---
layout: home

hero:
  name: skilltap
  text: Keep your agent tools in step
  tagline: Give agents one safe way to manage plugins, skills, MCP declarations, and shared instructions across your local agent harnesses.
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: CLI reference
      link: /reference/cli

features:
  - title: Native first
    details: Works with each harness's own marketplace and plugin lifecycle whenever possible.
  - title: Agent forward
    details: Clear commands, useful plans, and optional JSON output make it easy for people and agents to work together.
  - title: Precise support
    details: Each target reports verified, declaration-managed, observe-only, and unsupported components without overstating what loaded.
  - title: One machine-wide model
    details: Bring existing resources together, keep enabled harnesses aligned, and see drift from one place.
---

## Install the plugin first

Add the marketplace, then install or enable the plugin in the harness you
already use.

```bash
# Claude Code
claude plugin marketplace add nklisch/skilltap --scope user
claude plugin install skilltap@skilltap --scope user

# Codex marketplace
codex plugin marketplace add nklisch/skilltap
codex plugin add skilltap@skilltap
```

Need the standalone binary directly, or setting up a harness without its
plugin flow? Use the online installer after registering the marketplace:

```bash
# One-line installer
curl -fsSL https://skilltap.dev/install.sh | sh

# Or Homebrew
brew install nklisch/skilltap/skilltap
```

Then let skilltap check the setup:

```bash
skilltap bootstrap
```

## Tell your agent what you want

You do not need to memorize the CLI. Once the plugin is installed, describe
the outcome to your Codex or Claude agent and let it use skilltap for you:

> Use skilltap to check whether my enabled agent harnesses are healthy.

> Use skilltap to sync my global plugins and shared instructions across the
> targets that support them. Show me verified and declaration-managed work first.

> Use skilltap to install `formatter@example-plugins` in this project. If
> anything cannot carry over faithfully, explain it and ask before proceeding.

> Use skilltap to adopt my current Claude setup, then tell me what can move to
> my other enabled targets and which results would remain effective-unverified.

The agent can discover exact commands through `skilltap --help`, inspect with
`status` and `plan`, and bring incompatibilities, drift, or partial operations
back to you for a decision.

Older Codex builds without `plugin add` can finish installation through
`/plugins`. The bootstrap result explains the next step for the installed
harness versions.
