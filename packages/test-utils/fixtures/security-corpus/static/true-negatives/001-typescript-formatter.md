---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean TypeScript formatting skill with no security issues"
---
# TypeScript Formatter

Formats TypeScript files according to project conventions.

## Rules

- Use 2-space indentation
- Prefer `const` over `let`
- Use arrow functions for callbacks
- Sort imports alphabetically

## Examples

```typescript
// Before
const result = items.filter(function(item) {
    return item.active;
});

// After
const result = items.filter((item) => item.active);
```
