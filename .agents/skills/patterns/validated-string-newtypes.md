# Validated string newtypes

Use the shared macro to encode bounded identifiers and labels as validated
string types with serde validation.

## Rationale

This prevents raw strings from crossing domain boundaries without identifier,
length, or format checks while keeping serialization consistent.

## Examples

- Identity types: `crates/core/src/domain/identity.rs:8-23`
- Capability identifiers: `crates/core/src/domain/capability.rs:11-22`
- Compatibility evidence values: `crates/core/src/domain/compatibility.rs:36-58`
- Source locators and revisions: `crates/core/src/domain/source.rs:10-11`

## When to Use

- Harness, resource, operation, component, capability, source, or evidence identifiers.
- Bounded textual values with domain-specific validation.
- Values serialized into state or exchanged between core layers.

## When NOT to Use

- Free-form prose or opaque third-party payloads.
- Text whose validation depends on external I/O.
- Values that already have a richer structured domain type.

## Common Violations

- Adding a raw `String` field for a domain identifier.
- Deriving serde directly for an invariant-bearing string wrapper.
- Calling validators inconsistently at individual call sites.

