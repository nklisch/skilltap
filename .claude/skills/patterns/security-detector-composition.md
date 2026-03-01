# Pattern: Security Detector Composition

Independent pure detector functions each return `PatternMatch[]`, composed in a for-loop inside the scanner orchestrator.

## Rationale

Each detector is a single-concern pure function — easy to test, add, or reorder. The orchestrator (`scanStatic`) composes them by iterating over an array of detectors and merging their matches into a flat `StaticWarning[]`. This avoids a monolithic scan function and lets new detectors be added by appending to the array.

## Examples

### Example 1: PatternMatch type
**File**: `packages/core/src/security/patterns.ts:4`
```typescript
export type PatternMatch = {
  line: number | [number, number];  // single line or [start, end] range
  category: string;
  raw: string;        // escaped representation of the offending text
  visible?: string;   // invisible chars stripped (from out-of-character)
  decoded?: string;   // obfuscated content decoded (base64, hex, etc.)
};
```

### Example 2: Detector function signature
**File**: `packages/core/src/security/patterns.ts:25`
```typescript
export function detectInvisibleUnicode(content: string): PatternMatch[] {
  const findings = hasConfusables({ sourceText: content, detailed: true }) as ...;
  if (!findings || findings.length === 0) return [];
  // ... map findings to PatternMatch[]
}
```

### Example 3: Orchestrator composition
**File**: `packages/core/src/security/static.ts:177`
```typescript
const detectors = [
  detectInvisibleUnicode,
  detectHiddenHtmlCss,
  detectMarkdownHiding,
  detectObfuscation,
  detectSuspiciousUrls,
  detectDangerousPatterns,
  detectTagInjection,
];

for (const detect of detectors) {
  const matches = detect(content);
  for (const m of matches) {
    warnings.push({ file: relPath, ...m });  // PatternMatch → StaticWarning
  }
}
```

### Example 4: StaticWarning extends PatternMatch with file field
**File**: `packages/core/src/security/static.ts:14`
```typescript
export type StaticWarning = {
  file: string;
  line: number | [number, number];
  category: string;
  raw: string;
  visible?: string;
  decoded?: string;
};
```

## When to Use

- Adding a new security check: write a new `detectX(content: string): PatternMatch[]` function in `patterns.ts`, then append it to the `detectors` array in `static.ts`
- Each detector should have a single responsibility (one attack vector)
- Provide `visible` when the issue involves invisible characters; provide `decoded` when the issue involves obfuscated payloads

## When NOT to Use

- Don't add non-text checks (binary detection, file size) as detector functions — those run earlier as pre-filters in `scanStatic`, before the detector loop
- Don't combine multiple unrelated concerns in one detector function

## Common Violations

- Returning errors (throwing) from a detector instead of returning `[]` — detectors are pure and should never fail
- Checking file extensions or binary content inside a detector — pre-filters handle that before detectors run
- Adding a detector without appending it to the `detectors` array in `static.ts` — it won't run
