# Isolated native fixture roots

Integration tests allocate test-support-owned temporary homes, configuration
roots, workspaces, and fake native executables instead of touching host state.

## Rationale

This keeps end-to-end tests hermetic and guarantees test runs cannot modify the
operator's real Codex, Claude, HOME, or XDG configuration.

## Examples

- Compiled CLI isolation: `crates/cli/tests/compiled_binary.rs:33-42`
- External tree isolation: `crates/core/tests/runtime_integration.rs:118-130`
- Storage repository isolation: `crates/core/tests/storage_integration.rs:46-55`
- Fake harness isolation: `crates/harnesses/tests/detection.rs:41-45`

## When to Use

- Compiled CLI and harness detection/lifecycle tests.
- Filesystem, storage, publication, and external-tree integration tests.
- Any test involving HOME, XDG paths, executable discovery, or native processes.

## When NOT to Use

- Pure constructors and deterministic domain-unit tests.
- Tests that intentionally target a real external system under an explicit integration harness.

## Common Violations

- Writing to the developer's actual HOME or XDG configuration.
- Launching real `codex` or `claude` binaries in ordinary tests.
- Reusing fixed `/tmp` paths or repository-local mutable fixtures.
- Leaving fake processes or temporary roots unmanaged.

