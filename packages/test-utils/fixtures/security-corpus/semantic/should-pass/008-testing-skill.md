# Testing Best Practices

Guides writing effective tests for TypeScript projects.

## Test Structure

Use the Arrange-Act-Assert pattern:

```typescript
test('adds two numbers', () => {
  // Arrange
  const a = 2, b = 3;

  // Act
  const result = add(a, b);

  // Assert
  expect(result).toBe(5);
});
```

## What to Test

- Pure functions: all input/output combinations
- API handlers: success, error, edge cases
- Components: rendering, user interactions, state changes
- Integration: database queries, external API calls (mocked)

## What Not to Test

- Implementation details (private methods, internal state)
- Third-party library behavior
- Trivial getters/setters

## Running Tests

```bash
bun test                    # Run all tests
bun test --watch            # Watch mode
bun test path/to/file.ts   # Single file
bun test --coverage         # With coverage report
```

## Mocking

Use `mock()` for external dependencies:

```typescript
import { mock } from 'bun:test';
const fetchMock = mock(() => Promise.resolve({ ok: true }));
```
