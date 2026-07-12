---
layout: home

hero:
  name: skilltap
  text: Keep your agent tools in step
  tagline: Give agents one friendly way to help you manage Codex and Claude Code plugins, skills, marketplaces, and shared instructions.
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
  - title: Bring plugins with you
    details: Use compatible pieces from a Claude or Codex marketplace in the other harness, with clear warnings when behavior cannot carry over.
  - title: One machine-wide model
    details: Bring existing resources together, keep enabled harnesses aligned, and see drift from one place.
---

## Install the plugin first

Add the marketplace, then install or enable the plugin in the harness you
already use.

```bash
# Claude Code
claude plugin marketplace add https://github.com/nklisch/skilltap/tree/main/plugin --scope user
claude plugin install skilltap@skilltap --scope user

# Codex marketplace
codex plugin marketplace add https://github.com/nklisch/skilltap.git --ref main --sparse plugin
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

Codex may require its interactive `/plugins` flow to finish installing or
enabling the plugin. The bootstrap result explains the next step for the
installed harness versions.
