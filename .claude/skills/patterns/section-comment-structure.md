# Pattern: Section-Comment Structure

`// ── Name ──` separators using em-dash characters divide every source file into named sections.

## Rationale

With no IDE-generated region markers or code folding, the em-dash section comments provide visual structure that makes any file scannable at a glance. The consistent three-section layout (public types, public API, private helpers) means developers always know where to find things.

## Examples

### Example 1: cli.ts sections
**File**: `src/cli.ts:6,36,171`

Three sections: `// ── Public types ──` (interfaces), `// ── Public API ──` (exported functions), `// ── Private helpers ──` (internal utilities).

### Example 2: log.ts sections
**File**: `src/log.ts:6,16,20,47`

Four sections: public types, public state (the singleton), public API, private helpers.

### Example 3: All source files
**File**: `src/state.ts`, `src/orchestrator.ts`, `src/prompts.ts`, `src/index.ts`

Every source file follows the same pattern with at minimum public types, public API, and private helpers sections.

## When to Use

- Every new TypeScript source file in the project
- When adding a new logical section to an existing file
- The section comment goes on its own line with no code
