# skilltap

## Product

skilltap is a personal control plane for managing supported agent harness
environments. It adopts, reconciles, and updates native marketplaces, plugins,
complete skill directories, MCP configuration, and shared instructions.

It does not provide discovery, recommendations, security scanning, a TUI, or compatibility with the previous implementation.

## Foundation

Read these before designing or implementing changes:

- `docs/VISION.md` — purpose, audience, principles, and exclusions.
- `docs/SPEC.md` — authoritative behavior, commands, state, and reconciliation.
- `docs/ARCH.md` — Rust architecture and module boundaries.
- `docs/UX.md` — non-interactive CLI behavior and output.
- `docs/HARNESS-CONTRACTS.md` — Codex and Claude native contracts.

Foundation documents describe current or intended truth. Planned and time-bound work belongs in `.work/`.

## Product Constraints

- Commands are deterministic and non-interactive.
- Bare scoped commands operate globally.
- `--project` targets the current project.
- `--project <path>` targets another project.
- `--all-scopes` targets the managed computer.
- `--target` selects one registered harness or all enabled harnesses.
- `~/AGENTS.md` is the canonical global instruction file.
- A skill is the complete directory containing top-level `SKILL.md`.
- Native harness lifecycle commands take precedence over file materialization.
- Cross-harness transfer must be faithful or reported as partial.
- Partial foreground operations require explicit acknowledgment.
- Daemon updates never acknowledge partial operations or overwrite drift.
- skilltap does not search or browse marketplace contents.

## Architecture

The implementation is a stable Rust workspace:

- `skilltap-core` owns domain types, state, planning, reconciliation, compatibility, and updates.
- `skilltap-harnesses` owns the typed target registry, distinct native harness
  adapters, and user-service adapters.
- `skilltap-cli` owns argument parsing, rendering, and exit codes.
- `skilltap-test-support` owns fixtures and isolated test environments.

Dependencies point toward core. Core never writes terminal output or depends on concrete harness implementations.

Use native CLIs through direct argument vectors, not shell command strings. Prefer structured native output. Never treat harness caches as write APIs.

## State

skilltap stores machine-wide state under `${XDG_CONFIG_HOME:-$HOME/.config}/skilltap/`:

- `config.toml` — policy.
- `inventory.toml` — desired resources.
- `state.json` — machine-written provenance and observations.
- `managed/` — skilltap-owned artifacts and backups.

Authentication material and secrets never enter skilltap state.

## Development

The previous TypeScript implementation does not constrain the Rust architecture or public behavior. Do not add compatibility layers or migrations unless the foundation documents change explicitly.

Validate all external boundaries. Preserve unknown documented native fields when editing harness configuration. Keep reconciliation idempotent and test every mutating workflow by immediately repeating it and expecting no changes.

Use concise imperative Git commit messages with no trailers.

<!-- agile-workflow:start -->
## Agile-Workflow Substrate

Work tracked in `.work/` as markdown items with YAML frontmatter
(`kind, stage, tags, parent, depends_on, release_binding, research_refs,
research_origin`; a `[research]` item also carries the commissioning subset in
a `research_dials:` block: `scope_authority`, `verification_rigor`, `intent`,
`output_kind`).
Layout: `.work/active/{epics,features,stories}/`, `.work/backlog/`,
`.work/releases/<version>/`, `.work/archive/`.

**Primary query tool:** `.work/bin/work-view` filters by stage, tag, kind,
parent, and dependency. Common patterns:

- `work-view --ready` — items ready to work (deps satisfied)
- `work-view --stage review` — items awaiting an agent review pass (`/agile-workflow:review`)
- `work-view --parent <id>` / `--blocking <id>` — hierarchy / sequencing
- `work-view --scope all` — include terminal tiers: `releases/` (one summary doc per version) and
  `archive/` (bodyless ref stubs). Full bodies live in git history. By default work-view shows only
  active + backlog; `--release` / `--gate` auto-widen to all tiers.
- `work-view --help` for the full flag set

Foundation docs in `docs/` describe the system's current state or intended
future state, never the past; git history is the audit trail. Item files are
the durable state: update the body with implementation discoveries, review
findings, blockers, and decisions instead of relying on chat history.

Project agent rules live in `.agents/rules/*.md` (plugin-managed rules in
`.agents/rules/agile-workflow.md`); do not maintain `.claude/rules/*.md` as a
source of truth. No Rust code-pattern skill exists yet; document reusable
patterns only after they emerge from the v3 implementation.

**Before designing, implementing, or reviewing, read `.agents/rules/*.md`** —
the project's force-loaded agent rules. The agile-workflow hook auto-loads
these at session start and after compaction; read them directly when working
without the hook. Do not rely on prompt-time queue snapshots; query `work-view`
when queue state is needed.

Project-specific refactor style conventions belong in this file under
`## Refactor Style Conventions`. Detailed references belong in
`.agents/skills/refactor-conventions/` and extend `refactor-design`'s
defaults; they do not replace the built-in scan and they do not create
standalone plan docs.

Research handoffs flow one way from `.research/` into `.work/`; see the
agentic-research handoff contract. Research items carry `research_refs` and
`research_origin` back to their grounding artifacts.

<!-- agile-workflow:end -->
