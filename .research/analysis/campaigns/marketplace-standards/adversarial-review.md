---
reviewed: 2026-07-10
review_target: .research/analysis/briefs/current-agent-extension-standards.md
verification_rigor: standard
verdict: NEEDS-REVISION
---

# Adversarial review: current agent extension standards

## Mechanical preflight

The citation linter completed with 40 resolved citations, 0 broken citations,
0 thin attestations, and 0 pattern flags. The attestation-tier audit reported no
findings. The bibliography handle, number, and source-URL mappings agree with
the attestation frontmatter for all cited sources.

## Verification checklist

### (a) Semantic citation-chain walk

The cited source claims are semantically supported: whole-directory skill
boundaries, native skill roots and symlink behavior, manifest requirements,
marketplace locations, native lifecycle commands, instruction paths, settings
scopes, cache behavior, and plugin version precedence all walk to specific
attested passages.

Revision finding: the contract map at lines 43-50 repeats load-bearing native
contract claims without citations in the table. Later sections cite most of the
same facts, but those later citations do not make the table's claims
source-bound at their point of assertion. Add citations to the relevant cells
or attach a source column/row citation for each native-contract row. Mark the
`skilltap posture` column as inferred design guidance rather than presenting it
as source-attested.

### (b) Claim shapes missed by mechanical lint

Revision finding: several composed recommendations lack epistemic-status
markers even though no attestation directly establishes them. The main cases
are the opening decision at lines 29-33, the canonical-placement recommendation
at lines 67-72, the compatibility taxonomy at lines 181-189, and the adapter
recommendations embedded in the contradiction treatments at lines 202-203 and
216-218. Add an appropriate `{inferred: ...}` marker at each composed claim or
rewrite the surrounding structure so its inferred scope is explicit.

Revision finding: documentation-absence claims at lines 93-95, 160-163, and
223-229 are carefully worded as gaps in the current public contract, but the
final brief does not record the search that supports that bounded absence.
Carry forward the specialist search scope and use a confidence marker such as
`{confidence: current-public-docs}` where the claim depends on corpus coverage.

No cite-through overreach, unsupported named-feature claim, effort estimate,
or comparative superlative was found.

### (c) Coherence read for smoothed contradictions

The brief preserves the strict-authoring versus tolerant-loading tension and
the Claude declaration-versus-installation qualification. It also correctly
treats Codex/Claude marketplace similarity as incommensurable rather than as
schema parity. No contradiction was smoothed into a false shared contract.

The recommendations appended to those relationships still need inference
markers as noted under job (b); that is a status-labeling problem, not a
contradiction-resolution problem.

### (d) Noise domination and relevance weighting

The most authoritative attestation is used for each major claim: Agent Skills
for the portable format, OpenAI documentation for Codex contracts, and
Anthropic documentation for Claude contracts. Specialist syntheses and the
legacy import are not used as evidence. No less-relevant citation displaced a
more direct attestation.

`codex-config-reference` is listed in refresh verification but not cited in the
final brief. That is consistent with a consulted source that did not need to
carry a final claim and is not a citation-chain defect.

### (e) Quote-context walk

The synthesis contains no verbatim source quotations, so no quote-framing issue
was available to surface.

### (f) Analytical-tier inheritance walk

All 40 citations resolve to source-direct attestation files. None resolves to a
specialist brief, prior synthesis, position, glossary, or the imported legacy
document. The legacy document is used only in the `supersedes` lineage and
refresh-delta comparison, never as evidence.

### (g) Line-reference walk

The synthesis does not cite source line or section ranges directly. The
attestations contain source-internal anchors and line references, and the
claims checked under job (a) derive from those recorded passages.

### (h) Thin-attestation semantic check

The cited attestations are substantively sufficient for the claims they carry:
each records a scoped summary plus load-bearing passages or anchors. No cited
attestation is merely a heading, token quotation, or whole-source gloss unable
to support per-claim review.

## Required revision

1. Add source-bound citations to the native factual claims in the contract map.
2. Mark the map's skilltap posture and the other composed recommendations named
   under job (b) as inferred.
3. Add an explicit `## Disconfirming analysis` section. Record the search
   outcomes for Codex non-interactive plugin mutation and update semantics,
   the exact global Claude symlink composition, and Agent Skills distribution
   or lifecycle primitives; apply bounded confidence markers to absence claims.
4. Re-run citation lint after the revision.

## Verdict

**NEEDS-REVISION**

The evidence substrate and core conclusions are strong, but the final artifact
does not yet satisfy the discipline's source-bound assertion, inference-marker,
and explicit disconfirming-analysis requirements.

## Revision pass — 2026-07-10

### Prior-finding verification

- **Contract-map citations: resolved.** Each Codex and Claude native-contract
  cell now carries a point-of-claim citation to the appropriate source-direct
  attestation. The standalone portable form is also cited to Agent Skills. The
  introductory sentence explicitly scopes the final column as `{inferred:
  skilltap posture}` guidance.
- **Inference markers: resolved.** The opening architecture decision,
  canonical placement, state separation, compatibility taxonomy, consent rule,
  global bridge composition, and the recommendations in the contradiction
  treatments are now explicitly marked as inferred. The markers separate
  product and adapter conclusions from native contract claims.
- **Disconfirming analysis: resolved.** The synthesis now has an explicit
  `## Disconfirming analysis` section covering the searched Codex mutation and
  update surface, the composed global Claude bridge, and the absence of Agent
  Skills distribution/lifecycle primitives.
- **Bounded absence claims: resolved.** Current-documentation gaps are framed
  with `{confidence: current-public-docs}` and described as bounded source-set
  findings rather than proof that no implementation behavior exists.
- **Refresh lineage: intact.** The legacy import remains a `supersedes` lens,
  not evidence; all refreshed source handles remain declared and no old claim
  is grandfathered without verification.

### Fresh checks (a-h)

- **(a) Semantic chains:** the newly deployed table citations support the
  claims in their cells; no citation was added merely for proximity.
- **(b) Missed claim shapes:** the previously unmarked composed claims now
  carry explicit status. No new unsupported named feature, comparative
  superlative, or composed effort estimate appeared.
- **(c) Contradictions:** strict-versus-tolerant loading, Claude declaration
  versus installation, and native marketplace incommensurability remain
  structurally visible rather than merged.
- **(d) Relevance weighting:** primary OpenAI, Anthropic, and Agent Skills
  attestations still carry their respective authority boundaries; analytical
  artifacts are not substituted for sources.
- **(e) Quote context:** the synthesis still contains no source quotations.
- **(f) Analytical inheritance:** every citation resolves to source-direct
  attestation; the prior import remains uncited as evidence.
- **(g) Line references:** no synthesis claim asserts a source line range;
  attestation anchors remain sufficient for the semantic walk.
- **(h) Thin attestations:** no cited attestation is substantively thin for
  the claim it supports.

### Revision lint

Citation lint completed with **62 resolved citations, 0 broken, 0 thin, and 0
pattern flags**. The attestation-tier audit reported no findings.

### Final verdict

**APPROVED**

The revision satisfies every required change from the first adversarial pass
and preserves the source boundary, contradiction ledger, and refresh lineage.
