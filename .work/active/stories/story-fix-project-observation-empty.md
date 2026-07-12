---
id: story-fix-project-observation-empty
kind: story
stage: review
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Treat absent project harness roots as an empty observation

## Symptom

An all-scopes plan or sync reports observation failures for a clean project
that has no harness-specific directories yet.

## Root cause

The shared canonical-root observer treats every empty root set as unavailable,
even though project roots are optional until a resource is materialized.

## Fix approach

Allow project adapters to report an empty, healthy observation while preserving
strict first-use behavior for required global roots.

## Regression test

Harness observation tests and the isolated all-scopes compiled-binary scenario
cover an empty project with enabled harnesses.

## Implementation notes

- Canonical project observers now return a healthy empty observation when all
  optional project roots are absent; required global first-use behavior remains
  strict.
- Added a harness detection regression for an empty project.
