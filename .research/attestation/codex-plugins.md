---
source_handle: codex-plugins
fetched: 2026-07-10
source_url: https://learn.chatgpt.com/docs/plugins
provenance: source-direct
substrate_confidence: source-direct
---

# Plugins

## Summary

OpenAI documents plugins as installable bundles available across ChatGPT and
Codex surfaces. A bundle may provide skills, an MCP-backed app, or both, and the
broader component list also includes MCP servers, browser extensions, hooks,
and scheduled-task templates. Codex CLI exposes an interactive `/plugins`
browser organized by marketplace, with install, uninstall, enable, and disable
operations. New sessions are required before newly installed skills or tools
become available.

## Key passages

### Overview (lines 699-722)

> "Plugins bundle capabilities into reusable workflows in ChatGPT." (Overview,
> line 701)

- Plugins bundle reusable capabilities and can include skills and an
  MCP-backed app.
- Codex CLI and the IDE extension can browse and install plugins for a Codex
  environment.
- The documented component taxonomy includes skills, apps, MCP servers,
  browser extensions, hooks, and scheduled-task templates.
- Plugins can be shared through marketplace sources, including repository
  marketplaces for projects or teams.

### Codex CLI plugin directory (lines 767-780)

- `/plugins` opens the Codex CLI plugin browser.
- The browser groups entries by marketplace and supports inspecting,
  installing, uninstalling, enabling, and disabling plugins.
- The IDE extension likewise installs plugins for its selected Codex host.

### Activation and permissions (lines 781-789)

- Bundled skills become available in a new chat or CLI session after install.
- Plugin connectors or MCP servers may require additional setup or
  authentication.
- Capabilities running through a Codex host remain subject to that host's
  sandbox and approval policy.
