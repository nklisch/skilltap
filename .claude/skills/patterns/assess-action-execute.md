# Pattern: Assess-Action-Execute Loop

The orchestration loop follows a strict three-step cycle: assess disk state → plan next action → execute agent. State is never cached between iterations.

## Rationale

The orchestrator must be crash-safe and re-entrant. By re-assessing state from disk at the top of every loop iteration, the system recovers correctly from interruptions, manual marker file edits, or unexpected process exits. The three-phase separation — `assessState` (reads facts), `nextAction` (plans), execution (writes results) — keeps each concern isolated and testable independently.

## Examples

### Example 1: Core orchestration loop
**File**: `src/orchestrator.ts:87-199`
```typescript
while (true) {
  // Step 1: Assess — read all state from disk
  const state = await assessState(projectDir);
  logPhaseStatus(state);

  if (state.isComplete) break;

  // Step 2: Plan — decide what to do next
  const action = await nextAction(state, projectDir, fixAttempts);

  if (action.type === 'complete') break;

  // Step 3: Execute — run the agent
  const prompt = await loadPrompt(action.template, action.context);
  const meta = await loadSkillMeta(action.template);
  const agentModel = action.model ?? meta.model ?? 'sonnet';

  const result = await runAgent(prompt, {
    model: agentModel,
    tools: agentTools,
    workingDirectory: projectDir,
    onToolUse: (event) => log('info', agentSrc, formatToolUse(event), ...),
    onToolResult: (event) => log(...),
    onText: (text) => log('info', agentSrc, text),
  });
}
```

### Example 2: assessState reads all phase state from disk
**File**: `src/state.ts:93-207`
```typescript
export async function assessState(projectDir: string): Promise<ProjectState> {
  const roadmapContent = await readFileOr(roadmapPath, '');
  const buildPasses = await commandSucceeds('bun run build', projectDir);
  const testsPassing = await commandSucceeds('bun run test', projectDir);

  for (const slug of phaseSlugs) {
    const hasVision = await fileExists(join(phaseDir, 'VISION.md'));
    const researchCompleted = await fileExists(join(phaseDir, 'RESEARCH_COMPLETED'));
    // ... checks all marker files ...
    phases.push({ name: slug, hasVision, researchCompleted, ... });
  }
  return { phases, currentPhase, buildPasses, testsPassing, isComplete };
}
```

### Example 3: nextAction maps state to the next required action
**File**: `src/state.ts:209-507`
```typescript
export async function nextAction(
  state: ProjectState,
  projectDir: string,
  fixAttempts = 0,
): Promise<Action> {
  const phase = state.currentPhase;
  if (!phase) return { type: 'complete', ... };

  if (!phase.hasVision)        return { type: 'generate-vision', ... };
  if (!phase.hasPhaseRoadmap)  return { type: 'generate-phase-roadmap', ... };
  if (!phase.researchCompleted) return { type: 'research', ... };
  // ... continues through all lifecycle stages ...
}
```

## When to Use

- Any new lifecycle stage added to the orchestration loop follows this three-step structure
- `assessState` and `nextAction` are always called in sequence — never skip re-assessment

## When NOT to Use

- Short-lived one-shot operations outside the main loop (e.g., setup commands) don't need this cycle

## Common Violations

- Caching `state` across loop iterations — breaks crash recovery and makes manual marker edits ineffective
- Embedding decision logic in the execution step — keep planning (`nextAction`) separate from execution
- Skipping `assessState` by passing state forward — always re-read from disk to guarantee consistency
