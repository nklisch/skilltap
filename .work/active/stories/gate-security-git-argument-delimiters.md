---
id: gate-security-git-argument-delimiters
kind: story
stage: drafting
tags: [security]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: security
created: 2026-07-12
updated: 2026-07-12
---

# Delimit and validate external Git arguments

## Severity

Medium

## Domain

Input validation and command boundaries

## Location

- `crates/harnesses/src/update_resolution.rs:61-72`
- `crates/cli/src/application.rs:4163-4168,4217-4227`
- `crates/harnesses/src/lifecycle.rs:278-282,316-320`

## Evidence

External locator and requested-revision strings are passed positionally to Git
and native lifecycle commands without `--` delimiters; boundary validation does
not reject option-like values beginning with `-`.

## Remediation direction

Use `--` delimiters wherever supported, reject option-like locators and
revisions at the input boundary, and add adversarial direct-argv tests.
