# Pattern: View Model Transform

Pure functions map raw domain state (`ProjectState`, `PhaseState`) to view-ready models (`PhaseView`, `SubPhaseView`) with derived display fields and status strings — keeping UI logic out of the domain layer.

## Rationale
The orchestrator's domain types (`PhaseState`, `SubPhaseState`) carry implementation flags like `hasVerification`, `qualityGatePassed`, and `refactorApplied` that are not directly renderable. View models pre-compute derived values (display names, status strings) so UI components receive clean, renderable data with no conditional logic needed at render time.

## Examples

### Example 1: toPhaseViews — top-level state-to-view transform
**File**: `src/tui/views.ts:22`
```typescript
export function toPhaseViews(state: ProjectState): PhaseView[] {
  return state.phases.map(phase => {
    const firstIncompleteIdx = phase.subPhases.findIndex(sp => !sp.verificationPassed);

    return {
      name: phase.name,
      displayName: formatPhaseName(phase.name),             // ← derived display name
      status: phaseStatus(phase, state.currentPhase),       // ← derived status
      subPhases: phase.subPhases.map((sub, idx) => ({
        name: sub.slug,
        displayName: sub.displayName,
        status: subPhaseStatus(sub, phase, state.currentPhase, idx === firstIncompleteIdx),
      })),
    };
  });
}
```

### Example 2: phaseStatus / subPhaseStatus — status derivation helpers
**File**: `src/tui/views.ts:59`
```typescript
function phaseStatus(phase: PhaseState, currentPhase: PhaseState | null): PhaseStatus {
  if (phase.qualityGatePassed) return 'complete';
  if (phase.hasVerification && !phase.verificationPassed) return 'failed';
  if (currentPhase !== null && phase.name === currentPhase.name) return 'active';
  return 'pending';
}

function subPhaseStatus(
  sub: SubPhaseState,
  phase: PhaseState,
  currentPhase: PhaseState | null,
  isFirstIncomplete: boolean,
): PhaseStatus {
  if (sub.verificationPassed) return 'complete';
  if (sub.hasVerification && !sub.verificationPassed) return 'failed';
  const phaseIsActive = currentPhase !== null && phase.name === currentPhase.name;
  if (phaseIsActive && isFirstIncomplete) return 'active';
  return 'pending';
}
```

### Example 3: formatPhaseName — slug-to-display-name transform
**File**: `src/tui/views.ts:39`
```typescript
export function formatPhaseName(slug: string): string {
  const prefix = 'phase-';
  if (!slug.startsWith(prefix)) return slug;

  const rest = slug.slice(prefix.length);          // "1-foundation"
  const dashIdx = rest.indexOf('-');
  if (dashIdx === -1) return rest;

  const num = rest.slice(0, dashIdx);              // "1"
  const words = rest.slice(dashIdx + 1);           // "foundation"
  const titleCased = words
    .split('-')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');                                    // "Foundation"

  return `${num}: ${titleCased}`;                  // "1: Foundation"
}
```

### Example 4: Store consumes the transform
**File**: `src/tui/store.ts:88`
```typescript
setState(state: ProjectState): void {
  setStore({
    phases: toPhaseViews(state),   // ← transform called here, not at render
    currentPhaseName: state.currentPhase?.name ?? null,
    totalPhases: state.phases.length,
    buildPasses: state.buildPasses,
    testsPassing: state.testsPassing,
  });
},
```

## When to Use
- When domain state has boolean flags that need to be collapsed into a small set of display-ready status strings
- When the same derived value (display name, status) would otherwise be recomputed in multiple UI components
- When the domain layer should remain UI-agnostic

## When NOT to Use
- For transformations that require side effects or async I/O — views must be pure
- When the view model would be identical to the domain type — no transform needed

## Common Violations
- Adding UI logic to domain types (`PhaseState`, `ProjectState`) — domain types must not know about `PhaseStatus` or display names
- Computing status inline in UI components instead of using the transform layer — duplicates logic and breaks when domain types change
- Mutating domain state inside the transform — view transforms must be pure functions (no `phase.x = ...`)
