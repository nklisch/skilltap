---
description: Adopt, plan, and reconcile marketplaces, plugins, and skills.
---

# Managing Environments

skilltap separates desired inventory from fresh native observations. That makes
the direction explicit: `adopt` brings native configuration into inventory;
`sync` applies inventory to managed harnesses.

## Adopt native state

```bash
skilltap adopt --from codex
skilltap adopt --from claude --project
```

Adoption preserves source and scope, coalesces resources only when identity and
semantics match, and leaves conflicts for a caller to resolve. It never invokes
native mutation.

## Register explicit resources

```bash
skilltap marketplace add <source>
skilltap plugin install <plugin>@<marketplace>
skilltap skill install <source>
```

Marketplace sources and resource identities come from the caller. skilltap
does not browse or search their contents.

A standalone skill is its whole directory, not only `SKILL.md`. Compatible
skills use the standard `.agents/skills/<name>/` representation; adapters add
harness-specific links or managed copies only where the harness requires them.

## Plan before mutation

```bash
skilltap plan
skilltap plan --project --target claude
skilltap plan --all-scopes --json
```

Plans classify operations as native, faithful equivalent, managed
materialization, partial, unsupported, conflicting, or no-op. Every native
project file that may change appears in the plan.

## Apply and verify

```bash
skilltap sync
```

Safe independent operations may proceed even when another resource needs a
decision. A partial or lossy foreground operation requires explicit `--yes`.
That acknowledgment never overrides invalid configuration, missing required
components, drift, or native harness policy.

After mutation, skilltap observes the targets again. Repeating the same sync
against unchanged inputs produces no changes.
