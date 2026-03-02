---
name: extract-patterns
description: "Discover and document reusable code patterns from the codebase. Use this skill after implementing a feature, completing a milestone, or whenever you want to capture conventions and recurring structures so future work stays consistent. Also useful when onboarding to a new codebase to understand its idioms."
---

# Extract Patterns

Analyze the codebase to discover reusable code structures, shared abstractions, and recurring architectural approaches — then document them as pattern files that future work can reference.

## Why This Matters

Codebases develop idioms over time — ways of handling errors, structuring modules, writing tests, composing components. When these patterns are documented, every future change stays consistent without the developer needing to reverse-engineer conventions from scattered examples. These pattern files become a shared vocabulary for both humans and agents working in the codebase.

## What Counts as a Pattern

A pattern is a **recurring code structure** — the same architectural approach used in 2+ places. Focus on:

- **Reusable abstractions** — base classes, shared utilities, composition patterns
- **Module structure** — how services/components/handlers are organized
- **Error handling** — Result types, try-catch-fallback, error boundaries, recovery strategies
- **Data flow** — how data moves between layers, state management approaches
- **Testing infrastructure** — fixture factories, mock patterns, test helpers, setup conventions

Patterns are NOT coding style (naming conventions, formatting, import ordering). Style rules belong in CLAUDE.md.

## Outputs

This skill produces three artifacts:

1. **`.claude/skills/patterns/{slug}.md`** — Individual pattern files with rationale and concrete code examples. These auto-load as skill references when future work touches related code.

2. **`.claude/rules/patterns.md`** — A dense one-line-per-pattern index. This loads into every conversation's context automatically, giving quick pointers to full pattern details.

3. **`.claude/skills/patterns/SKILL.md`** — Updated listing of available pattern files.

## Workflow

### Step 1: Read Existing Context

Before scanning, read what's already documented to avoid duplicating work:

- `.claude/skills/patterns/*.md` — existing pattern files (if any)
- `.claude/rules/patterns.md` — existing index (if it exists)
- `CLAUDE.md` — project conventions (to avoid overlapping with style rules)

### Step 2: Scan the Codebase

Explore the project's source directories to find recurring structures. Scan across multiple dimensions:

- **Shared Abstractions & Utilities** — Reusable functions, base classes, common helpers, shared types used across multiple modules. Note which modules use each.
- **Architectural Patterns** — How modules are organized, how services/components compose, how data flows between layers, how configuration is handled, how async operations and errors propagate.
- **Testing Infrastructure** — Shared fixtures, test utilities, factory functions, mocking approaches, assertion helpers.

For each candidate, note file paths and line numbers of concrete examples.

For larger codebases, consider using the Agent tool with explore subagents to scan these dimensions in parallel.

### Step 3: Cross-Check and Filter

After scanning, read 3-4 key files to verify findings. A pattern needs:

- **2+ concrete examples** in the codebase (ideally 3+)
- A clear rationale for why the codebase does it this way
- Enough specificity that someone could follow the pattern in new code

Drop anything with only one example — that's a one-off, not a pattern yet. Drop anything that's really a style preference rather than a structural approach.

### Step 4: Write Pattern Files

For each confirmed pattern, write `.claude/skills/patterns/{slug}.md`:

```markdown
# Pattern: {Pattern Name}

{One-line description of what this pattern does}

## Rationale

{Why this pattern exists — what problem it solves, what tradeoffs it makes}

## Examples

### Example 1: {description}
**File**: `path/to/file.ts:42`
```language
// concrete code from the codebase
```

### Example 2: {description}
**File**: `path/to/other.ts:18`
```language
// concrete code from the codebase
```

## When to Use

- {circumstance where this pattern applies}

## When NOT to Use

- {circumstance where a different approach is better}

## Common Violations

- {what going against this pattern looks like and why it causes problems}
```

### Step 5: Write the Dense Index

Create or update `.claude/rules/patterns.md`:

- One line per pattern: `- **{name}**: {terse description} → [patterns/{slug}.md]`
- Keep under 30 lines total
- Order by frequency of relevance (most commonly needed patterns first)

### Step 6: Update the Patterns Skill

Update `.claude/skills/patterns/SKILL.md` to list all current pattern files in its "Available Patterns" section. Include the slug filename and a short description for each.

## Handling Existing Patterns

When patterns already exist from a previous extraction:

- **Confirm** existing patterns still hold — verify examples are still in the codebase
- **Update** stale patterns whose examples have moved or changed
- **Remove** patterns whose examples have been deleted from the codebase
- **Add** new patterns discovered since the last extraction
- **Flag contradictions** where new code conflicts with a documented pattern
