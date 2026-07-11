---
id: epic-rust-control-plane-website-cli-contract
kind: story
stage: review
tags: [content, documentation]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-cli-shell]
release_binding: null
research_refs: []
research_origin: null
gate_origin: docs
created: 2026-07-11
updated: 2026-07-11
---

# Align the Public CLI Result Contract

Update the public website CLI reference to the authoritative foundation and
implemented contract: plain result labels, schema-1 JSON fields, and exit codes
`0`–`3`. Remove obsolete top-level `targets`/`exit_code` claims, regenerate
`website/public/llms-full.txt` through the repository's generator, and verify
the website build and exact generated-output check. Do not change product
behavior or foundation docs.

## Implementation notes

- Files changed: `website/reference/cli.md`, generated
  `website/public/llms-full.txt`, and this story.
- Tests added: none; this is a public-documentation contract correction.
- Verification: ran the existing `gen-llms-txt.mjs` generator, confirmed a
  second generation was byte-identical, and completed the VitePress production
  build.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch: direct-read only; the authoritative foundation and renderer/output
  implementation made the correction surface unambiguous.

## Review correction

- Restored the foundation's command-specific human result vocabulary for plain
  output. The initial correction had mistakenly documented the temporary
  generic renderer labels as the intended public contract.
- Kept the schema-1 JSON result classes and exit-code mapping unchanged.
- Regenerated the LLM bundle, repeated the byte-identity check, and rebuilt the
  website after the correction.
