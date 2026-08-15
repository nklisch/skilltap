---
id: gate-tests-agy-plan-branches
kind: story
stage: done
tags: [testing]
parent: feature-replace-gemini-with-agy
depends_on: []
release_binding: 4.0.0
gate_origin: tests
created: 2026-08-15
updated: 2026-08-15
---

# Cover AGY drift and unsupported MCP plan branches

Add stable compiled-binary evidence that AGY refuses to overwrite a modified
owned MCP entry. Also prove that an unmappable optional MCP server is omitted
without emitting unsupported native fields.

The tests exercise the real AGY adapter, preserve the drifted native document,
and constrain every fake invocation to `agy --version`. Required MCP behavior
remains covered by the shared component contract because current supported
source readers classify MCP declarations as optional.

## Verification

Both focused compiled-binary tests pass. The exact-profile test now covers
owned MCP drift, and the optional-omission test covers plan-level codec refusal.
