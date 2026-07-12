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
  - title: One machine-wide model
    details: Bring existing resources together, keep enabled harnesses aligned, and see drift from one place.
---

## Install and connect your harnesses

Install the binary, or add the plugin through the harness you already use.

```bash
# One-line installer
curl -fsSL https://skilltap.dev/install.sh | sh

# Or Homebrew
brew install nklisch/skilltap/skilltap
```

```bash
# Claude Code
claude plugin marketplace add https://github.com/nklisch/skilltap/tree/main/plugin --scope user
claude plugin install skilltap@skilltap --scope user

# Codex marketplace
codex plugin marketplace add https://github.com/nklisch/skilltap.git --ref main --sparse plugin
```

Then let skilltap check the setup:

```bash
skilltap bootstrap
```

Codex may require its interactive `/plugins` flow to finish installing or
enabling the plugin. The bootstrap result explains the next step for the
installed harness versions.
