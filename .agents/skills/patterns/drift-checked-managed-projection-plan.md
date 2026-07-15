# Drift-Checked Managed Projection Plan

Plan skill trees and MCP configuration as fingerprinted writes, re-observe every owned projection against its prior fingerprint, and fail with `Drifted` rather than overwriting a replacement.

## Rationale

File-managed adapters own documented destination paths and codecs, while planning and application happen in separate phases. A harness, operator, or concurrent skilltap process can replace projected content between those phases. The adapter must therefore describe current and desired bytes without mutating, verify previously owned identities against recorded fingerprints, and surface drift instead of silently clobbering another writer.

Combining per-projection current and desired byte parts into `ManagedProjectionPlan` fingerprints gives the executor one transactional bundle to revalidate under lock while preserving adapter-specific formats such as JSON, JSONC, TOML, and skill directory trees.

## Examples

### Assemble trees, files, manifests, and dual fingerprints

**File**: `crates/harnesses/src/adapters/gemini_managed.rs:77`

```rust
fn plan_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
    let plugin = match &context.input {
        ManagedProjectionInput::Apply { checkout } => {
            Some(read_selected_plugin(context, checkout)?)
        }
        ManagedProjectionInput::Remove => None,
    };
    let (skill_root, config_root) = destination_paths(context)?;
    let (trees, mut current_parts, mut desired_parts, skill_manifest) =
        plan_skills_with_policy(&skill_root, context, plugin.as_ref(), SKILL_POLICY)?;
    let (mcp_write, mcp_manifest) = plan_mcp(
        &config_root,
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;

    Ok(ManagedProjectionPlan {
        trees,
        files: mcp_write.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal && !desired_parts.is_empty())
            .then(|| fingerprint_contents(&desired_parts)),
    })
}
```

### Reject a replaced owned skill tree

**File**: `crates/harnesses/src/adapters/configuration_constrained/common.rs:341`

```rust
fn verify_prior_skill(
    prior: &[ManagedProjection],
    destination: &RelativeArtifactPath,
    current: Option<&ObservedTree>,
    policy: SkillProjectionPolicy,
) -> Result<(), ManagedProjectionError> {
    let Some(expected) = prior.iter().find_map(|projection| match projection {
        ManagedProjection::Skill { id, fingerprint } if id == destination => Some(fingerprint),
        _ => None,
    }) else {
        return Ok(());
    };
    if current
        .map(|(_, tree)| fingerprint_tree(destination, tree))
        .as_ref()
        != Some(expected)
    {
        return Err(ManagedProjectionError::Drifted {
            detail: policy.diagnostics.drifted,
        });
    }
    Ok(())
}
```

### Distinguish owned drift from an unowned MCP conflict

**File**: `crates/harnesses/src/adapters/qwen_managed.rs:574`

```rust
if let Some(expected_fingerprint) = prior {
    if current.as_ref().map(json_fingerprint).as_ref() != Some(expected_fingerprint) {
        return Err(ManagedProjectionError::Drifted {
            detail: "An owned Qwen MCP server is missing or was replaced.",
        });
    }
    if let Some(value) = &current {
        current_parts.extend(json_fingerprint_bytes(value));
    }
} else if current.is_some() && !removal {
    return Err(ManagedProjectionError::McpConflict);
}
```

### Centralize the shape for a harness family

**File**: `crates/harnesses/src/adapters/configuration_constrained/common.rs:140`

```rust
pub(crate) fn plan_skills<P: SkillProjectionSource + ?Sized>(
    skill_root: &AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&P>,
) -> Result<SkillProjectionPlan, ManagedProjectionError> {
    plan_skills_with_policy(
        skill_root,
        context,
        plugin,
        SkillProjectionPolicy::agent_skill_contract(),
    )
}
```

## When to Use

- Adding a file-managed harness adapter that owns skill directories or an MCP configuration document.
- Any adapter where destination content can change between planning and application.
- Building one atomic projection bundle across both directory trees and configuration files.

## When NOT to Use

- Native lifecycle adapters that mutate through a bounded harness CLI.
- Observe-only targets that never plan mutations.
- Pure compatibility classification without destination writes.

## Common Violations

- Mutating destination content while planning instead of producing expected and desired snapshots.
- Skipping the prior-fingerprint comparison for an owned projection.
- Treating an unowned existing entry as drift instead of a conflict.
- Fingerprinting an unrelated whole document rather than the owned projection parts.
- Failing to clear the managed manifest during removal, or clearing it during apply.
