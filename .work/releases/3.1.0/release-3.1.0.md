---
id: release-3.1.0
kind: release
stage: released
tags: []
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Release 3.1.0

## Summary

- Expand the typed target registry to seventeen harnesses with exact per-component support tiers: verified/native or managed, mixed, declaration-managed, and observe-only.
- Use target-agnostic managed plugin projection while preserving exact ownership, drift detection, rollback, and status semantics.
- Keep project skills canonical under `.agents/skills/<name>/` and project them through registry-derived links or managed destinations.
- Refresh registered marketplaces before daemon plugin and Git-backed skill updates.
- Preserve unknown-version zero-write behavior, daemon exclusion for declaration-managed operations, and bounded native/filesystem execution.

## Gate runs

- **Security** — 0 critical/high/medium findings. Two low-severity hardening findings were routed to the unbound backlog: bounded Git-root subprocess execution and confinement of remaining top-level filesystem writes.
- **Tests** — 2 high- and 2 medium-priority release findings, all fixed: exact declaration-managed daemon/acknowledgment coverage, native post-apply journal recovery, and exact acknowledgment validation.
- **Cruft** — 4 medium-confidence release findings, all fixed: shared managed skill planning, path observation, projection helpers, and removal of dead target-name plumbing. Two public-API removal decisions remain unbound backlog proposals.
- **Docs** — 2 high-confidence release findings, both fixed: the self-hosted skill now reflects the expanded registry, and the changelog carries `v3.1.0`.
- **Patterns** — codified `drift-checked-managed-projection-plan` and regenerated its index/digest. Two broader consistency refactors remain unbound for subsequent work.

## Verification

- `cargo test --workspace --all-targets` passes, including the release package, declaration-managed matrix, native recovery, and compiled-binary suites.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `cargo fmt --all -- --check` and `git diff --check` are clean.
- `cargo test -p skilltap --test plugin_package` passes all 4 package-channel tests.
- `npm run build` under `website/` builds all 8 pages and regenerates `llms-full.txt`.
- The compiled binary reports `skilltap 3.1.0`; Cargo packages and all tracked plugin/marketplace manifests agree on `3.1.0`.
- Durable isolated real-binary evidence covers 237 commands, 2,837 assertions, 203 single-document JSON commands, and 516 whitelisted fake-process invocations with no real network, auth, browser, editor, TUI, or harness mutation.

## Shipment

- **Date shipped:** 2026-07-15
- **Mapping:** tag-based (`v3.1.0`)
- **Items shipped:** 80
- **Gate findings:** 10 release-bound remediations completed; 6 non-blocking hardening, public-API decision, and broader consistency proposals remain unbound.
- **Publishing:** local annotated source tag only; no remote push or hosting release was performed.

## Shipped items

The full bodies live in Git history under the configured `delete-refs` retention policy.

| id | title | kind | archived_atop | git ref |
|----|-------|------|---------------|---------|
| `epic-expanded-harness-support` | Expanded Harness Support | epic | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission` | Cursor, Zoo Code, and ZCode Candidate Admission | feature | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained` | Configuration-Constrained Adapters for Kimi, Vibe, and Kilo | feature | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed` | Declaration-Managed Target Authority | feature | — | `fbd35191` |
| `epic-expanded-harness-support-file-managed` | File-Managed Adapters for Gemini, OpenCode, and Kiro | feature | — | `fbd35191` |
| `epic-expanded-harness-support-native-coexistence` | Native-Coexistence Adapters for Droid, Qwen, and Copilot | feature | — | `fbd35191` |
| `epic-expanded-harness-support-pi` | Pi Compound Target Adapter | feature | — | `fbd35191` |
| `epic-expanded-harness-support-project-skill-links` | Validate and Link Project-Local Skills | feature | — | `fbd35191` |
| `epic-expanded-harness-support-registry` | Typed Target Registry and Adapter Contract | feature | — | `fbd35191` |
| `epic-expanded-harness-support-trust-interactive` | Trust- and Interactive-State Adapters for Junie and Amp | feature | — | `fbd35191` |
| `feature-daemon-marketplace-refresh` | Refresh Marketplaces During Daemon Updates | feature | v3.0.3 | `54239f77` |
| `feature-managed-fallback-target-parity` | Complete Managed Fallback Target Parity | feature | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-acceptance` | Verify Candidate Dispositions and Isolation | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-cursor-admission` | Resolve Cursor Admission | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-cursor-boundary` | Validate Cursor Boundaries | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-gate` | Define Candidate Admission Authority and Gate | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-zcode-admission` | Resolve ZCode Admission | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-zcode-boundary` | Validate ZCode Boundaries | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-zoo-admission` | Resolve Zoo Code Admission | story | — | `fbd35191` |
| `epic-expanded-harness-support-candidate-admission-zoo-boundary` | Validate Zoo Code Boundaries | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-acceptance` | Prove Configuration-Constrained Adapter Acceptance | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-contract-lock` | Lock Kimi, Vibe, and Kilo Native Contracts | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-kilo` | Implement the Kilo Code Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-kimi` | Implement the Kimi Code Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-projection-scope` | Generalize and Gate Managed Projection | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-source` | Normalize Portable Source Components Privately | story | — | `fbd35191` |
| `epic-expanded-harness-support-configuration-constrained-vibe` | Implement the Mistral Vibe Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed-acceptance` | Prove Declaration-Managed Acceptance End to End | story | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed-authority-contract` | Define Exact-Profile Declaration Authority | story | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed-daemon-safety` | Keep Declaration-Managed Work Out of the Daemon | story | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed-execution-status` | Revalidate Declarations and Separate Effective Status | story | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed-migration-regressions` | Migrate Existing Profiles and Preserve Regressions | story | — | `fbd35191` |
| `epic-expanded-harness-support-declaration-managed-planner-acknowledgment` | Plan and Execute Exact Partial Acknowledgments | story | — | `fbd35191` |
| `epic-expanded-harness-support-file-managed-acceptance` | Prove Integrated File-Managed Adapter Acceptance | story | — | `fbd35191` |
| `epic-expanded-harness-support-file-managed-contracts` | Establish Scope-Aware File-Managed Adapter Contracts | story | — | `fbd35191` |
| `epic-expanded-harness-support-file-managed-gemini` | Implement the Gemini CLI Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-file-managed-kiro` | Implement the Kiro CLI Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-file-managed-opencode` | Implement the OpenCode Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-native-coexistence-acceptance` | Prove Integrated Native-Managed Coexistence | story | — | `fbd35191` |
| `epic-expanded-harness-support-native-coexistence-contract` | Route Native and Managed Lifecycle by Evidence | story | — | `fbd35191` |
| `epic-expanded-harness-support-native-coexistence-copilot` | Implement the GitHub Copilot CLI Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-native-coexistence-factory` | Implement the Factory Droid Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-native-coexistence-qwen` | Implement the Qwen Code Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-pi-acceptance` | Verify Pi Conditional-Target Acceptance | story | — | `fbd35191` |
| `epic-expanded-harness-support-pi-adapter` | Implement Pi Core and Companion Observation | story | — | `fbd35191` |
| `epic-expanded-harness-support-pi-integration` | Integrate Pi Status and Mutation Authorization | story | — | `fbd35191` |
| `epic-expanded-harness-support-pi-profile` | Establish Conditional Compound-Profile Contracts | story | — | `fbd35191` |
| `epic-expanded-harness-support-project-skill-links-acceptance` | Prove the Project Skill Link Lifecycle | story | — | `fbd35191` |
| `epic-expanded-harness-support-project-skill-links-contract` | Define Project Skill Validation and Layout Contracts | story | — | `fbd35191` |
| `epic-expanded-harness-support-project-skill-links-filesystem` | Add a Confined Project Symlink Boundary | story | — | `fbd35191` |
| `epic-expanded-harness-support-project-skill-links-lifecycle` | Reconcile Canonical Project Skills and Target Links | story | — | `fbd35191` |
| `epic-expanded-harness-support-project-skill-links-observation` | Observe and Report Project Skill Health | story | — | `fbd35191` |
| `epic-expanded-harness-support-registry-adapters` | Codex and Claude Adapter Migration | story | — | `fbd35191` |
| `epic-expanded-harness-support-registry-cli` | CLI Parser, Help, and Composition Dispatch | story | — | `fbd35191` |
| `epic-expanded-harness-support-registry-config` | Registry-Driven Configuration Map | story | — | `fbd35191` |
| `epic-expanded-harness-support-registry-contract` | Target Registry and Adapter Contract | story | — | `fbd35191` |
| `epic-expanded-harness-support-registry-test-support` | Reusable Adapter Acceptance Contract | story | — | `fbd35191` |
| `epic-expanded-harness-support-trust-interactive-acceptance` | Verify Junie and Amp Integration | story | — | `fbd35191` |
| `epic-expanded-harness-support-trust-interactive-amp` | Implement the Amp Adapter | story | — | `fbd35191` |
| `epic-expanded-harness-support-trust-interactive-contract-lock` | Lock Junie and Amp Native Contracts | story | — | `fbd35191` |
| `epic-expanded-harness-support-trust-interactive-junie` | Implement the Junie Adapter | story | — | `fbd35191` |
| `feature-daemon-marketplace-refresh-acceptance` | Verify Daemon Marketplace Refresh End to End | story | — | `fbd35191` |
| `feature-daemon-marketplace-refresh-execution` | Execute Marketplace Refresh and Plugin Updates as One Plan | story | — | `fbd35191` |
| `feature-daemon-marketplace-refresh-task-graph` | Build the Daemon Native-Update Task Graph | story | — | `fbd35191` |
| `feature-managed-fallback-target-parity-acceptance` | Shared Managed-Projection Acceptance Matrix | story | — | `fbd35191` |
| `feature-managed-fallback-target-parity-codex-adapter` | Codex Managed-Projection Adapter | story | — | `fbd35191` |
| `feature-managed-fallback-target-parity-contract` | Managed Projection Port Contract and Pure Types | story | — | `fbd35191` |
| `feature-managed-fallback-target-parity-contract-evidence` | Managed Projection Contract Evidence Amendment | story | — | `fbd35191` |
| `feature-managed-fallback-target-parity-orchestrator` | Target-Agnostic Managed-Project Orchestrator | story | — | `fbd35191` |
| `gate-cruft-remove-common-target-name-plumbing` | Remove dead target-name plumbing from shared projection planning | story | — | `556b16a9` |
| `gate-cruft-share-adapter-path-existence` | Share adapter path-existence helpers | story | — | `556b16a9` |
| `gate-cruft-share-projection-helpers` | Share trust-interactive projection helpers | story | — | `556b16a9` |
| `gate-cruft-unify-file-managed-skill-planning` | Unify duplicated file-managed skill planning | story | — | `556b16a9` |
| `gate-docs-changelog-3-1-0` | Add the 3.1.0 changelog entry | story | — | `556b16a9` |
| `gate-docs-self-hosted-skill-registry` | Roll the self-hosted skill forward to the expanded registry | story | — | `556b16a9` |
| `gate-patterns-3-1-0` | Patterns extracted for 3.1.0 | story | — | `556b16a9` |
| `gate-tests-declaration-acceptance-real-profiles` | Exercise declaration acceptance against real unverified profiles | story | — | `556b16a9` |
| `gate-tests-declaration-daemon-skip-matrix` | Byte-verify declaration-managed daemon skips across every target | story | — | `556b16a9` |
| `gate-tests-execution-acknowledgment-exact-match` | Cover exact execution-acknowledgment validation | story | — | `556b16a9` |
| `gate-tests-native-journal-after-apply-recovery` | Cover native journal-after-apply inventory recovery | story | — | `556b16a9` |
