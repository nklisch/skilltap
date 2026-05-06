# Phase 28 — Project Manifest + Lockfile

## Goal

`skilltap.toml` and `skilltap.lock` can be loaded, saved, and round-tripped. Publishable plugins inside a repo can be discovered. No behavior is wired in yet — Phase 29 (sync) is the first consumer.

## Decisions

### Defer 28.2 (manifest entry resolution) to Phase 29

Roadmap 28.2 says "Implement `manifest/resolve.ts` — resolve manifest entries to `ResolvedDeps[]` with source adapter dispatch." But the only consumer of resolution is Phase 29's sync engine. Resolution logic without a consumer is dead code; better to land it together with the consumer that exercises it.

### File locations and atomicity

- `skilltap.toml` lives at project root (next to `package.json`/`bunfig.toml`).
- `skilltap.lock` lives next to it.
- Both writes go through `tmp + rename` for atomicity.
- Reads return defaults when files don't exist (no error). Callers test for presence via `manifestExists()`.

### TOML emit shape

`smol-toml.stringify` doesn't preserve comments, but skilltap.toml authored by users will round-trip fine. We'll emit a minimal default header comment when *creating* a manifest from scratch, but plain stringify when *updating* an existing one (preserving structure as best we can).

## Implementation Units

### Unit 1 — `core/src/manifest/load.ts`

```typescript
export async function manifestExists(projectRoot: string): Promise<boolean>
export async function loadManifest(projectRoot: string): Promise<Result<ProjectManifest>>
```

`loadManifest` returns the schema-defaulted empty manifest if the file doesn't exist (caller can decide what to do). Parses TOML, validates with Zod.

### Unit 2 — `core/src/manifest/save.ts`

```typescript
export async function saveManifest(
  projectRoot: string,
  manifest: ProjectManifest,
): Promise<Result<void>>
```

Writes via `tmp + rename`. Stringifies the manifest verbatim — no comment template (the file is meant to be edited by users with `skilltap install`/`remove`).

### Unit 3 — `core/src/manifest/lockfile.ts`

```typescript
export async function lockfileExists(projectRoot: string): Promise<boolean>
export async function loadLockfile(projectRoot: string): Promise<Result<Lockfile>>
export async function saveLockfile(
  projectRoot: string,
  lockfile: Lockfile,
): Promise<Result<void>>
```

Same shape as manifest I/O. Default lockfile is `{ version: 1, skill: [], plugin: [] }`.

### Unit 4 — `core/src/manifest/publish.ts`

```typescript
export async function discoverPublishablePlugins(repoRoot: string): Promise<{
  publishable: PluginManifestV2[];
  rejected: { path: string; reason: string }[];
}>
```

Reads every `.skilltap/*.toml` file. Parses each; valid manifests with `publish = true` go into `publishable`. Invalid manifests or `publish = false` go into `rejected` (with reason). `.skilltap/` not present → `publishable: [], rejected: []`.

### Unit 5 — `core/src/manifest/index.ts` (new barrel)

Re-exports everything in `manifest/`. The existing schema/range exports in `src/index.ts` get replaced with a single `export * from "./manifest"`.

### Unit 6 — Tests

- `manifest/load.test.ts` + `save.test.ts` (or combined `io.test.ts`) — round-trip empty + populated, file-not-found, invalid TOML.
- `manifest/lockfile.test.ts` — same.
- `manifest/publish.test.ts` — discovers valid and rejects invalid; `.skilltap/` absent; `publish: false` rejected.

## Verification

```bash
bun test packages/core/src/manifest/
```

## Out of Scope

- Writing manifest from `install`/`remove` — Phase 30 (when install supports `skilltap.toml`).
- Resolving manifest entries to clone-able sources — Phase 29.
- Detecting drift — Phase 29.
