---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean React best practices skill"
---
# React Patterns

Best practices for React component development.

## Component Structure

- Use functional components with hooks
- Extract custom hooks for reusable logic
- Keep components under 200 lines
- Colocate tests with components

## State Management

Prefer local state for UI-only concerns. Use context for theme, auth status,
and locale. Reach for external state libraries only when prop drilling becomes
painful across 3+ levels.

## Performance

- Memoize expensive computations with `useMemo`
- Wrap callbacks in `useCallback` when passing to memoized children
- Use `React.lazy` for route-level code splitting
