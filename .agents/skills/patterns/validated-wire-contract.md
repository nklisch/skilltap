# Validated wire contract boundary

Persisted domain types serialize through private `*Wire` DTOs and deserialize
through validating constructors.

## Rationale

This keeps schema/version checks, cross-field invariants, duplicate detection,
and unknown-field rejection at every persistence boundary instead of allowing
serde to construct invalid domain values.

## Examples

- Config documents: `crates/core/src/storage/config.rs:210-220,330-350`
- Inventory graphs: `crates/core/src/storage/inventory.rs:8-21,145-161`
- Operation contracts: `crates/core/src/domain/operation.rs:504-537,843-876`
- State documents: `crates/core/src/storage/state.rs:749-789`

## When to Use

- Persisted configuration, inventory, state, plans, operations, or domain graphs.
- Serialized values with schema or cross-field invariants.
- Formats where unknown fields must be rejected.

## When NOT to Use

- Opaque external payloads that are not domain state.
- Flat values whose invariant is already represented by a validated newtype.
- Deliberately forward-compatible extension envelopes.

## Common Violations

- Deriving `Deserialize` directly on an invariant-carrying domain struct.
- Accepting wire fields without routing through the domain constructor.
- Omitting `deny_unknown_fields` on a closed persisted contract.

