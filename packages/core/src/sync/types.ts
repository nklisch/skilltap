export type DriftKind =
  | "add"
  | "remove"
  | "ref-mismatch"
  | "lock-stale"
  | "lock-missing"
  | "lock-orphan";

export type DriftTarget = "skill" | "plugin" | "mcp";

export interface DriftItem {
  kind: DriftKind;
  target: DriftTarget;
  /** Manifest key / source identifier (e.g. "github:n/commit-helper"). */
  source: string;
  /** Declared in manifest. */
  declared?: { ref?: string; range?: string };
  /** Currently installed (from state.json). */
  installed?: { ref?: string; sha?: string };
  /** Recorded in lockfile. */
  locked?: { ref?: string; sha?: string; range?: string };
  /** Human-readable description (set per-kind by drift.ts). */
  reason?: string;
}

export interface DriftReport {
  items: DriftItem[];
  inSync: boolean;
}

export interface SyncPlan extends DriftReport {
  /**
   * `items` reordered for execution: removals first, then ref changes,
   * then adds, then bookkeeping (lock-* categories).
   */
  ordered: DriftItem[];
}
