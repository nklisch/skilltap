---
name: patterns
description: "Project code patterns and conventions. Auto-loads when implementing, designing, verifying, or reviewing code. Provides detailed pattern definitions with code examples."
user-invocable: false
allowed-tools: Read, Glob, Grep
---

# Project Patterns Reference

This skill contains detailed pattern documentation for this project.
See individual pattern files for full details with code examples.

Available patterns:
- [validated-wire-contract.md](validated-wire-contract.md) — Serialize domain values through private wire DTOs and rebuild them through validating constructors.
- [validated-string-newtypes.md](validated-string-newtypes.md) — Represent bounded domain text with one validated, serde-aware newtype rather than raw `String`.
- [bounded-native-process-port.md](bounded-native-process-port.md) — Resolve binaries and run direct argument vectors through the bounded runner with explicit limits.
- [isolated-native-fixture-roots.md](isolated-native-fixture-roots.md) — Exercise native and filesystem behavior only inside test-support-owned temporary roots and fake binaries.
