# Verification record

## Mechanical lint

- Three specialist briefs: 67 resolved citations, 0 broken, 0 thin attestations, 0 pattern flags.
- Parent synthesis after revision: 60 resolved citations, 0 broken, 0 thin attestations, 0 pattern flags.
- URL checks were disabled for the final mechanical run because Anthropic's documentation rejects the linter's HEAD probe; specialists verified direct GET access.

## Lead spot-check

The lead sampled load-bearing claims across authorities and claim shapes:

- `agentskills-spec`: confirmed that the managed artifact is the complete directory with top-level `SKILL.md`, not the Markdown file alone.
- `codex-agents-md`: confirmed Codex-home global instruction paths, override precedence, and root-to-working-directory project layering.
- `claude-memory`: confirmed that Claude reads `CLAUDE.md`, not `AGENTS.md`, and documents import or symlink bridging.
- `codex-build-plugins` plus `codex-plugins`: confirmed the separation between non-interactive marketplace commands and the documented interactive plugin browser.
- `claude-plugins-reference`: confirmed native plugin commands, versioned caching, and manifest/marketplace/SHA update precedence.

The spot-check found no cite-through to the imported legacy lens, no unsupported quotation framing, and no semantic mismatch between the sampled synthesis claims and their attestations. Composed product recommendations are explicitly marked as inferred after the first adversarial pass.

## Result

Mechanical and lead spot-check floors pass. The second adversarial pass approved
the revised synthesis; its checklist and verdict are recorded in
`adversarial-review.md`.
