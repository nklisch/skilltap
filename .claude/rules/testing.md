# Testing Rules

## What must be tested

**New CLI output** → validate with a subprocess or PTY test. If the output is produced by clack (`log.*`, spinner), use `runInteractive` (PTY) — it renders fully in TTY mode. If output is from `process.stdout.write` or `successLine`/`errorLine`, `runSkilltap` (pipe) works.

**New commands** → at least one happy-path subprocess test (`runSkilltap` or `runInteractive`). Verify exit code, key output text, and any filesystem side-effects (installed files, symlinks, config changes).

**New flags** → exercise the flag explicitly in a test. For boolean flags that change behavior, test both the on and off states. Don't assume a flag is covered because the underlying logic is tested.

**New core functions** → unit tests in the same package. Use `Result` assertion pattern: `expect(result.ok).toBe(true)` then `if (!result.ok) return` guard before accessing `.value`.

## Test selection

| Scenario | Use |
|---|---|
| Pure logic, no I/O | `bun:test` unit test directly |
| Filesystem / git ops | Integration test with `makeTmpDir` + fixture repos |
| CLI flags, exit codes, stdout/stderr text | `runSkilltap` subprocess |
| Clack prompts, spinner output, PTY-rendered UI | `runInteractive` PTY session |

## Rules

- `runInteractive` is for clack-rendered output. `runSkilltap` runs in pipe mode — `isTTY` is false, spinner state is unreliable. Don't use `runSkilltap` to assert on spinner/log output.
- Never use `--skip-scan` as a crutch to avoid testing the scan path. If the feature involves scanning, the test should run the scan.
- Flag parsing: avoid `--no-*` flag names — mri intercepts them as negations of the base flag, making `args["no-*"]` unusable. Use explicit names (`--quiet`, `--force`, etc.) instead.
- Tests that create temp dirs must clean up in `finally` or `afterEach`. Never rely on test runner cleanup.
- `bun test` runs synchronously (foreground). Never run it with `run_in_background: true` — it spawns dozens of processes and starves the machine.
