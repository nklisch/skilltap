# Revalidated execution ports

Bind validated adapter requests to operation IDs, revalidate the plan under the
configuration lock, and apply only requests that still match the operation.

## Rationale

Planning and mutation remain separate while every adapter closes plan/apply
races at the shared executor boundary. Core owns operations; concrete adapters
retain native and filesystem mechanics.

## Examples

- Managed project files and trees: `crates/cli/src/application/execution.rs:243`
- Managed complete skills: `crates/cli/src/application/execution.rs:589`
- Instruction bridges: `crates/cli/src/application/execution.rs:803`
- Native harness lifecycle: `crates/harnesses/src/lifecycle.rs:517`

Each implementation rejects missing request bindings, verifies the expected
resource/action/surface, and returns a typed `OperationOutcome` through the
common execution contract.

## When to Use

- Any new mutating adapter executed from a skilltap plan.
- Native or filesystem actions whose preconditions can change after planning.

## When NOT to Use

- Pure planning, observation, or compatibility classification.
- Reads that do not publish or remove state.

## Common Violations

- Mutating during planning.
- Accepting a request absent from the plan.
- Skipping action, surface, or state revalidation under the lock.
