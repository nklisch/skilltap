# Design: Phase 45 — Migrate Command Verification + Polish

## Overview

Phase 45 is the smallest remaining phase. The migrate command core (`packages/core/src/migrate/`) was built incrementally across Phase 40 and Phase 27. What's already in place:

- ✅ `detect.ts` — finds v0.x state markers (installed.json/plugins.json, config-v1 keys).
- ✅ `config-v1.ts` — translates `[security.human]`/`[security.agent]`/`[agent-mode]`/`[[security.overrides]]` to flat `[security]`. Stricter wins on per-mode collapse. HTTP taps detected and aborted.
- ✅ `run.ts:runMigrate()` — orchestrates: detect → translate config → migrate state per scope → backup originals to `*.v1.bak` → write new files.
- ✅ Tests cover the translator (236 lines) and orchestrator (150 lines).

What Phase 45 adds:

1. **Doctor post-migrate verification.** After successful migrate, run `runDoctor()` and surface results. If checks fail, show them but don't abort (migration already succeeded; doctor is informational).
2. **Manifest verification.** After migrate, parse `skilltap.toml` (if present) to confirm it's still parseable. Surface a warning if not.
3. **End-to-end integration test.** Fixture-based test that exercises all v0.x markers + verifies state.json shape + .v1.bak files + doctor pass.

This phase is intentionally scoped small. Phase 46 (polish + release) is where the user-facing release happens.

## Acceptance Criteria

- `runMigrate()` returns a `MigrationReport` that includes the doctor-post-migrate result.
- `MigrationReport.doctorReport?: DoctorResult` field added (optional — only present after a non-no-op migration).
- The CLI `migrate` command surfaces doctor findings: green checkmark if all pass; warning lines if any check fires.
- `runMigrate()` validates `skilltap.toml` parse on completion and adds a warning to `MigrationReport.warnings` if it can't parse.
- One new end-to-end test covers a comprehensive v0.x fixture: installed.json + plugins.json + v1 config + v1 manifest. Asserts on state.json shape, config flat-block, .v1.bak presence, doctor pass.
- Full `bun test` passes.

## Implementation Units

### Unit 1: Doctor post-migrate integration

**Files**:
- `packages/core/src/migrate/run.ts` — extend `runMigrate()` to call `runDoctor()` after successful state writes.
- `packages/cli/src/commands/migrate.ts` — surface doctor findings in command output.

```typescript
// run.ts — extend MigrationReport and runMigrate body

import { runDoctor, type DoctorResult } from "../doctor";

export interface MigrationReport {
  alreadyMigrated: boolean;
  scopes: ("global" | "project")[];
  changes: MigrationFileChange;
  warnings: string[];
  doctorReport?: DoctorResult;   // NEW — only present when migration ran (not on no-op)
}

// In runMigrate(), after the existing migrate body succeeds, before returning ok():
let doctorReport: DoctorResult | undefined;
if (!alreadyMigrated) {
  doctorReport = await runDoctor({ projectRoot: options.projectRoot });
}

return ok({
  alreadyMigrated: false,
  scopes,
  changes: { written, renamed },
  warnings,
  doctorReport,
});
```

```typescript
// commands/migrate.ts — render doctor findings

if (result.value.doctorReport) {
  const failed = result.value.doctorReport.checks.filter((c) => c.status === "fail");
  const warned = result.value.doctorReport.checks.filter((c) => c.status === "warn");
  if (failed.length === 0 && warned.length === 0) {
    out.success("Doctor: all checks passed.");
  } else {
    if (failed.length > 0) {
      out.error(`Doctor: ${failed.length} check(s) failed`, "run `skilltap doctor` for details.");
    }
    if (warned.length > 0) {
      out.warn(`Doctor: ${warned.length} check(s) warning`, "run `skilltap doctor` for details.");
    }
  }
}
```

**Implementation Notes**:
- `runDoctor` exists from earlier phases. It accepts `{ projectRoot? }` and returns `DoctorResult`.
- Don't fail the migrate if doctor warns/fails — migration already succeeded; doctor is post-migration verification.
- For the JSON output mode, include `doctorReport` in the JSON event.

**Acceptance Criteria**:
- [ ] `MigrationReport.doctorReport` is set on non-no-op migrations.
- [ ] CLI surfaces "Doctor: all checks passed" when clean.
- [ ] CLI surfaces "Doctor: N check(s) failed/warning" with hint to run `skilltap doctor`.
- [ ] Existing migrate tests still pass.

---

### Unit 2: Manifest verification

**File**: `packages/core/src/migrate/run.ts`

After the state and config writes, parse `skilltap.toml` if it exists at `projectRoot`. Append to `warnings` if it doesn't parse cleanly.

```typescript
// At end of runMigrate body, before doctor:
if (options.projectRoot) {
  const manifestPath = join(options.projectRoot, "skilltap.toml");
  const manifestFile = Bun.file(manifestPath);
  if (await manifestFile.exists()) {
    try {
      const text = await manifestFile.text();
      parse(text);   // smol-toml parse — throws on bad TOML
    } catch (e) {
      warnings.push(`skilltap.toml at ${manifestPath} did not parse cleanly: ${e}`);
    }
  }
}
```

**Acceptance Criteria**:
- [ ] When `skilltap.toml` parses cleanly, no warning added.
- [ ] When `skilltap.toml` is malformed, a warning is added to `MigrationReport.warnings`.
- [ ] Migration still succeeds even when manifest parse fails (warning, not fatal).

---

### Unit 3: End-to-end integration test

**File**: `packages/core/src/migrate/run.test.ts` — extend with a comprehensive fixture test.

```typescript
test("end-to-end migration: full v0.x setup translates cleanly", async () => {
  const env = await createTestEnv();
  try {
    // Set up a complete v0.x fixture:
    //   ~/.config/skilltap/installed.json (v1, with one skill)
    //   ~/.config/skilltap/plugins.json (v1, with one plugin)
    //   ~/.config/skilltap/config.toml (v1: [security.human], [security.agent], [agent-mode], [[security.overrides]])
    await writeV1Installed(env, [{ name: "test-skill", repo: "github:owner/repo", scope: "global" }]);
    await writeV1Plugins(env, [{ name: "test-plugin", scope: "global", components: [] }]);
    await writeV1Config(env, {
      "security.human": { scan: "static", on_warn: "prompt" },
      "security.agent": { scan: "static", on_warn: "fail", require_scan: true },
      "agent-mode": { enabled: true, scope: "global" },
      "security.overrides": [{ match: "trusted-tap", kind: "tap", preset: "none" }],
    });

    const result = await runMigrate({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // Verify state.json
    const stateText = await Bun.file(`${env.configDir}/skilltap/state.json`).text();
    const state = JSON.parse(stateText);
    expect(state.version).toBe(2);
    expect(state.skills.map((s) => s.name)).toContain("test-skill");
    expect(state.plugins.map((p) => p.name)).toContain("test-plugin");

    // Verify config.toml is flat (no per-mode blocks, no [agent-mode])
    const configText = await Bun.file(`${env.configDir}/skilltap/config.toml`).text();
    expect(configText).not.toContain("[security.human]");
    expect(configText).not.toContain("[security.agent]");
    expect(configText).not.toContain("[agent-mode]");
    expect(configText).toContain("[security]");

    // Verify .v1.bak files exist
    expect(await Bun.file(`${env.configDir}/skilltap/installed.json.v1.bak`).exists()).toBe(true);
    expect(await Bun.file(`${env.configDir}/skilltap/plugins.json.v1.bak`).exists()).toBe(true);
    expect(await Bun.file(`${env.configDir}/skilltap/config.toml.v1.bak`).exists()).toBe(true);

    // Verify doctor was run and passed (or surfaced known warnings)
    expect(result.value.doctorReport).toBeDefined();
  } finally {
    await env.cleanup();
  }
});
```

Helpers `writeV1Installed`, `writeV1Plugins`, `writeV1Config` are local test helpers that write the fixture files in v0.x format using the `InstalledJsonSchema` / `PluginsJsonSchema` / TOML.

**Acceptance Criteria**:
- [ ] Test passes with the full v0.x fixture.
- [ ] All assertions on state.json, config.toml, and `.v1.bak` files succeed.
- [ ] doctorReport is populated.

---

## Implementation Order

Single agent handles all three units in one commit.

1. Unit 2 (manifest verification) — smallest, isolated.
2. Unit 1 (doctor post-migrate) — adds the `doctorReport` field; CLI surfaces.
3. Unit 3 (e2e test) — verifies both new units + the existing migrate body.

## Verification

```bash
bun run build
bun test packages/core/src/migrate/
bun test packages/cli/src/commands/migrate.test.ts
bun test
```

## Risks

| Risk | Mitigation |
|---|---|
| `runDoctor()` is slow (it does file I/O across all checks) | Acceptable — migrate is a one-time operation. |
| Doctor check failure post-migrate confuses user — was the migration broken or was the env broken? | Make it clear in CLI output: "Doctor: N check(s) failed — your environment may have other issues unrelated to migration. Run `skilltap doctor` for details." |
| End-to-end test is brittle (depends on createTestEnv path conventions) | Use existing `@skilltap/test-utils` fixtures; reuse patterns from `migrate/run.test.ts`. |
