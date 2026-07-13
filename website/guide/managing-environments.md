---
description: Adopt, plan, and reconcile marketplaces, plugins, and skills.
---

# Managing Environments

skilltap separates desired inventory from fresh native observations. That makes
the direction explicit: `adopt` brings native configuration into inventory;
`sync` applies inventory to managed harnesses.

Codex-native global paths resolve under
`${CODEX_HOME:-$HOME/.codex}`. This override does not move skilltap state and
never moves the canonical global instruction file at `~/AGENTS.md`.

## Adopt native state

```bash
skilltap adopt --from codex
skilltap adopt --from claude --project
```

Adoption preserves source and scope, coalesces resources only when identity and
semantics match, and leaves conflicts for a caller to resolve. It never invokes
native mutation.

Claude project-shared declarations are still observed because they can affect
the effective project environment. They are reported as visible,
non-adoptable evidence: skilltap does not import them into its personal project
scope, and there is no shared-scope adoption selector.

## Register explicit resources

```bash
skilltap marketplace add <source>
skilltap plugin install <plugin>@<marketplace>
skilltap skill install <source>
```

Marketplace sources and resource identities come from the caller. skilltap
does not browse or search their contents.

A desired resource is identified by its logical ID plus its exact concrete
scope. This scope-bearing resource key is used consistently by inventory,
observations, dependencies, plans, and apply state. The same logical ID can
therefore coexist globally and in multiple projects.

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

Logical resource and component selectors are resolved only across the scopes
selected by the command. Each resulting operation carries the exact scoped
resource key, and the selector's scope must match the operation's scope.

Mutation authority comes only from a verified capability profile compiled into
skilltap for the exact observed harness executable and version. Runtime probes
may preserve or narrow that authority; they never widen it. Unknown harness
versions remain observable but receive no mutation authority.

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

## Delegate the workflow, not the decision

Humans can ask an agent for a high-level outcome instead of assembling a
command sequence. For example:

> Use skilltap to make this project's Codex and Claude plugin setup match, but
> show me anything partial or incompatible before you proceed.

The agent should inspect status, produce a plan, apply only authorized work,
and explain any next action skilltap reports. A request to use skilltap does
not authorize the agent to conceal drift, bypass a required acknowledgment, or
choose which unsupported behavior is acceptable.
