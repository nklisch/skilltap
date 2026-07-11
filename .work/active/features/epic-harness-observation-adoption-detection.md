---
id: epic-harness-observation-adoption-detection
kind: feature
stage: done
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-contracts, epic-harness-observation-adoption-runtime]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Harness Detection and Capability Profiles

Build the concrete adapter registry, Codex/Claude installation and opaque
version detection, compiled verified scope-aware profile selection, and
read-only probe narrowing. Profiles are the mutation allowlist; help text is
negative evidence only, JSON success must pass its parser contract, and unknown
versions never gain mutation authority. Provide reusable scripted fake-native
fixtures for missing/unexecutable binaries, output/timeout failures, known and
unknown versions, probe drift, and executable replacement. Do not yet observe
resources or expose CLI commands.

## Design

Detection is a harness-neutral boundary between the runtime ports and later
resource observers. A registry owns only the supported Codex and Claude
adapters, each adapter declares its configured binary, version arguments,
parser contract, and profile catalogue. Detection returns an installation
envelope with reachability, canonical executable identity, opaque native
version, and safe boundary errors; it never infers mutation authority from
help text, filesystem presence, or an unparsed success response.

Version parsing is strict and bounded. A known version selects one compiled
profile whose observation and mutation capabilities are scope-aware. An
unknown but parseable version remains observable with an explicit unknown
version authority and no mutation capabilities. Unsupported, malformed, or
ambiguous version output is an observation finding rather than a guessed
profile. Profile definitions are immutable code-owned data and are the single
source of truth for later mutation allowlists.

Optional read-only probes run only after installation reachability and strict
JSON success. Probe results can narrow a compiled profile independently for
global and project scope; they may never widen capabilities, introduce a new
capability, or turn an unknown version into mutation authority. Probe drift,
duplicate fields, unexpected schemas, timeout, output overflow, executable
replacement, and scope mismatch are safe typed findings. Probe requests use
the bounded process runtime with explicit environment and no inherited state.

The test-support layer adds deterministic fake-native modes for missing and
unexecutable binaries, known/unknown versions, valid and malformed JSON,
probe narrowing/drift, output floods, hangs, and identity replacement. The
feature stops at installation/profile evidence: resource observation and CLI
composition remain downstream features.

## Design decisions

- **Profile authority**: only a verified compiled profile grants mutation
  capabilities; unknown versions remain observe-only.
- **Probe semantics**: probes can narrow baseline support per scope but never
  widen it or supply authority absent from the compiled profile.
- **Failure isolation**: one harness detection failure is a typed installation
  result and does not prevent sibling harness detection.
- **Native output**: all version/probe output passes the strict JSON boundary;
  help text is negative evidence only and is never parsed as capability truth.

## Implementation units

1. `epic-harness-observation-adoption-detection-fixtures` — add deterministic
   fake-native detection/probe modes and replacement/failure fixtures —
   depends on `[]`.
2. `epic-harness-observation-adoption-detection-registry` — implement the
   Codex/Claude adapter registry and safe installation/version boundary —
   depends on `[epic-harness-observation-adoption-runtime,
   epic-harness-observation-adoption-contracts,
   epic-harness-observation-adoption-detection-fixtures]`.
3. `epic-harness-observation-adoption-detection-profiles` — implement compiled
   scope-aware profile selection and unknown-version observe-only authority —
   depends on `[epic-harness-observation-adoption-detection-registry]`.
4. `epic-harness-observation-adoption-detection-probes` — implement strict
   bounded read-only probe execution and monotonic capability narrowing —
   depends on `[epic-harness-observation-adoption-detection-profiles]`.
5. `epic-harness-observation-adoption-detection-integration` — verify sibling
   isolation, parser/error safety, deterministic detection, replacement
   handling, and native Linux/macOS behavior — depends on
   `[epic-harness-observation-adoption-detection-fixtures,
   epic-harness-observation-adoption-detection-registry,
   epic-harness-observation-adoption-detection-profiles,
   epic-harness-observation-adoption-detection-probes]`.

## Acceptance criteria

- Codex and Claude installations resolve through configured binaries with
  canonical identities, opaque versions, safe failures, and no resource
  observation or CLI writes.
- Known versions select only compiled profiles; unknown versions expose
  observation evidence but no mutation capabilities.
- Probes use strict bounded JSON and can only narrow declared capabilities per
  scope; malformed, drifted, timed-out, overflowing, or replaced executions
  remain safe findings.
- Fake-native fixtures cover missing/unexecutable binaries, known/unknown
  versions, parser failures, output/deadline limits, probe drift, and identity
  replacement without relying on timing or inherited environment.
- All focused and workspace verification remains locked, deterministic, and
  safe across Linux and native macOS behavior jobs.

## Implementation

- Completed all five detection stories: deterministic fake-native fixtures,
  Codex/Claude registry detection, compiled profile selection, monotonic JSON
  probes, and sibling-isolated end-to-end tests.
- `skilltap-harnesses` now composes the runtime ports without resource
  observation or mutation APIs. Known v3 profiles grant only compiled
  capabilities; unknown versions remain observe-only.

## Verification

- Harness detection tests (6), fixture tests (16), workspace Clippy, and the
  locked workspace suite pass. Detection errors and probe payloads remain
  closed and secret-safe.

## Review

- Aggregate review approved after the locked workspace ladder: 211 core tests,
  six harness detection tests, 16 fixture tests, warnings-denied Clippy, and
  the existing CLI/integration suites all pass. Detection remains read-only,
  sibling-isolated, and authority-safe for unknown versions.
