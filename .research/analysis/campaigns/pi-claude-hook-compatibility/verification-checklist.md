---
provenance: agent-synthesis
role: adversarial-reader
campaign: pi-claude-hook-compatibility
updated: 2026-07-12
---

# Adversarial-reader verification checklist — `pi-claude-hook-compatibility` campaign parent synthesis

## Verdict: `APPROVED`

The parent synthesis (`parent.md`) is source-bound, internally consistent, and supports its observe-only decision. No material blocker was found during jobs a–h. Concrete observations are recorded below; none rise to a revision requirement.

---

## Job findings

### (a) Semantic citation-chain walk

Walked each load-bearing claim back to its cited attestation and confirmed semantic support:

- **Decision/verdict claims** — that `@hsingjui/pi-hooks` does not satisfy skilltap's hook-equivalence contract and must not grant mutation authority — are supported by `pi-hooks-source` (nine-event subset, `command`-only, divergent payload/blocking/timing/timeout/matcher semantics) and `claude-hooks-reference` (full ~30-event contract, faithful field semantics). ✓
- **Identity claims** — exact package `@hsingjui/pi-hooks@0.0.2`, Pi extension entrypoint, peer-dependency migration, npm as version authority — are supported by `pi-hooks-identity-npm`, `pi-hooks-identity-installed`, and `pi-hooks-identity-github`. ✓
- **Health/ownership claims** — `pi list` reports presence/path only, absence of `hooks` key renders the extension inert, disjoint config surfaces with `pi-mcp-adapter` — are supported by `pi-hooks-health-pi-runtime`, `pi-hooks-health-installed-package`, and `pi-hooks-health-mcp-adapter-installed`. ✓
- **MCP interaction claim** — that a configured `PreToolUse` matcher for `mcp__.*` can observe/deny/rewrite MCP-adapter tools — is supported by `pi-hooks-source` (README/tool-name matcher list). ✓

No load-bearing claim was found to lack attestation support or to be supported only by an analytical-tier lens.

### (b) Claim-shapes the mechanical lint missed

- No plausible-looking attributions are missing citations.
- No cite-throughs are over-extended beyond the attested source.
- No comparative superlatives are framed as source-attested facts.
- **Observation only:** the "Required Pi adapter evidence" list (items 1–7) is prescriptive/contractual rather than descriptive. Individual items that rest on attested facts (e.g., item 5's enablement/observation gap, grounded in `pi-hooks-health-pi-runtime`) carry citations; the remaining items are internal acceptance criteria and do not require external citation.

### (c) Coherence-read for smoothed contradictions

No smoothing detected. The parent preserves contradictions as explicit side-by-side positions with typed relationships:

- `qualifies`: package self-description vs. semantic contract.
- `contradicts`: npm release version vs. repository manifest version.
- `incommensurable`: Pi package-scope precedence vs. hook-group concatenation.
- `tension`: installed package presence vs. inert effective health.

Each contradiction names its handles and avoids resolution-by-paraphrase.

### (d) Noise/relevance weighting

Citation relevance is sound across the artifact:

- Semantic equivalence claims cite the semantic triad (`pi-hooks-source`, `pi-extension-events`, `claude-hooks-reference`).
- Package-identity claims cite the identity attestations.
- Health/ownership/observation claims cite the health attestations.

No case was found where a less-relevant attestation was cited while a more-relevant one went unused.

### (e) Quote-context walk (`GR.4`)

The parent paraphrases rather than relying on extensive verbatim quotes. The few quoted/retained phrases — e.g., the package description "Claude Code-compatible command hooks for the Pi coding agent" — are accurately attributed as the publisher's self-description, and the parent immediately qualifies it as best-effort/command-only. No source qualifier was stripped to sharpen a claim.

### (f) Analytical-tier-inheritance walk

- The parent explicitly states: "Specialist analyses were used as composition lenses and are not citation targets."
- No specialist brief is cited as `[handle]{N}`.
- Composed claims are marked with the closed epistemic-status vocabulary (`{inferred: ...}` and `{extends}`), distinguishing source-attested from composed propositions.
- The source map resolves every `{N}` to a source-direct attestation in `.research/attestation/`, not to an analytical-tier artifact.

### (g) Line/range-reference walk

The parent uses handle-level citations (`[handle]{N}`) throughout. It does not assert specific line, section, or paragraph ranges, so no range-existence check is required. No false line references were found.

### (h) Thin-attestation check (`GR.5`, semantic complement)

All 13 cited attestations in the parent source map are substantively thick:

- `pi-hooks-source`: full source-tree walk with file-level mechanisms, quotes, and structural metadata.
- `claude-hooks-reference`: full reference body with per-event input/output/exit-code/timeout facts.
- `pi-extension-events`: installed SDK doc read with event-by-event semantics.
- Identity and health attestations: multiple anchored facts, source-internal values, and structural metadata.

No attestation is a token heading or a single blockquote paraphrasing at whole-source granularity.

---

## Lint-output assessment

Ran `lint-citations.py` (from the agentic-research plugin) against `parent.md` and all three specialist briefs. The lint emitted `[low] unreachable-source` notices for:

- `claude-hooks-reference`, `pi-hooks-identity-pi-docs`, `pi-hooks-identity-alternatives` (in `parent.md` and `specialists/package-identity.md` / `specialists/hook-semantics.md`).
- `pi-packages`, `pi-settings`, `pi-mcp-adapter` (in `specialists/health-ownership.md`).

**Assessment:** These are low-severity network-reachability notices. Every flagged handle resolves to a genuinely source-direct attestation in `.research/attestation/` with `fetched: 2026-07-12` and either `source_url` or `source_path`. Per the task instruction, low network-reachability lint notices are treated as findings **only if** the attestation is not genuinely source-direct; here they are source-direct, so they are not recorded as revision findings. No semantic gaps were dismissed.

The lint also emitted `[warn]` pattern flags (`version-number`, `comparative-superlative`) on version mentions and the word "unique." These are mechanical heuristics, not fabrication findings. The version mentions are factual (`0.0.2`, `0.80.6`, `1.5s`, `60s`, `v2.1.205`); the "unique" claim is supported by the keyword-intersection search in `pi-hooks-identity-alternatives`. No correction is needed.

---

## Coverage verification

| Required coverage area | Status | Evidence in `parent.md` |
|---|---|---|
| **Package identity** | ✓ Covered | "Package and lifecycle contract" section: exact package, version, manifest, install scopes, Pi extension entrypoint, npm-as-authority tension. |
| **Semantic equivalence** | ✓ Covered | "Semantic compatibility" section: supported nine-event subset, command-only limitation, and bullet list of material incompatibilities (async, Stop, exit codes, timeouts, updatedInput/updatedToolOutput, casing, etc.). |
| **Health/ownership** | ✓ Covered | "Health and compound-profile observation" section: presence vs. effective health, inert-when-no-`hooks`-key state, TUI-only enablement, independent install/remove/update identity. |
| **MCP interaction** | ✓ Covered | Paragraph on disjoint config surfaces and the `mcp__.*` tool-event coupling that can block MCP-adapter-mediated tools. |
| **Adapter acceptance evidence** | ✓ Covered | "Required Pi adapter evidence" section: seven-item checklist mapping research findings to the mutation-authority gate, with explicit statement of which items the current package clears/fails. |

---

## Completion decision

**Current `@hsingjui/pi-hooks@0.0.2` must not grant mutation authority to the Pi compound target under skilltap's hook-equivalence contract.**

The parent synthesis correctly reaches the observe-only verdict:

1. **Semantic gate fails.** The package is a partial, behaviorally divergent shim: nine of ~30 Claude events, `command`-only, ignored `async`, unfaithful Stop timing, exit-code blocking limited to `PreToolUse`, merge-vs-replace `updatedInput`, field-name mismatch on `updatedToolOutput`, wrong timeout defaults, lowercase tool-name matching, and missing safety caps.
2. **Health/enablement gate is not independently observable.** `pi list` reports presence and path but not version, health, or enable state. The extension is inert when the `hooks` key is absent. Resource enable/disable is TUI-only (`pi config`).
3. **Identity/lifecycle is observable but insufficient.** npm is the authoritative version surface; the repository HEAD is not authoritative. The package has stable identity and install/remove/update surfaces, but those alone do not clear the semantic gate.

The hook companion therefore fails the semantic half of the compound-target mutation gate. A compiled Pi profile should remain observe-only for hook-mediated mutation until the package (or a successor) clears the required evidence items.

---

## Result

`APPROVED` — the campaign parent synthesis is ready for the next ARD stage or for operational handoff. No source or work-item edits were made.
