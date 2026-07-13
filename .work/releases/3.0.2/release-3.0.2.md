---
id: release-3.0.2
kind: release
stage: released
tags: []
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Release 3.0.2

## Bound items

- `epic-skilltap-plugin-distribution` and its 39 completed descendants —
  self-hosted Claude/Codex plugin package, verified binary bootstrap, CLI help
  contract, primary plugin installation story, website parity, and release
  automation. These completed items were late-bound from the prior workflow.
- `epic-real-harness-recovery-and-adapter-expansion` — complete native
  lifecycle, state recovery, instruction repair, bootstrap, diagnostics, and
  adapter-eligibility research bundle, including 23 completed descendants that
  were late-bound with their parent.
- `story-agent-directed-usage-docs` — agent-directed human usage guidance.
- 11 release-gate remediation stories — confined and bounded project I/O,
  exact recovery/publication evidence, unsupported-plugin blocking, native
  retry and instruction repair coverage, rollback residual reporting, and
  bounded cleanup.
- `gate-patterns-3.0.2` — four recurring implementation patterns codified.

Total non-release items: **77** (2 epics, 11 features, 64 stories). There were
no unbound archived stubs.

## Gate runs

- **gate-security** (2026-07-12) — 2 findings (1 high, 1 medium), both fixed and reviewed.
- **gate-tests** (2026-07-12) — 6 gaps (3 critical, 2 high, 1 medium), all fixed and reviewed.
- **gate-cruft** (2026-07-12) — 2 medium findings, both removed and reviewed.
- **gate-docs** (2026-07-12) — 8 findings: six corrected; two future-facing
  product promises retained and scoped as implementation features.
- **gate-patterns** (2026-07-12) — 4 patterns codified; 1 rollback adoption
  defect fixed and reviewed.

### Binding-consistency warnings

BINDING CONSISTENCY — release 3.0.2 (`epic_cohesion: phased`):

- INCOMPLETE — `feature-relaxed-target-harness-research` remains unbound under
  the bound recovery epic. This is informational: it is a research input,
  research items never bind to releases, and phased epic delivery permits the
  unbound child.

No cross-version conflicts were found.

## Verification

- Full workspace tests and all-target/all-feature Clippy are green.
- Website build, release contract, installer, install-surface, plugin package,
  and cutover checks are green.
- Security, tests, cruft, docs, and patterns release gates run before tagging.

## Shipment

- **Date shipped:** 2026-07-12
- **Mapping:** tag-based
- **Items shipped:** 77
- **Gate findings:** 19 findings or coverage gaps fixed/reconciled, plus 4
  implementation patterns codified.
- **Publishing:** signed source tag and GitHub Actions release artifacts,
  website deployment, and Homebrew formula handoff.

## Shipped items

Full bodies live in Git history under the configured `delete-refs` retention
policy. Recover one with `git show <git-ref>:<former-path>`.

| id | title | kind | archived_atop | git ref |
|----|-------|------|---------------|---------|
| `epic-skilltap-plugin-distribution` | Skilltap Self-Hosted Plugin Distribution | epic | — | `210e607d` |
| `epic-real-harness-recovery-bootstrap-transport` | Repair bootstrap release transport | feature | — | `210e607d` |
| `epic-real-harness-recovery-filesystem-instructions` | Preserve skill executability and correct instruction bridges | feature | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle` | Align native lifecycle adapters with current harness contracts | feature | — | `210e607d` |
| `epic-real-harness-recovery-runtime-boundary` | Detect and isolate real harness processes | feature | — | `210e607d` |
| `epic-real-harness-recovery-state-diagnostics` | Make lifecycle state and diagnostics target-exact | feature | — | `210e607d` |
| `epic-skilltap-plugin-distribution-bootstrap` | Verified Skilltap Binary Bootstrap | feature | — | `210e607d` |
| `epic-skilltap-plugin-distribution-cli-contract` | Agent-Readable CLI Help and Errors | feature | — | `210e607d` |
| `epic-skilltap-plugin-distribution-cutover` | Retire the Legacy Skilltap Skills Publisher | feature | — | `210e607d` |
| `epic-skilltap-plugin-distribution-guidance` | Skilltap Agent Guidance | feature | — | `210e607d` |
| `epic-skilltap-plugin-distribution-package` | Canonical Plugin Package and Channel Metadata | feature | — | `210e607d` |
| `epic-skilltap-plugin-distribution-release` | Versioned Plugin and Binary Release | feature | — | `210e607d` |
| `epic-real-harness-recovery-filesystem-instructions-executable-intent` | Preserve normalized executable intent through skill publication | story | — | `210e607d` |
| `epic-real-harness-recovery-filesystem-instructions-relative-bridges` | Compute and validate canonical instruction bridges | story | — | `210e607d` |
| `epic-real-harness-recovery-filesystem-instructions-repair-completion` | Complete instruction repair postconditions and sync results | story | — | `210e607d` |
| `epic-real-harness-recovery-filesystem-instructions-repair-outcome` | Complete successful acknowledged instruction repairs | story | — | `210e607d` |
| `epic-real-harness-recovery-filesystem-instructions-umask-independent-modes` | Publish exact managed modes independently of umask | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-contracts` | Attest exact native profiles and command vectors | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery` | Recover the exact managed Pending journal shape | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-managed-project-load-contract` | Complete the managed Codex project load contract | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest` | Reconcile managed project projections from an installed component manifest | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-managed-project` | Materialize unsupported Codex project lifecycle safely | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-postcondition-retry-safety` | Make failed postcondition retries observation-safe | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-postconditions` | Verify lifecycle postconditions with actionable diagnostics | story | — | `210e607d` |
| `epic-real-harness-recovery-native-lifecycle-scope-aware-presence` | Match native resource presence by concrete scope | story | — | `210e607d` |
| `epic-real-harness-recovery-runtime-boundary-diagnostics-completion` | Complete typed diagnostics across lifecycle surfaces | story | — | `210e607d` |
| `epic-real-harness-recovery-runtime-boundary-diagnostics` | Project actionable detection diagnostics | story | — | `210e607d` |
| `epic-real-harness-recovery-runtime-boundary-process-context` | Build the explicit native process context | story | — | `210e607d` |
| `epic-real-harness-recovery-runtime-boundary-version-decoding` | Decode real Codex and Claude versions | story | — | `210e607d` |
| `epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle` | Reconcile dual-native lifecycle without losing siblings | story | — | `210e607d` |
| `epic-real-harness-recovery-state-diagnostics-output-contract` | Align help and diagnostic aggregation | story | — | `210e607d` |
| `epic-real-harness-recovery-state-diagnostics-output-test-parity` | Align postcondition tests with canonical recovery actions | story | — | `210e607d` |
| `epic-real-harness-recovery-state-diagnostics-target-evidence` | Persist lifecycle evidence per target | story | — | `210e607d` |
| `epic-real-harness-recovery-state-diagnostics-update-eligibility` | Count only actionable available updates | story | — | `210e607d` |
| `gate-cruft-discard-backup-handle` | Discard unused backup handle directly | story | — | `210e607d` |
| `gate-cruft-unused-catalog-mutation-api` | Remove abandoned catalog mutation API | story | — | `210e607d` |
| `gate-patterns-3.0.2` | Patterns extracted for 3.0.2 | story | — | `210e607d` |
| `gate-security-bounded-project-observation` | Bound hostile managed project observation | story | — | `210e607d` |
| `gate-security-project-ancestor-symlink-escape` | Prevent managed project writes through symlink ancestors | story | — | `210e607d` |
| `gate-tests-custom-home-repair-result` | Require completed output after successful custom-home repair | story | — | `210e607d` |
| `gate-tests-descriptor-mode-replacement` | Prove mode changes cannot follow a replaced destination | story | — | `210e607d` |
| `gate-tests-managed-project-publication-failures` | Exercise every managed project publication failure boundary | story | — | `210e607d` |
| `gate-tests-managed-terminal-journal-recovery` | Exercise terminal managed journal failure through lifecycle retry | story | — | `210e607d` |
| `gate-tests-managed-unsupported-only-plugin` | Keep unsupported-only managed plugins blocked with acknowledgment | story | — | `210e607d` |
| `gate-tests-remove-opposite-state-retry` | Cover remove retry when recovered observation remains present | story | — | `210e607d` |
| `story-agent-directed-usage-docs` | Show humans how to delegate skilltap workflows to agents | story | — | `210e607d` |
| `story-fix-managed-skill-rollback-residuals` | Report managed skill rollback residuals | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening` | Close bootstrap artifact redirect and publication races | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-artifact-portable-rollback-safety` | Make artifact publication and rollback safe on every supported platform | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-artifacts` | Bounded release transport and binary installation | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-cli-rollback-race-coverage` | Complete deterministic CLI rollback race coverage | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety` | Make CLI bootstrap rollback identity-safe | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-command-coverage` | Restore isolated bootstrap command acceptance coverage | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-command` | First-class bootstrap command and result contract | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-contract` | Bootstrap release and update policy contract | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-daemon-binary-policy` | Apply the bootstrap binary policy in the optional daemon | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-daemon-target-lock` | Make daemon binary updates target and lock the running installation | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-harness-contract-coverage` | Complete first-party harness bootstrap contract coverage | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-harness` | Harness detection and first-party plugin setup | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-bootstrap-installer` | Online installer and plugin bootstrap parity | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-cli-diagnostics` | Boundary-aware, secret-safe diagnostics | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-cli-help-contract` | Complete executable help contract | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-cli-verification` | Compiled CLI contract and reference parity | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-cutover-canonical-verification` | Verify canonical plugin and binary cutover evidence | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-cutover-legacy-record` | Record legacy skilltap retirement and archive handoff | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-cutover-sibling-parity` | Check active sibling marketplace parity without mutation | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-guidance-core` | Author the portable skilltap activation skill | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-guidance-diagnostics` | Document diagnostic, update, and recovery decisions | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-guidance-layout` | Document skilltap configuration and instruction bridges | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-guidance-validation` | Validate the complete guidance artifact | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-release-contract` | Enforce release identity and artifact contracts | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-release-install-surfaces` | Align website, Homebrew, and secondary marketplace installation surfaces | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-release-installer` | Align one-line installation with bootstrap | story | — | `210e607d` |
| `story-skilltap-plugin-distribution-release-verification` | Gate and verify the versioned release publication | story | — | `210e607d` |
| `story-skilltap-plugin-package-assets` | Establish canonical plugin publication assets | story | — | `210e607d` |
| `story-skilltap-plugin-package-validation` | Validate the canonical plugin package boundary | story | — | `210e607d` |
| `epic-real-harness-recovery-and-adapter-expansion` | Restore real harness operation and expand adapter eligibility | epic | — | `210e607d` |
