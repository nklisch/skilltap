# Research Conventions

The `.research/` substrate holds external source material, source-direct
attestations, source-coherent precis, and cross-source analysis. It is separate
from `.work/`, which holds operational decisions and delivery state.

## Layout

- `reference/<corpus>/` — raw source fetches and an append-only corpus `INDEX.md`.
- `attestation/<handle>.md` — one source-direct attestation per fetched source.
- `precis/` — source-coherent aggregations authored from raw material.
- `analysis/briefs/` — standalone cross-source briefs.
- `analysis/campaigns/` — multi-specialist and program research bundles.
- `analysis/positions/` — settled positions.
- `analysis/hypothesis/` — working hypotheses and ledgers.
- `.import-holding/` — retained, non-authoritative legacy lenses awaiting refresh.

Reads flow down-gradient only: `reference → attestation → precis → analysis`.
Lower tiers never read project framing or higher-tier analysis.

## Frontmatter contracts

Attestations require:

```yaml
source_handle: <handle>
fetched: <YYYY-MM-DD>
source_url: <URL> # or source_path
provenance: source-direct
```

Precis artifacts require intrinsic source and temporal metadata plus:

```yaml
provenance: agent-authored-from-raw
```

Analysis briefs and positions require intrinsic temporal metadata plus an
authorial-role provenance value. Allowed `provenance` values are:

- `source-direct`
- `agent-authored-from-raw`
- `agent-synthesis`
- `generated-listing`
- `hybrid-curated`

Refresh outputs carry `supersedes` and `refresh_verification` alongside the
scalar `provenance` field. Imported holding artifacts carry `import_origin` and
`intended_output_kind`; they never carry authoritative-tier provenance.

## Citation rule

Research citations use `[handle]{N}`. `N` resolves against the append-only
numbered bibliography in `reference/<corpus>/INDEX.md` and `references.md`.

The required chain is:

```text
brief claim → [handle]{N} → attestation/<handle>.md → fetched source
```

The cited specific must already appear in the attestation before the citation
is authored. Analytical artifacts and import-holding files are lenses, never
citation targets.

## Typed cross-references

Artifacts may carry directed typed relationships:

```yaml
related:
  - to: <slug-or-relative-path>
    type: <predicate>
    note: <optional rationale>
```

Author only the forward edge. Reverse views are derived.

## Lifecycle

Research artifacts use a `status` and a `temporal_contract`. Baseline temporal
contracts are `write-once-on-converge`, `extend-on-source-rev`,
`supersedes-prior`, `ttl-bounded`, and `re-engage-on-trigger`.

Research does not use work-item drafting, implementation, review, or done
stages. Corrections fix an artifact in place with a revision record. Reversals
and refreshes produce a new artifact with a `supersedes` pointer while retaining
the prior record.

## Authoring and enforcement

Every research-authoring context follows the ARD discipline bundle. Source-bound
citation, per-source attestation, active disconfirmation, contradiction
handling, composed-claim markers, and the terminal spot-check are mandatory.

Run the plugin's `lint-citations.py` against every authoritative synthesis with
`.research/attestation/` as the attestation directory. The lint is a mechanical
floor; it does not replace the lens exclusion or semantic spot-check.

## Invariants

- Never fabricate a citation, footnote, bibliographic field, or named-feature claim.
- Never cite a prior analysis or import-holding lens as source substrate.
- Never synthesize from a source before its attestation exists.
- Surface contradictions structurally; do not average them away.
- Mark composed or uncertain claims with the closed epistemic-status vocabulary.
- Preserve the reference → attestation → precis → analysis direction.
- Research may ground `.work/` through explicit handoff; operational state never rewrites research.
- Raw sources are substrate, not analytical lenses.
