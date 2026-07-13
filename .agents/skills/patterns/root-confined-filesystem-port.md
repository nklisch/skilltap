# Root-confined filesystem ports

Access a managed descendant through a validated root plus
`RelativeArtifactPath`, using descriptor-relative no-follow operations.

## Rationale

One confinement boundary prevents ancestor and final-component symlink escapes
while giving planning, publication, rollback, and verification the same path
identity and resource limits.

## Examples

- Bounded read, atomic write, and removal port:
  `crates/core/src/runtime/filesystem/directory_tree.rs:83`
- Managed project catalog planning: `crates/cli/src/application.rs:1335`
- Marketplace catalog reads: `crates/cli/src/application.rs:1717`
- MCP configuration planning: `crates/cli/src/application.rs:1994`
- Apply and rollback consumers: `crates/cli/src/application/execution.rs:261`

The caller supplies a canonical root and validated relative path; the port
opens each ancestor without following links and enforces read limits before
allocating the full payload.

## When to Use

- Files managed beneath a known project or configuration root.
- Planning and mutation that must agree on one confined identity.

## When NOT to Use

- User-selected external source trees, which use the external-tree observer.
- Native harness commands, which use the bounded native-process port.

## Common Violations

- Joining an absolute path and calling ordinary `std::fs` operations.
- Following an ancestor symlink.
- Reading before applying byte limits.
- Publishing outside descriptor-relative atomic replacement.
