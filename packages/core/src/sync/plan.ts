import type { DriftKind, DriftReport, SyncPlan } from "./types";

const ORDER: DriftKind[] = [
  "remove",
  "ref-mismatch",
  "add",
  "lock-stale",
  "lock-missing",
  "lock-orphan",
];

export function planSync(report: DriftReport): SyncPlan {
  const ordered = [...report.items].sort(
    (a, b) => ORDER.indexOf(a.kind) - ORDER.indexOf(b.kind),
  );
  return {
    ...report,
    ordered,
  };
}
