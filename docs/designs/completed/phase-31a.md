# Phase 31a — v2.0 Security Policy (Additive)

## Goal

A v2.0 policy module that composes a single `EffectivePolicyV2` from `ConfigV2`, CLI flags, and environment. Trust-list short-circuit. No human/agent split, no presets, no overrides table. Purely additive — `core/src/policy.ts` (v1.0) is left untouched and still drives v1.0 install/update/remove paths.

## Scope decision: split Phase 31

The original Phase 31 plan (handoff notes in PROGRESS.md) listed 10 work items including HTTP adapter removal, install reader cutover, and sync apply. That's a destructive migration with high blast radius. Splitting:

- **Phase 31a (this design)**: New v2 policy module + tests. Additive only.
- **Phase 31b (next session)**: HTTP registry adapter removal + tap config cleanup.
- **Phase 31c (later session)**: Install/update/remove cutover to v2 state + sync apply.

Splitting respects context limits and keeps each step verifiable in isolation. The user's directive ("simplify [security] (though still allow for it to be used)") is most directly served by 31a — that's the headline v2.0 simplification.

## Design

### `EffectivePolicyV2`

```typescript
export type EffectivePolicyV2 = {
  // Behavior
  yes: boolean;        // from --yes or --agent (agent implies yes)
  agent: boolean;      // resolved from flag / env / config

  // Placement
  scope: "global" | "project" | "";
  also: string[];

  // Security
  scanMode: "semantic" | "static" | "none";
  onWarn: "prompt" | "fail" | "install";
  skipScan: boolean;   // --skip-scan flag

  // Per-source: true after trust-list match → caller skips scan entirely.
  trusted: boolean;
};
```

No `requireScan` field. v2.0 model: if you want scan required, set `scan ≠ "none"`. If a user passes `--skip-scan` against trusted-list-only configuration, that's their choice.

### `CliFlagsV2`

```typescript
export type CliFlagsV2 = {
  agent?: boolean;
  noAgent?: boolean;
  yes?: boolean;
  noYes?: boolean;
  strict?: boolean;
  noStrict?: boolean;
  skipScan?: boolean;
  deep?: boolean;        // forces scanMode = "semantic" for this invocation
  project?: boolean;
  global?: boolean;
};
```

### Resolution rules

`composeV2(config, flags, env?)`:

1. **agent**: Resolve from precedence `--no-agent` > `--agent` > `SKILLTAP_AGENT=1` > `config.agent.default`. Error if `config.agent.block === true` and `agent` resolves to `true`.
2. **yes**: `--no-yes` (off) > `--yes` (on) > `agent` (on) > config `defaults.yes` if such key existed (it doesn't in v2 — agent is the only way to default-yes).
3. **scanMode**: `--deep` (semantic) > `config.security.scan`.
4. **onWarn**: `--strict` (fail) > `--no-strict` (revert to config) > `config.security.on_warn`.
5. **scope**: `--project` (project) > `--global` (global) > `config.defaults.scope`.
6. **also**: `config.defaults.also` (CLI `--also` is appended by callers — not modelled here).
7. **skipScan**: `flags.skipScan ?? false`.
8. **trusted**: `false` (set by `composeV2ForSource`).

### `composeV2ForSource(config, flags, source)`

Takes a `{ tapName?: string; sourceUrl: string }`. Computes the base policy, then applies the trust list:

- If any pattern matches `tapName` or `sourceUrl` → `trusted = true`, `scanMode = "none"`.
- Else pass through.

### Trust-glob matcher

```typescript
export function trustMatches(pattern: string, target: string): boolean
export function isTrusted(trust: string[], source: {tapName?: string; sourceUrl: string}): boolean
```

Pattern syntax: literal characters + `*` wildcard (matches any run of chars). All regex specials escaped except `*`. Anchored at start and end (^…$). Examples:

- `"home"` matches `tapName === "home"`.
- `"github.com/corp/*"` matches sourceUrl `https://github.com/corp/foo` (or `github.com/corp/foo` if no scheme).
- `"npm:@corp/*"` matches sourceUrl `npm:@corp/code-review@1.0`.

We don't normalize URLs (no protocol-stripping, no query-string handling) — patterns must match the exact source string as it appears in install commands. Documented for users; tests cover both URL forms.

## Implementation Units

### Unit 1 — `core/src/policy-v2/types.ts`

Just the type definitions (`EffectivePolicyV2`, `CliFlagsV2`, `EnvV2`, `SourceForPolicy`).

### Unit 2 — `core/src/policy-v2/trust-glob.ts`

`trustMatches(pattern, target)` and `isTrusted(trust, source)` pure functions.

### Unit 3 — `core/src/policy-v2/compose.ts`

`composeV2(config, flags, env?)` and `composeV2ForSource(config, flags, env, source)`. Returns `Result<EffectivePolicyV2, UserError>` to surface the agent-block error.

### Unit 4 — `core/src/policy-v2/index.ts`

Barrel.

### Unit 5 — `core/src/index.ts` update

Add `export * from "./policy-v2"`.

### Unit 6 — Tests

- `policy-v2/trust-glob.test.ts` — exact, wildcard, mixed, no-match, both target dimensions.
- `policy-v2/compose.test.ts` — every precedence rule, every edge case, agent-block error.

## Verification

```bash
bun test packages/core/src/policy-v2/
```

Plus a regression check that v1.0 policy.ts and existing tests still work (they should — additive-only):

```bash
bun test packages/core/src/policy.test.ts
```

## Out of Scope

- HTTP registry removal — Phase 31b.
- Install/update/remove cutover to v2 state + policy — Phase 31c.
- Sync apply — Phase 31c.
- v1.0 schema deletion — Phase 31c.
- `--deep` flag wiring into install/update CLI — happens when callers cut over.
