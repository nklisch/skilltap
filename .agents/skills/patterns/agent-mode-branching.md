# Pattern: Output Interface (replaces agent-mode-branching)

> **This pattern has been superseded.** The explicit `runAgentMode()` / `runInteractiveMode()` split and `policy.agentMode` no longer exist. See **[output-interface.md](output-interface.md)** for the current pattern.

## What changed

Previously, CLI commands forked early based on `policy.agentMode`:
```typescript
// OLD — no longer exists
if (policy.agentMode) return runAgentMode(args, config, policy);
return runInteractiveMode(args, config, policy);
```

The `EffectivePolicy` type no longer has an `agentMode` field. There is no `runAgentMode()` or `runInteractiveMode()` in the codebase.

## Current approach

Commands call `setupOutput(args)` once and receive an `Output` handle. The mode is resolved automatically via `pickMode()`:
- `--json` → json mode (structured events, no human text)
- TTY stdout → tty mode (clack spinners, ANSI colors)
- Piped stdout / CI / agent → plain mode (clean text lines)

All output goes through `out.info()`, `out.success()`, `out.error()`, `out.progress()`, and `out.json()`. No explicit agent/interactive branching at the command level.

See [output-interface.md](output-interface.md) for full details and examples.
