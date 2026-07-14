---
id: epic-expanded-harness-support-candidate-admission-zoo-boundary
kind: story
stage: implementing
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-gate]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zoocode-skills.md
  - .research/attestation/zoocode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Validate Zoo Code Boundaries

## Checkpoint

Resolve Zoo Code's highest-risk extension boundary before any production path or
adapter is added. Record the complete source-bound and isolated evidence table in
this story body and finish with exactly one disposition: `admitted`,
`observe_only`, or `blocked`.

Validation must establish:

- exact Zoo extension identity, installed version, and deterministic supported
  host/CLI command that can run through the bounded process port;
- every supported global and project skill root in the documented `.roo` and
  `.agents` families, mode-specific precedence, complete sibling access,
  executable intent, update visibility, and project-over-global collision;
- the stable global `mcp_settings.json` path on macOS and Linux and project
  `.roo/mcp.json`, both as supported direct-write files rather than editor
  databases or caches;
- exact `mcpServers` schema, stdio/HTTP/SSE, enablement/tool policy, scope
  precedence, preservation of unknown/unowned entries, and owned update/removal;
- a deterministic reload/effective server/tool observation after direct edits;
- isolated profile redirection proving the operator's real HOME/editor profile,
  credentials, extension storage, and caches stay untouched.

A candidate integration test at
`crates/harnesses/tests/candidate_zoo_boundary.rs` is created only if all native
roots and processes can be isolated safely. UI automation or screen scraping may
identify a lead but cannot satisfy a gate check.

## Acceptance evidence

- [ ] Every admission check has exact official source, fetched date, native
      version/output bytes, isolated action, observed result, and disconfirming
      result where relevant.
- [ ] No path is accepted solely because the settings UI opened it.
- [ ] Complete skill and MCP updates are effectively observed after the
      documented reload/restart behavior.
- [ ] Removal deletes only proven owned entries and preserves native/unmanaged
      siblings and unknown fields.
- [ ] Immediate repeats produce no file, identity, or effective-state change.
- [ ] The disposition names every missing check. Missing platform-independent
      global MCP or cache-independent effective observation prevents
      `admitted`.

## Ordering

Runs after the shared gate and before Zoo's admission checkpoint. It does not
edit the canonical registry or production adapter modules.
