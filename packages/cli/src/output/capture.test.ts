import { describe, expect, test } from "bun:test";
import { createCaptureOutput } from "./capture";

describe("createCaptureOutput", () => {
  test("records info event", () => {
    const out = createCaptureOutput();
    out.info("hello world");
    expect(out.events).toContainEqual({ kind: "info", message: "hello world" });
  });

  test("records warn event with optional hint", () => {
    const out = createCaptureOutput();
    out.warn("heads up", "fix it");
    expect(out.events).toContainEqual({
      kind: "warn",
      message: "heads up",
      hint: "fix it",
    });
  });

  test("records warn event without hint", () => {
    const out = createCaptureOutput();
    out.warn("heads up");
    expect(out.events).toContainEqual({
      kind: "warn",
      message: "heads up",
      hint: undefined,
    });
  });

  test("records error event", () => {
    const out = createCaptureOutput();
    out.error("something broke", "do this");
    expect(out.events).toContainEqual({
      kind: "error",
      message: "something broke",
      hint: "do this",
    });
  });

  test("records success event", () => {
    const out = createCaptureOutput();
    out.success("Installed X");
    expect(out.events).toContainEqual({
      kind: "success",
      message: "Installed X",
    });
  });

  test("records block event with default stderr stream", () => {
    const out = createCaptureOutput();
    out.block(["line1", "line2"]);
    expect(out.events).toContainEqual({
      kind: "block",
      lines: ["line1", "line2"],
      stream: "stderr",
    });
  });

  test("records block event with stdout stream", () => {
    const out = createCaptureOutput();
    out.block(["line1"], { stream: "stdout" });
    expect(out.events).toContainEqual({
      kind: "block",
      lines: ["line1"],
      stream: "stdout",
    });
  });

  test("records json event", () => {
    const out = createCaptureOutput();
    out.json({ kind: "install:done", records: ["foo"] });
    expect(out.events).toContainEqual({
      kind: "json",
      event: { kind: "install:done", records: ["foo"] },
    });
  });

  test("records progress lifecycle events", () => {
    const out = createCaptureOutput();
    const p = out.progress("Loading");
    p.update("halfway");
    p.succeed("done loading");

    expect(out.events[0]).toEqual({ kind: "progress:start", label: "Loading" });
    expect(out.events[1]).toEqual({
      kind: "progress:update",
      label: "Loading",
      message: "halfway",
    });
    expect(out.events[2]).toEqual({
      kind: "progress:done",
      label: "Loading",
      message: "done loading",
    });
  });

  test("records progress fail event", () => {
    const out = createCaptureOutput();
    const p = out.progress("Cloning");
    p.fail("network error");

    expect(out.events[1]).toEqual({
      kind: "progress:fail",
      label: "Cloning",
      message: "network error",
    });
  });

  test("records raw event", () => {
    const out = createCaptureOutput();
    out.raw("verbatim text");
    expect(out.events).toContainEqual({ kind: "raw", text: "verbatim text" });
  });

  test("events array preserves call order", () => {
    const out = createCaptureOutput();
    out.info("first");
    out.success("second");
    out.error("third");
    expect(out.events[0]).toMatchObject({ kind: "info", message: "first" });
    expect(out.events[1]).toMatchObject({ kind: "success", message: "second" });
    expect(out.events[2]).toMatchObject({ kind: "error", message: "third" });
  });

  test("mode defaults to 'plain'", () => {
    const out = createCaptureOutput();
    expect(out.mode).toBe("plain");
  });

  test("mode can be overridden", () => {
    const out = createCaptureOutput("json");
    expect(out.mode).toBe("json");
  });

  test("progress pause/resume are no-ops (no crash, no events)", () => {
    const out = createCaptureOutput();
    const p = out.progress("Working");
    const countBefore = out.events.length;
    p.pause();
    p.resume();
    expect(out.events.length).toBe(countBefore);
  });

  test("multiple progress handles are independent", () => {
    const out = createCaptureOutput();
    const p1 = out.progress("Task A");
    const p2 = out.progress("Task B");
    p1.succeed("A done");
    p2.fail("B failed");

    const starts = out.events.filter((e) => e.kind === "progress:start");
    expect(starts).toHaveLength(2);
    const doneA = out.events.find((e) => e.kind === "progress:done");
    expect(doneA).toMatchObject({ label: "Task A" });
    const failB = out.events.find((e) => e.kind === "progress:fail");
    expect(failB).toMatchObject({ label: "Task B" });
  });
});
