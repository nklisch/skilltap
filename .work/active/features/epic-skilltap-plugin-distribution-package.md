---
id: epic-skilltap-plugin-distribution-package
kind: feature
stage: done
tags: [architecture, infra]
parent: epic-skilltap-plugin-distribution
depends_on: []
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Canonical Plugin Package and Channel Metadata

## Brief

Create the repository-owned plugin publication tree described by the
foundation: one complete `skilltap` skill directory plus separate native
Claude and Codex manifests and marketplace catalog definitions. The feature
establishes the public plugin identity, component paths, portable frontmatter
rules, and version/source parity checks that every later publication step can
trust.

This is the package contract, not the final guidance prose, binary bootstrap,
or release workflow. It must preserve the harness distinction: Claude and
Codex documents are validated independently, and no Pi or universal plugin
manifest is introduced.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: foundation package contract; bootstrap, guidance, and
  release work depend on it.

## Foundation references

- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Validation
- `docs/ARCH.md` — Plugin Publication Boundary
- `docs/HARNESS-CONTRACTS.md` — Codex and Claude plugin/marketplace contracts
- `.research/analysis/briefs/current-agent-extension-standards.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/codex.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/claude.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md`

## Design decisions

- **Canonical source**: The package is authored and versioned in the skilltap
  repository. The active `../skills` repository publishes a second marketplace
  entry that points directly at this plugin subdirectory; `nklisch/skilltap-
  skills` is only a legacy migration source.
- **Channel parity**: Claude and Codex receive distinct native manifests and
  catalogs around one complete shared skill directory; no Pi channel or
  universal manifest is added.

## Architectural choice

Use one repository-owned publication root (`plugin/`) containing the complete
shared skill and both native channel documents. Each harness gets its own
manifest and marketplace catalog at the documented root-relative location;
neither catalog is translated from the other. The catalog entries resolve the
same package root with a documented `./` source, so a marketplace fetched from
this repository cannot drift into a copied plugin tree. The active sibling
publisher can therefore reference `plugin/` directly and the release feature
can verify one source identity.

The alternatives were (1) two channel-specific plugin roots, which would make
the skill and future references duplicate artifacts, and (2) a generated
archive-only layout, which would hide native documents from local validation and
make the development marketplace harder to exercise. The single-root layout
optimizes source-of-truth clarity while retaining native schemas; generated
release archives remain a release concern rather than a second authoring tree.

The package version is read from the Cargo workspace release identity during
validation. The manifests and catalog entries carry that same version and
public `skilltap` name; no independent package version file or sibling copy is
introduced. Catalog source URLs/refs are intentionally release-owned, while the
checked-in development catalogs use the package-relative `./` source.

## Implementation Units

### Unit 1: Canonical publication assets

**Files**:

- `plugin/.claude-plugin/plugin.json`
- `plugin/.codex-plugin/plugin.json`
- `plugin/.claude-plugin/marketplace.json`
- `plugin/.agents/plugins/marketplace.json`
- `plugin/skills/skilltap/SKILL.md`

**Story**: `story-skilltap-plugin-package-assets`

The two manifests use their native schemas and expose the same public identity
(`skilltap`), release version, and concise package description. Claude's
marketplace catalog includes the required `name`, `owner`, and `plugins` fields;
Codex's catalog uses its documented marketplace entry shape. Both entries point
to the package root through a native relative source and never use a Claude
catalog as a Codex catalog or vice versa. The initial `SKILL.md` is a strict,
loadable package stub (the guidance feature owns its substantive prose) and
must remain a complete skill directory boundary; future supporting files stay
beside it under `plugin/skills/skilltap/`.

The package root must not contain hooks, MCP servers, executables, or other
components that the guidance brief does not require. A component added later
must be represented in both channel compatibility decisions before it is
published.

### Unit 2: Native package contract validation

**File**: `crates/cli/tests/plugin_package.rs`

**Story**: `story-skilltap-plugin-package-validation`

The compiled workspace test suite validates the checked-in publication tree at
the repository boundary. It should use the existing complete-tree and
frontmatter rules rather than introducing a second parser. The test helper
surface is:

```rust
fn package_root() -> std::path::PathBuf;
fn read_json(relative: &str) -> serde_json::Value;
fn assert_manifest(relative: &str, expected_name: &str, expected_version: &str);
fn assert_marketplace(relative: &str, expected_channel: Channel, expected_version: &str);
fn assert_complete_skill(relative: &str, expected_name: &str);
fn source_resolves_inside_package(value: &serde_json::Value) -> bool;
```

`Channel` is a private test enum with `Claude` and `Codex` variants; it keeps
channel-specific required fields and source interpretation explicit rather than
reusing one flattened schema. `assert_complete_skill` verifies that the
directory exists, has a regular top-level `SKILL.md`, preserves all sibling
files, has strict `name`/`description` frontmatter, and contains no symlinked
artifact. `assert_manifest` and `assert_marketplace` validate JSON shape,
identity, version, channel-specific paths, and source containment. Tests must
fail for missing required fields, mismatched name/version, malformed JSON,
`../` source traversal, missing top-level `SKILL.md`, and a catalog that points
to a different plugin root.

## Implementation Order

1. `story-skilltap-plugin-package-assets` — establish the single package tree,
   native documents, and strict skill stub.
2. `story-skilltap-plugin-package-validation` — add boundary validation and
   malformed-package fixtures/tests; depends on the assets being present.

The feature itself owns no release automation and does not edit `../skills`.
Release integration later validates the sibling marketplace pointer against this
package root; the cutover feature handles the separate legacy repository.

## Testing

### Package-shape tests: `crates/cli/tests/plugin_package.rs`

- Valid package tree loads as a complete `skilltap` skill for both target
  harnesses.
- Claude and Codex manifests preserve their native directories and do not
  appear under the other channel's reserved manifest directory.
- Both catalogs contain one exact `skilltap` entry whose relative source stays
  within `plugin/` and whose version matches `CARGO_PKG_VERSION`.
- Catalogs do not expose arbitrary marketplace contents or discovery metadata.
- Complete-directory assertions retain supporting files if the guidance
  feature adds them later.

### Negative contract cases

Validation fixtures (temporary copied package roots, never the working tree)
cover malformed JSON, missing `owner`/`plugins` or channel-required fields,
identity/version drift, source traversal, missing/non-regular `SKILL.md`,
invalid frontmatter, and symlinked skill entries. The test harness must keep
these fixtures under its isolated temporary root and never mutate the user's
home, native harness caches, or `../skills`.

## Risks

- **Native schema drift**: Codex's marketplace schema and non-interactive
  plugin surface are less fully documented than Claude's. Keep parsing
  channel-specific and fail validation with the exact document/path; do not
  infer Codex fields from Claude. If the public schema changes, update the
  attested contract before changing package files.
- **Version drift at release time**: Static package metadata can diverge from
  Cargo unless the validator compares every manifest/catalog version to the
  workspace package version. Release checks must repeat this assertion against
  the tagged source and generated assets.
- **Future skill resources**: The skill is a directory, not a lone Markdown
  file. Tests must fingerprint the complete tree and reject symlink escapes so
  guidance references cannot accidentally rely on files outside the plugin.
- **Self-source resolution**: Relative `./` entries are safe only when resolved
  from the marketplace root. Validation must reject traversal and release
  integration must use an explicit repository subdirectory pointer for the
  active sibling publisher.

## Acceptance Criteria

- `plugin/` contains one complete `skills/skilltap/` directory with a strict
  top-level `SKILL.md` and separate Claude/Codex native manifests and catalogs.
- Every native document validates independently against its channel contract;
  no channel file is interpreted through the other channel's schema.
- The package's public identity and version are consistent across both
  manifests and catalogs and match the Cargo release identity.
- Relative catalog sources resolve inside `plugin/` and never traverse above
  the marketplace root.
- Boundary tests cover valid and malformed package trees, preserve supporting
  skill resources, reject symlink escapes, and run entirely in isolated test
  roots.
- No package asset introduces marketplace search, ranking, recommendation, or
  broad inventory discovery.

## Implementation notes

Both child stories are implemented and awaiting review. The canonical package
assets and boundary tests are self-contained under this repository; no active
publisher checkout or native harness state was changed.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard fresh-context feature review. Both child stories are
complete and independently reviewed. The package preserves native Claude and
Codex schemas around one complete skill directory, validates Cargo-version and
source parity at the repository boundary, and introduces no discovery or
native-state mutation. The plugin publication tree in `docs/ARCH.md` was
aligned with the implemented catalog locations before approval.
