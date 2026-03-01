# Pattern: Make-Factory Test

`make*` factory functions create fully-specified default objects for tests, accepting optional partial overrides.

## Rationale

Tests need valid, complete objects (like `ProjectState` or `Action`) to exercise orchestration logic. Instead of inlining large object literals in every test, factory functions provide sensible defaults that tests override only for the property under examination. The `make*` naming convention signals test-only construction; the `overrides: Partial<T> = {}` signature enables targeted mutations without noise. Shared factories live in `tests/helpers/factories.ts`; local one-off factories live inline in the test file.

## Examples

### Example 1: Shared state factories in helpers
**File**: `tests/helpers/factories.ts:8`
```typescript
export function makePhaseState(overrides: Partial<PhaseState> = {}): PhaseState {
  return {
    name: 'phase-1-foundation',
    hasVision: false,
    hasPhaseRoadmap: false,
    researchCompleted: false,
    subPhases: [],
    hasDesign: false,
    isImplemented: false,
    hasVerification: false,
    verificationPassed: false,
    hasRefactorPlan: false,
    refactorApplied: false,
    integrationPassed: false,
    patternsExtracted: false,
    qualityGatePassed: false,
    ...overrides,
  };
}

export function makeProjectState(overrides: Partial<ProjectState> = {}): ProjectState {
  return {
    phases: [],
    currentPhase: null,
    buildPasses: false,
    testsPassing: false,
    isComplete: false,
    ...overrides,
  };
}
```

### Example 2: Parametric action factory with spread overrides (local)
**File**: `tests/unit/orchestrator.test.ts:82`
```typescript
function makeAction(overrides: Partial<Action> = {}): Action {
  return {
    type: 'generate-vision',
    phase: 'phase-1-test',
    template: 'roadmap-to-vision',
    model: 'sonnet',
    tools: [],
    context: {},
    ...overrides,
  };
}

// Usage:
mockNextAction.mockResolvedValue(makeAction({ type: 'complete' }));
mockNextAction.mockResolvedValue(makeAction({ type: 'implement', template: 'implement' }));
```

### Example 3: Using shared factories in state tests
**File**: `tests/unit/state.test.ts:328`
```typescript
const phase = makePhaseState({
  name: 'phase-1-test',
  hasVision: true,
  hasPhaseRoadmap: true,
  researchCompleted: true,
});
const state = makeProjectState({ phases: [phase], currentPhase: phase });

const action = await nextAction(state, tempDir, 0);
expect(action.type).toBe('generate-design');
```

## When to Use

- Creating test objects for complex interfaces with many required fields
- When multiple tests need the same base object but test different property states
- Any test requiring a `ProjectState`, `Action`, or `AgentResult` — use factories, not inline literals
- Factories shared across 2+ test files belong in `tests/helpers/factories.ts`; local-only factories can be inline

## When NOT to Use

- Simple primitive values or small plain objects — inline them directly
- When every test needs a completely different object — factories don't save noise

## Common Violations

- Inlining large complete object literals in each test — creates duplication and makes intent opaque
- Factories that don't accept `overrides` — callers must mutate after construction instead of declaring intent at call site
- Duplicating shared factories in multiple test files instead of putting them in `tests/helpers/factories.ts`
