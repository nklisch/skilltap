import { describe, expect, test } from "bun:test";
import { FooterBar } from "./footer";

// ── Mock stream ───────────────────────────────────────────────────────────────

function makeMockStream(isTTY = true, rows = 24) {
  const written: string[] = [];
  const handlers: Record<string, Array<() => void>> = {};

  const stream = {
    isTTY,
    rows,
    columns: 80,
    write(chunk: any, _enc?: any, _cb?: any): boolean {
      written.push(typeof chunk === "string" ? chunk : (chunk?.toString?.() ?? ""));
      return true;
    },
    on(event: string, handler: () => void) {
      (handlers[event] ??= []).push(handler);
    },
    off(event: string, handler: () => void) {
      if (handlers[event]) handlers[event] = handlers[event].filter((h) => h !== handler);
    },
    emit(event: string) {
      handlers[event]?.forEach((h) => h());
    },
  };

  return { stream, written, handlers };
}

// ── rows getter / setter ──────────────────────────────────────────────────────

describe("FooterBar rows property", () => {
  test("getter returns realRows - 1 while active", () => {
    const { stream } = makeMockStream(true, 30);
    const bar = new FooterBar(stream as any);
    bar.open();
    expect(stream.rows).toBe(29);
    bar.close();
  });

  test("getter clamps to minimum of 3", () => {
    const { stream } = makeMockStream(true, 3);
    const bar = new FooterBar(stream as any);
    bar.open();
    expect(stream.rows).toBe(3); // max(3 - 1, 3) = 3
    bar.close();
  });

  test("setter does not throw (Bun TTY _refreshSize compat)", () => {
    const { stream } = makeMockStream(true, 30);
    const bar = new FooterBar(stream as any);
    bar.open();
    // This is what Bun's internal _refreshSize does — it must not throw.
    expect(() => {
      (stream as any).rows = 40;
    }).not.toThrow();
    bar.close();
  });

  test("setter updates _snapshotRows (real stdout has no own rows descriptor)", () => {
    // On real Bun process.stdout, `rows` is a prototype getter so
    // getOwnPropertyDescriptor returns undefined — _realRowsValue() falls back
    // to _snapshotRows. Simulate that by deleting the own `rows` property.
    const { stream } = makeMockStream(true, 30);
    delete (stream as any).rows; // no own property → _snapshotRows is the source
    const bar = new FooterBar(stream as any);
    bar.open();
    (stream as any).rows = 40; // triggers our setter
    expect(stream.rows).toBe(39); // 40 - FOOTER_HEIGHT(1)
    bar.close();
  });

  test("rows restored to original value after close", () => {
    const { stream } = makeMockStream(true, 24);
    const bar = new FooterBar(stream as any);
    bar.open();
    expect(stream.rows).toBe(23);
    bar.close();
    expect(stream.rows).toBe(24);
  });
});

// ── open / close lifecycle ────────────────────────────────────────────────────

describe("FooterBar open / close", () => {
  test("isActive is false before open", () => {
    const { stream } = makeMockStream();
    const bar = new FooterBar(stream as any);
    expect(bar.isActive).toBe(false);
  });

  test("isActive is true after open on TTY", () => {
    const { stream } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    expect(bar.isActive).toBe(true);
    bar.close();
  });

  test("does not activate on non-TTY stream", () => {
    const { stream } = makeMockStream(false);
    const bar = new FooterBar(stream as any);
    bar.open();
    expect(bar.isActive).toBe(false);
  });

  test("isActive is false after close", () => {
    const { stream } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.close();
    expect(bar.isActive).toBe(false);
  });

  test("calling open twice is a no-op", () => {
    const { stream, written } = makeMockStream(true, 24);
    const bar = new FooterBar(stream as any);
    bar.open();
    const writtenAfterFirst = written.length;
    bar.open(); // should be ignored
    expect(written.length).toBe(writtenAfterFirst);
    bar.close();
  });

  test("write interception is installed on open and removed on close", () => {
    const { stream } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    const originalWrite = stream.write;
    bar.open();
    // Interceptor replaces the function reference
    expect(stream.write).not.toBe(originalWrite);
    bar.close();
    // After close the interceptor is gone — a plain write no longer triggers
    // cursor-save sequences (the original write just appends to `written`)
    expect(bar.isActive).toBe(false);
  });

  test("resize listener is removed after close", () => {
    const { stream, handlers } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    expect(handlers["resize"]?.length).toBe(1);
    bar.close();
    expect(handlers["resize"]?.length ?? 0).toBe(0);
  });
});

// ── write interceptor ─────────────────────────────────────────────────────────

describe("FooterBar write interceptor", () => {
  test("write interception passes through to original stream", () => {
    const { stream, written } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("select");
    const before = written.length;
    stream.write("hello");
    // At least our "hello" was written (interceptor calls through)
    const allWritten = written.slice(before).join("");
    expect(allWritten).toContain("hello");
    bar.close();
  });

  test("erase-down sequence triggers footer repaint", () => {
    const { stream, written } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("select");
    const before = written.length;
    stream.write("\x1b[J"); // erase-down — clack emits this on every render
    const repaint = written.slice(before).join("");
    // Repaint uses cursor-save (\x1b7) and positions to footer row
    expect(repaint).toContain("\x1b7");
    bar.close();
  });

  test("non-erase write does not trigger extra repaint", () => {
    const { stream, written } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("select");
    const before = written.length;
    stream.write("plain text");
    // Only the "plain text" itself — no cursor-save sequence added
    const extra = written.slice(before).join("");
    expect(extra).toBe("plain text");
    bar.close();
  });
});

// ── setContext ────────────────────────────────────────────────────────────────

describe("FooterBar setContext", () => {
  test("setContext while inactive does not throw", () => {
    const { stream } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    expect(() => bar.setContext("select")).not.toThrow();
  });

  test("setContext to same value does not repaint", () => {
    const { stream, written } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("select");
    const before = written.length;
    bar.setContext("select"); // same — should not repaint
    expect(written.length).toBe(before);
    bar.close();
  });

  test("setContext to different value triggers repaint", () => {
    const { stream, written } = makeMockStream(true);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("select");
    const before = written.length;
    bar.setContext("multiselect");
    expect(written.length).toBeGreaterThan(before);
    bar.close();
  });
});

// ── resize handling ───────────────────────────────────────────────────────────

describe("FooterBar resize handling", () => {
  test("resize event triggers footer repaint", () => {
    const { stream, written } = makeMockStream(true, 24);
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("confirm");
    const before = written.length;
    stream.emit("resize");
    expect(written.length).toBeGreaterThan(before);
    bar.close();
  });

  test("setter + resize event: repaint uses updated row count (no own descriptor)", () => {
    // Simulate real Bun stdout where rows is a prototype getter (no own property).
    // _realRowsValue() then uses _snapshotRows, which the setter updates.
    const { stream, written } = makeMockStream(true, 24);
    delete (stream as any).rows;
    const bar = new FooterBar(stream as any);
    bar.open();
    bar.setContext("confirm");
    // Simulate Bun _refreshSize: assign new rows, then emit resize
    (stream as any).rows = 50; // triggers setter → _snapshotRows = 50
    stream.emit("resize");
    // The repaint should position footer at row 50
    const repaint = written.join("");
    expect(repaint).toContain("\x1b[50;1H");
    bar.close();
  });
});
