---
id: epic-cross-harness-materialization-skills-mcp
kind: feature
stage: done
tags: []
parent: epic-cross-harness-materialization
depends_on: [epic-cross-harness-materialization-compatibility]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Materialize Portable Skills and MCP

Project portable skills and conditionally representable MCP components through
documented target paths with managed ownership.

## Design decisions

- **Where do portable skills land?** The canonical `.agents/skills/<name>/`
  tree is the managed source when the target can load it. A harness-specific
  complete-directory projection is planned only when the target requires it;
  `SKILL.md` is never copied alone.
- **When is MCP portable?** Only when the concrete transport, authentication
  references, variables, and load path are documented and preserved. Secrets
  and credential values never enter a projection or state record.
- **Who writes projections?** This feature emits target-bound projection plans;
  the downstream publish feature performs atomic filesystem publication and
  ownership recording.

## Architectural choice

Use a pure projection plan with adapter-owned load-root mappings. A generic
"copy every component" implementation was rejected because MCP semantics and
skill load paths differ by harness. Writing directly from this feature was
also rejected because publication, backup, and ownership are a separate
transaction boundary. The chosen plan records complete source trees, target
roots, and conditional MCP metadata without performing I/O.

## Implementation Units

### Unit 1: Portable skill projection planning (trickiest unit)
**File**: `crates/core/src/materialization.rs`
**Story**: `epic-cross-harness-materialization-skills-mcp-skills`

```rust
pub enum ProjectionRoot {
    CanonicalAgentsSkills,
    CodexSkills,
    ClaudeSkills,
}

pub struct ComponentProjection {
    pub component: ComponentId,
    pub target: HarnessId,
    pub root: ProjectionRoot,
    pub source_path: RelativeArtifactPath,
    pub destination: RelativeArtifactPath,
    pub complete_tree: bool,
}

pub fn plan_skill_projections(
    graph: &SourceComponentGraph,
    materialization: &MaterializationPlan,
    target: &HarnessId,
) -> Result<Vec<ComponentProjection>, ProjectionError>;
```

**Implementation Notes**:
- Only `ComponentKind::Skill` entries with source provenance are projected.
- Validate that each skill source points to a complete directory with a
  top-level `SKILL.md`; preserve all sibling files and never flatten content.
- Destination names derive from the normalized component identity without
  renaming collisions. The canonical root is emitted once per source skill;
  target-specific roots are emitted only when required by the adapter mapping.

**Acceptance Criteria**:
- [ ] Every included skill produces a complete-tree projection with an exact
      source and destination path.
- [ ] Missing provenance, invalid names, and non-skill components fail closed.
- [ ] Repeated planning is deterministic and emits no filesystem operation.

### Unit 2: Conditional MCP projection mapping
**File**: `crates/core/src/materialization.rs` and `crates/harnesses/src/materialization.rs`
**Story**: `epic-cross-harness-materialization-skills-mcp-mcp`

```rust
pub struct McpProjection {
    pub component: ComponentId,
    pub target: HarnessId,
    pub destination: RelativeArtifactPath,
    pub transport: McpTransport,
    pub credential_references: BTreeSet<String>,
}

pub trait McpProjectionMapper {
    fn map(
        &self,
        component: &ResourceComponent,
        provenance: &ComponentProvenance,
        target: &HarnessId,
    ) -> Result<McpProjection, ProjectionError>;
}
```

**Implementation Notes**:
- Parse concrete MCP declarations through strict bounded JSON/TOML adapters;
  preserve transport and variable references but redact or reject credential
  values.
- Unsupported transports, auth mechanisms, or target load paths return a
  compatibility consequence rather than a guessed copy.
- Mapping returns only relative managed destinations; the publish feature owns
  absolute path resolution and atomic writes.

**Acceptance Criteria**:
- [ ] Supported stdio/HTTP transport fixtures preserve command/URL semantics
      and credential references without secrets.
- [ ] Unsupported auth/transport is an explicit projection error with the
      affected component identity.
- [ ] A mapper never reads or writes a harness cache as a configuration API.

### Unit 3: Projection plan integration
**File**: `crates/core/src/materialization.rs`
**Story**: `epic-cross-harness-materialization-skills-mcp-integration`

```rust
pub struct ProjectionPlan {
    pub skills: Vec<ComponentProjection>,
    pub mcp: Vec<McpProjection>,
}

pub fn plan_component_projections(
    graph: &SourceComponentGraph,
    materialization: &MaterializationPlan,
    target: &HarnessId,
    mcp: &impl McpProjectionMapper,
) -> Result<ProjectionPlan, ProjectionError>;
```

**Implementation Notes**:
- Consume only components included by the prior compatibility/materialization
  plan; omitted optional components remain visible to that plan and are not
  silently reintroduced.
- Sort projections by component identity and destination for stable output.
- Keep managed ownership metadata out of this pure plan; publication records it
  after complete-tree writes succeed.

**Acceptance Criteria**:
- [ ] A partial materialization plan cannot produce a projection for an
      excluded component.
- [ ] Skill and MCP projections are deterministic and target-bound.
- [ ] Mapping failures return before any publication or state mutation.

## Implementation Order

1. `epic-cross-harness-materialization-skills-mcp-skills`
2. `epic-cross-harness-materialization-skills-mcp-mcp`
3. `epic-cross-harness-materialization-skills-mcp-integration`

## Testing

- Core fixtures cover complete skill trees, missing `SKILL.md`, source
  provenance, excluded components, and deterministic destination ordering.
- Harness fixtures cover Codex/Claude skill roots and MCP transport/auth
  mappings with secret-redaction assertions.
- Integration tests prove projection planning is read-only and consumes exact
  compatibility selectors.

## Risks

The main risk is assuming that a syntactically valid MCP document has portable
runtime semantics. The mapper therefore requires explicit transport/auth
support evidence and returns a typed failure for everything else; publish will
not receive an ambiguous projection.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this autopilot
  run is intentionally single-agent and no different model was selected.

## Implementation notes

- Completed child stories: `epic-cross-harness-materialization-skills-mcp-skills`,
  `epic-cross-harness-materialization-skills-mcp-mcp`, and
  `epic-cross-harness-materialization-skills-mcp-integration`.
- Delivered complete-tree canonical/Claude skill projections, strict MCP
  transport/auth mapping, credential-reference redaction, and pure composed
  projection plans.
- Verification: full workspace tests and clippy passed.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Deep feature review completed inline in degraded fresh-context mode
because this run is intentionally single-agent. The completeness pass verified
complete skill directories, canonical-vs-target roots, MCP references, and
excluded-component filtering. The adversarial pass rejected literal credentials
and non-equivalent transport types, confirming no secret or cache write path.
Full workspace tests and clippy passed.
