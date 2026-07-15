---
id: release-3.1.0
kind: release
stage: quality-gate
tags: []
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Release 3.1.0

## Bound items

80 completed non-release items: the original 69-item bundle plus 11 late-bound, completed gate items. The original bundle contains 68 active items and one archived feature (`feature-daemon-marketplace-refresh`, completed atop `v3.0.3`).

| id | kind | source | stage | archived_atop |
|----|------|--------|-------|---------------|
| epic-expanded-harness-support | epic | active | done | — |
| epic-expanded-harness-support-candidate-admission | feature | active | done | — |
| epic-expanded-harness-support-configuration-constrained | feature | active | done | — |
| epic-expanded-harness-support-declaration-managed | feature | active | done | — |
| epic-expanded-harness-support-file-managed | feature | active | done | — |
| epic-expanded-harness-support-native-coexistence | feature | active | done | — |
| epic-expanded-harness-support-pi | feature | active | done | — |
| epic-expanded-harness-support-project-skill-links | feature | active | done | — |
| epic-expanded-harness-support-registry | feature | active | done | — |
| epic-expanded-harness-support-trust-interactive | feature | active | done | — |
| feature-managed-fallback-target-parity | feature | active | done | — |
| epic-expanded-harness-support-candidate-admission-acceptance | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-cursor-admission | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-cursor-boundary | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-gate | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-zcode-admission | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-zcode-boundary | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-zoo-admission | story | active | done | — |
| epic-expanded-harness-support-candidate-admission-zoo-boundary | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-acceptance | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-contract-lock | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-kilo | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-kimi | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-projection-scope | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-source | story | active | done | — |
| epic-expanded-harness-support-configuration-constrained-vibe | story | active | done | — |
| epic-expanded-harness-support-declaration-managed-acceptance | story | active | done | — |
| epic-expanded-harness-support-declaration-managed-authority-contract | story | active | done | — |
| epic-expanded-harness-support-declaration-managed-daemon-safety | story | active | done | — |
| epic-expanded-harness-support-declaration-managed-execution-status | story | active | done | — |
| epic-expanded-harness-support-declaration-managed-migration-regressions | story | active | done | — |
| epic-expanded-harness-support-declaration-managed-planner-acknowledgment | story | active | done | — |
| epic-expanded-harness-support-file-managed-acceptance | story | active | done | — |
| epic-expanded-harness-support-file-managed-contracts | story | active | done | — |
| epic-expanded-harness-support-file-managed-gemini | story | active | done | — |
| epic-expanded-harness-support-file-managed-kiro | story | active | done | — |
| epic-expanded-harness-support-file-managed-opencode | story | active | done | — |
| epic-expanded-harness-support-native-coexistence-acceptance | story | active | done | — |
| epic-expanded-harness-support-native-coexistence-contract | story | active | done | — |
| epic-expanded-harness-support-native-coexistence-copilot | story | active | done | — |
| epic-expanded-harness-support-native-coexistence-factory | story | active | done | — |
| epic-expanded-harness-support-native-coexistence-qwen | story | active | done | — |
| epic-expanded-harness-support-pi-acceptance | story | active | done | — |
| epic-expanded-harness-support-pi-adapter | story | active | done | — |
| epic-expanded-harness-support-pi-integration | story | active | done | — |
| epic-expanded-harness-support-pi-profile | story | active | done | — |
| epic-expanded-harness-support-project-skill-links-acceptance | story | active | done | — |
| epic-expanded-harness-support-project-skill-links-contract | story | active | done | — |
| epic-expanded-harness-support-project-skill-links-filesystem | story | active | done | — |
| epic-expanded-harness-support-project-skill-links-lifecycle | story | active | done | — |
| epic-expanded-harness-support-project-skill-links-observation | story | active | done | — |
| epic-expanded-harness-support-registry-adapters | story | active | done | — |
| epic-expanded-harness-support-registry-cli | story | active | done | — |
| epic-expanded-harness-support-registry-config | story | active | done | — |
| epic-expanded-harness-support-registry-contract | story | active | done | — |
| epic-expanded-harness-support-registry-test-support | story | active | done | — |
| epic-expanded-harness-support-trust-interactive-acceptance | story | active | done | — |
| epic-expanded-harness-support-trust-interactive-amp | story | active | done | — |
| epic-expanded-harness-support-trust-interactive-contract-lock | story | active | done | — |
| epic-expanded-harness-support-trust-interactive-junie | story | active | done | — |
| feature-daemon-marketplace-refresh-acceptance | story | active | done | — |
| feature-daemon-marketplace-refresh-execution | story | active | done | — |
| feature-daemon-marketplace-refresh-task-graph | story | active | done | — |
| feature-managed-fallback-target-parity-acceptance | story | active | done | — |
| feature-managed-fallback-target-parity-codex-adapter | story | active | done | — |
| feature-managed-fallback-target-parity-contract | story | active | done | — |
| feature-managed-fallback-target-parity-contract-evidence | story | active | done | — |
| feature-managed-fallback-target-parity-orchestrator | story | active | done | — |
| feature-daemon-marketplace-refresh | feature | archive | done | v3.0.3 |

### Late-bound gate items

| id | gate | stage |
|----|------|-------|
| gate-tests-declaration-daemon-skip-matrix | tests | done |
| gate-tests-execution-acknowledgment-exact-match | tests | done |
| gate-tests-native-journal-after-apply-recovery | tests | done |
| gate-tests-declaration-acceptance-real-profiles | tests | done |
| gate-cruft-unify-file-managed-skill-planning | cruft | done |
| gate-cruft-share-adapter-path-existence | cruft | done |
| gate-cruft-share-projection-helpers | cruft | done |
| gate-cruft-remove-common-target-name-plumbing | cruft | done |
| gate-docs-self-hosted-skill-registry | docs | done |
| gate-docs-changelog-3-1-0 | docs | done |
| gate-patterns-3-1-0 | patterns | done |

## Gate runs

- **Security** — 0 critical/high/medium findings. Two low-severity hardening findings were routed to the unbound backlog: bound Git-root subprocess execution and confinement of remaining top-level filesystem writes.
- **Tests** — 2 high- and 2 medium-priority release findings. Added exact declaration-managed daemon/acknowledgment coverage, post-apply native journal recovery coverage, and exact acknowledgment validation; all four items are done.
- **Cruft** — 4 medium-confidence release findings. Shared managed skill planning, path observation, and projection helpers were consolidated; dead target-name plumbing was removed. Two public-API removal decisions remain unbound backlog proposals.
- **Docs** — 2 high-confidence release findings. The self-hosted skill now describes the expanded registry, and this release has a changelog entry.
- **Patterns** — codified `drift-checked-managed-projection-plan`; regenerated the index/digest. Two broader consistency refactors remain unbound for a subsequent release.

## Release artifacts

- Workspace package versions: `3.1.0`.
- Changelog: `v3.1.0` entry present.
- Shipping mapping: tag-based as `v3.1.0`.
