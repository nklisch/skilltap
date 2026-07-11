---
id: epic-cross-harness-materialization-graph
kind: feature
stage: review
tags: []
parent: epic-cross-harness-materialization
depends_on: [epic-native-marketplace-plugin-lifecycle, epic-standalone-skill-lifecycle]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Build Source Component Graphs

Represent plugin components, dependencies, requiredness, provenance, and
target-independent identity without browsing or guessing source contents.

## Design decisions

- **Where is source truth read?** A harness adapter reads only an explicitly
  selected plugin source through a core port; the core never searches
  marketplaces, caches, or arbitrary directories.
- **How are component identities assigned?** Adapters emit documented,
  target-independent component identifiers. The normalizer rejects duplicates,
  dangling dependencies, self-dependencies, and cycles rather than repairing
  or namespacing them.
- **How is provenance represented?** Semantic component data remains the
  existing `ComponentGraph`; a source graph adds validated relative paths and
  declared names so later materializers can explain exactly what they copied.

## Architectural choice

Use a typed source-graph normalizer in `skilltap-core` fed by a narrow
`PluginGraphReader` port. The alternative of letting each harness adapter
construct a target-specific `ComponentGraph` would duplicate dependency and
validation rules and make cross-harness identity drift likely. A raw manifest
blob would preserve more input but would leak native formats into the core and
make compatibility decisions non-deterministic. The chosen design keeps native
parsing in adapters, validates the normalized declarations once, and carries
only bounded provenance across the port.

## Implementation Units

### Unit 1: Source graph contract and normalizer (trickiest unit)
**File**: `crates/core/src/plugin_graph.rs`
**Story**: `epic-cross-harness-materialization-graph-contract`

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    ComponentGraph, ComponentGraphError, ComponentId, ComponentKind,
    ComponentRequiredness, RelativeArtifactPath, Source,
};

pub struct ComponentDeclaration {
    pub id: ComponentId,
    pub kind: ComponentKind,
    pub requiredness: ComponentRequiredness,
    pub dependencies: BTreeSet<ComponentId>,
    pub relative_path: RelativeArtifactPath,
    pub declared_name: Option<String>,
}

pub struct ComponentProvenance {
    relative_path: RelativeArtifactPath,
    declared_name: Option<String>,
}

pub struct SourceComponentGraph {
    source: Source,
    components: ComponentGraph,
    provenance: BTreeMap<ComponentId, ComponentProvenance>,
}

pub trait PluginGraphReader {
    fn read(
        &self,
        source: &Source,
    ) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError>;
}

pub fn normalize(
    source: Source,
    declarations: impl IntoIterator<Item = ComponentDeclaration>,
) -> Result<SourceComponentGraph, PluginGraphError>;
```

**Implementation Notes**:
- Convert declarations into the existing `ResourceComponent` and
  `ComponentGraph`; `ComponentGraph::new` remains the single dependency
  validator.
- Validate every relative path and optional declared name before constructing
  the graph. Preserve deterministic `BTree*` ordering and never retain raw
  manifest bytes, argv, or cache paths.
- `PluginGraphReadError` distinguishes an unavailable explicit source,
  malformed documented manifest, and unsupported source kind. It is a
  boundary error, not a compatibility classification.

**Acceptance Criteria**:
- [ ] Normalizing the same declarations in different input orders yields equal
      graphs and provenance.
- [ ] Duplicate IDs, dangling dependencies, self-dependencies, and cycles
      fail before a graph is returned.
- [ ] A declaration with an invalid relative path or invalid declared name
      fails without retaining any source data.
- [ ] The reader port cannot be called without an explicit `Source` argument.

### Unit 2: Native manifest readers
**File**: `crates/harnesses/src/plugin_graph.rs`
**Story**: `epic-cross-harness-materialization-graph-readers`

```rust
pub struct CodexPluginGraphReader { /* verified filesystem/process ports */ }
pub struct ClaudePluginGraphReader { /* verified filesystem/process ports */ }

impl PluginGraphReader for CodexPluginGraphReader {
    fn read(&self, source: &Source)
        -> Result<Vec<ComponentDeclaration>, PluginGraphReadError>;
}
impl PluginGraphReader for ClaudePluginGraphReader {
    fn read(&self, source: &Source)
        -> Result<Vec<ComponentDeclaration>, PluginGraphReadError>;
}
```

**Implementation Notes**:
- Parse only documented `.codex-plugin/plugin.json` or
  `.claude-plugin/plugin.json` and convention directories for the explicitly
  selected source. Use existing filesystem/process ports and bounded reads.
- Emit stable IDs, requiredness, dependencies, relative paths, and declared
  names for skills, MCP, hooks, and other documented components. Unknown
  component declarations are returned as target-specific kinds or explicit
  unsupported declarations; they are never silently dropped.
- Keep caches observation-only and do not infer components by marketplace
  browsing.

**Acceptance Criteria**:
- [ ] Codex and Claude fixture plugins produce normalized declarations for all
      documented component directories in the fixture.
- [ ] Missing or malformed manifests return a typed error and no partial graph.
- [ ] An unselected cache or neighboring repository is never read.
- [ ] Reader tests verify bounded process arguments and preserve unknown native
      fields outside the normalized contract.

### Unit 3: Graph integration and evidence handoff
**File**: `crates/core/src/materialization.rs`
**Story**: `epic-cross-harness-materialization-graph-integration`

```rust
pub fn read_and_plan_graph<R: PluginGraphReader>(
    reader: &R,
    source: Source,
) -> Result<SourceComponentGraph, PluginGraphError>;
```

**Implementation Notes**:
- Add a pure handoff that invokes the reader once, normalizes declarations,
  and passes the resulting semantic graph to the existing materialization
  planner without selecting targets or acknowledging loss.
- Keep source graph evidence ephemeral until the later compatibility and
  publish features decide what state and managed-artifact records persist.

**Acceptance Criteria**:
- [ ] Reader failures prevent planning and produce no inventory, state, or
      managed-artifact writes.
- [ ] A successful graph is accepted by `plan_materialization` with the same
      required/optional semantics as the source declarations.
- [ ] Repeating the handoff with unchanged source declarations is identical.

## Implementation Order

1. `epic-cross-harness-materialization-graph-contract`
2. `epic-cross-harness-materialization-graph-readers`
3. `epic-cross-harness-materialization-graph-integration`

## Testing

- Unit tests in `crates/core/src/plugin_graph.rs` cover validation, ordering,
  provenance, and typed errors.
- Harness tests use fixture plugins for both native manifests and assert that
  only explicit source paths are read.
- Core integration tests verify graph-to-materialization planning and the
  no-write behavior on reader failure.

## Risks

The main risk is that a native manifest can describe behavior not expressible
by the normalized component kinds. The fallback is to preserve a documented
target-specific component declaration with provenance and let the compatibility
feature classify it as unsupported or partial; the graph reader must not guess
an equivalent kind.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this autopilot
  run is intentionally single-agent and no different model was selected.

## Implementation notes

- Completed child stories: `epic-cross-harness-materialization-graph-contract`,
  `epic-cross-harness-materialization-graph-readers`, and
  `epic-cross-harness-materialization-graph-integration`.
- Delivered typed source graph normalization, bounded Codex/Claude manifest
  readers, provenance, and pure graph-to-materialization planning handoff.
- Verification: targeted core and harness tests plus clippy passed; the full
  workspace suite remains the final feature review gate.
