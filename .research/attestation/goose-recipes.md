---
source_handle: goose-recipes
fetched: 2026-07-12
source_url: https://goose-docs.ai/docs/guides/recipes/recipe-reference/
provenance: source-direct
---

# Goose recipe-scoped MCP

Goose recipes can bundle MCP extension configurations and may be stored in a
project. A recipe is an explicitly launched workflow rather than an ambient
project configuration: its prompt/instructions and selected extensions apply
when the recipe is run. This is not equivalent to automatically loading an MCP
server for ordinary Goose sessions opened in the project.

## Key passages

- The recipe schema includes an `extensions` array for MCP servers.
- Extension types include stdio and streamable HTTP.
- The recipe storage guide distinguishes global and project-local recipes.
- Recipe usage is explicit through `goose run --recipe ...` or a recipe session.
