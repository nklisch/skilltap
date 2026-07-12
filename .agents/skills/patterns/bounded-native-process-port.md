# Bounded native process request port

Resolve the executable first, then invoke native commands through
`NativeProcessRequest` and `NativeProcessRunner` with direct arguments and
explicit limits.

## Rationale

This centralizes executable identity checks, argument handling, environment
control, working-directory scoping, output limits, and process failure mapping
for every harness interaction.

## Examples

- Git revision resolution: `crates/harnesses/src/update_resolution.rs:56-81`
- Harness lifecycle execution: `crates/harnesses/src/lifecycle.rs:86-104`
- Harness detection: `crates/harnesses/src/lib.rs:99-119`
- Daemon service probe: `crates/cli/src/entrypoint.rs:707-724`

## When to Use

- Harness lifecycle, detection, update, Git, or service-manager commands.
- Native processes whose output or runtime must be bounded.
- Operations requiring executable identity revalidation.

## When NOT to Use

- Pure in-process domain computation.
- Reading regular files through the filesystem abstraction.
- Test-only helper subprocesses that do not exercise the product boundary.

## Common Violations

- Building shell command strings.
- Calling `std::process::Command` from an adapter instead of the bounded runner.
- Passing unresolved executable paths.
- Omitting process limits or allowing ambient environment inheritance.

