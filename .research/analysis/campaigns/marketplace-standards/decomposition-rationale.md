# Decomposition rationale

## Candidate decompositions

### A. Ecosystem by authority boundary — chosen

- Codex native contracts
- Claude Code native contracts
- Agent Skills portable standard

This split preserves source authority and lets each specialist reason about a
coherent native system before the synthesis compares them. It also isolates the
portable standard from harness-specific extensions.

### B. Artifact layer

- Skills and agents
- Plugins and marketplaces
- Configuration and instructions
- Installation and update state

This is useful for comparison, but each lane would need to interpret two native
systems at once and could accidentally flatten distinct lifecycle semantics.

### C. Lifecycle operation

- Register and install
- Adopt and reconcile
- Update and remove
- Diagnose compatibility

This maps closely to skilltap commands, but biases the research toward the
planned product model before the native contracts have been established.

## Decision

Choose A for evidence collection, then use B and C as cross-join lenses in the
final synthesis. The decomposition is deliberately self-flagged: ecosystem lanes
may duplicate evidence about shared artifacts, so the synthesis must reconcile
overlapping claims rather than count independent specialists as corroboration.

## Bracket framing

- Lower bracket: exact file formats, paths, commands, schemas, scope rules, and update behavior stated by primary sources.
- Middle bracket: faithful equivalences and incompatibilities among the three authorities.
- Upper bracket: adapter requirements and constraints for skilltap; these are recommendations inferred from the lower and middle brackets, not claims about native tools.
