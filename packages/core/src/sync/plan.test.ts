import { describe, expect, test } from "bun:test";
import { planSync } from "./plan";
import type { DriftItem, DriftReport } from "./types";

const ITEM = (kind: DriftItem["kind"], source: string): DriftItem => ({
  kind,
  target: "skill",
  source,
});

describe("planSync", () => {
  test("empty report passes through with ordered=[]", () => {
    const report: DriftReport = { items: [], inSync: true };
    const plan = planSync(report);
    expect(plan.inSync).toBe(true);
    expect(plan.ordered).toEqual([]);
  });

  test("orders items: remove → ref-mismatch → add → lock-stale → lock-missing → lock-orphan", () => {
    const items: DriftItem[] = [
      ITEM("lock-orphan", "a"),
      ITEM("add", "b"),
      ITEM("remove", "c"),
      ITEM("ref-mismatch", "d"),
      ITEM("lock-missing", "e"),
      ITEM("lock-stale", "f"),
    ];
    const plan = planSync({ items, inSync: false });
    expect(plan.ordered.map((i) => i.kind)).toEqual([
      "remove",
      "ref-mismatch",
      "add",
      "lock-stale",
      "lock-missing",
      "lock-orphan",
    ]);
  });

  test("preserves source and target on each item", () => {
    const items: DriftItem[] = [
      { kind: "add", target: "plugin", source: "github:c/d" },
      { kind: "remove", target: "skill", source: "github:n/r" },
    ];
    const plan = planSync({ items, inSync: false });
    expect(plan.ordered[0]).toMatchObject({ kind: "remove", target: "skill", source: "github:n/r" });
    expect(plan.ordered[1]).toMatchObject({ kind: "add", target: "plugin", source: "github:c/d" });
  });

  test("does not mutate the input items array", () => {
    const items: DriftItem[] = [ITEM("add", "x"), ITEM("remove", "y")];
    const original = [...items];
    planSync({ items, inSync: false });
    expect(items).toEqual(original);
  });
});
