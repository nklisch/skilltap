# Pattern: fg Color Helper

All TUI component color props are wrapped through the `fg()` helper from `format.ts`, which returns `undefined` when `NO_COLOR` is set — eliminating inline ternaries throughout JSX.

## Rationale

Ink/opentui renders pass `undefined` as the "no color" value for `fg` props. Without a helper, every component would need `noColor() ? undefined : color` at every call site. The `fg()` helper centralizes this logic and keeps JSX clean. Combined with `TUI_COLORS` constants, it enforces a palette-based color system with automatic colorless mode.

## Examples

### Example 1: Central helper definition
**File**: `src/tui/format.ts:56-62`
```typescript
/**
 * Returns color string when color output is enabled, undefined when NO_COLOR is set.
 * Shorthand for the `noColor() ? undefined : color` ternary used in JSX fg props.
 */
export function fg(color: string): string | undefined {
  return noColor() ? undefined : color;
}
```

### Example 2: Used with TUI_COLORS palette in header
**File**: `src/tui/header.tsx:47-51`
```tsx
<box flexDirection="row" height={1} width="100%">
  <text content={projectName} fg={fg(TUI_COLORS.info)} />
  <text content={`  ${phaseProgress()}`} />
  <text content={agentLabel()} fg={fg(TUI_COLORS.warning)} />
  <text content={spinnerChar()} fg={fg(TUI_COLORS.warning)} />
  <text content={elapsed()} fg={fg(TUI_COLORS.muted)} />
</box>
```

### Example 3: Dynamic color selection in log component
**File**: `src/tui/log.tsx:88-91`
```tsx
const icon = logIcon(entry);
const iconColor = icon !== ''
  ? fg(TUI_COLORS.muted)
  : undefined;
```

### Example 4: Status-conditional color in sidebar
**File**: `src/tui/sidebar.tsx:52-64`
```tsx
const STATUS_COLORS: Record<PhaseStatus, string> = {
  complete: TUI_COLORS.success,
  active: TUI_COLORS.warning,
  failed: TUI_COLORS.error,
  pending: TUI_COLORS.muted,
};
// ...
<text content={icon} fg={fg(STATUS_COLORS[phase.status])} />
```

## When to Use

- Any JSX `fg` prop in TUI components
- Any place that would otherwise write `noColor() ? undefined : someColor`
- When building new TUI components that render colored text

## When NOT to Use

- Non-JSX contexts (pure string formatting for stdout uses `noColor()` directly with ANSI escape codes)
- Background/border colors (same pattern applies, but `fg()` is specifically named for foreground — consider extracting a separate helper if needed)

## Common Violations

- Inlining `noColor() ? undefined : color` in JSX instead of calling `fg()` — makes colorless mode harder to verify
- Hardcoding hex colors directly in JSX without `TUI_COLORS` — bypasses the central palette and makes auditing difficult
- Forgetting `fg()` entirely and passing a raw hex string — renders incorrectly in NO_COLOR environments
