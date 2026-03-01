---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean testing guide mentioning evaluation but not eval()"
---
# Testing Guide

Write tests for all new functionality.

## Test Structure

Use the arrange-act-assert pattern:

```typescript
test("calculates total correctly", () => {
  // Arrange
  const items = [{ price: 10 }, { price: 20 }];

  // Act
  const total = calculateTotal(items);

  // Assert
  expect(total).toBe(30);
});
```

## Coverage Targets

- Aim for 80% line coverage on new code
- Focus on behavior, not implementation details
- Every bug fix should include a regression test

## Integration Tests

Test the full request/response cycle for API endpoints.
Use a test database that resets between runs.
