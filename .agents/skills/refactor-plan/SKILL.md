---
name: refactor-plan
description: "Plan safe, incremental refactoring work. Use this when code has accumulated duplication, when structure needs improvement, when abstractions are missing, or when you want to restructure code before adding new features. Guides analysis of refactoring opportunities and produces a plan via plan mode."
---

# Refactor Plan

Analyze the codebase for structural improvements, then enter plan mode with a concrete, step-by-step refactoring plan. Each step should be small enough to apply, test, and commit independently — minimizing risk while steadily improving code quality.

## Why Step-by-Step

Refactoring is safest when each change is small and verifiable. A plan with 5 discrete steps that each pass tests is far less risky than one giant "improve everything" change. If step 3 breaks something, you roll back step 3, not the whole effort. This also makes code review easier — each step has a clear before/after.

## What to Look For

Focus on high-value structural improvements:

- **Duplicate Logic** — Code that does the same or very similar things in multiple places. Look for duplicated error handling, data transformations, validation, API call patterns, setup/teardown sequences.
- **Missing Abstractions** — Multiple modules implementing similar logic that could be a shared utility, base class, or common helper.
- **Pattern Violations** — Code that deviates from established patterns (check `.claude/skills/patterns/` if pattern files exist). Inconsistent approaches to the same problem across modules.
- **Tangled Responsibilities** — Modules doing too many things, or logic living in the wrong layer.
- **Complex Conditionals / Deep Nesting** — Multi-level if/else chains, hard-to-parse boolean expressions, inverted logic (double negatives), or conditions that could be extracted into a named predicate or helper function for clarity. Look for early-return opportunities that flatten nested blocks, findIndex/filter predicates written as multi-step if-chains, and nested loops inside conditionals.

Deprioritize purely aesthetic changes (renaming, reordering, reformatting) — they create churn without measurable improvement.

## Workflow

### Step 1: Understand Context

Read the relevant source files and any existing documentation:

- `.claude/skills/patterns/*.md` — established patterns (if they exist)
- `CLAUDE.md` — project conventions
- The code area targeted for refactoring

### Step 2: Find Refactoring Opportunities

Scan the codebase for the categories above. For each opportunity, note:

- Which files are involved (with line references)
- What the current structure looks like
- Why it's a problem (duplication count, inconsistency, etc.)

For larger codebases, consider using the Agent tool with explore subagents to scan for duplicates, missing abstractions, and pattern violations in parallel.

### Step 3: Categorize by Value

Group findings into:

- **High value** — Reduces duplication, extracts shared abstractions, consolidates divergent approaches
- **Medium value** — Improves consistency, aligns with established patterns
- **Low value** — Minor structural improvements, mostly cosmetic

### Step 4: Enter Plan Mode

Use `EnterPlanMode` to present the refactoring plan. Structure each step as:

- **What** — The specific refactor (extract utility, deduplicate handler, align with pattern)
- **Why** — The concrete problem it solves (N duplicated blocks, inconsistent error handling across M files)
- **Files** — Which files are touched
- **Risk** — Low/Medium/High based on how many callers are affected
- **Verification** — How to confirm the step worked (beyond "tests pass" — e.g., "no remaining duplicates of X", "all handlers now use the shared utility")

Order steps by dependency — later steps can depend on earlier ones being complete.

## What Makes a Good Refactor Step

Each step should be:

- **Self-contained** — Can be applied, built, tested, and committed on its own
- **Ordered by dependency** — Later steps can depend on earlier ones
- **Concrete** — Before/after states are specific enough to act on without ambiguity
- **Verified** — Has specific checks beyond "tests pass"

Avoid:

- Steps too large to verify in isolation
- Combining unrelated refactors in one step (if one breaks, the other gets rolled back too)
- Changing public APIs without a migration path
- Refactors with unclear benefit relative to their risk
