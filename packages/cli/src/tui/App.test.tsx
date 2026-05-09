import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { App } from "./App";
import type { AppContext } from "./state/types";

function makeContext(overrides: Partial<AppContext> = {}): AppContext {
  return {
    dispatchInstall: async () => ({ ok: true }),
    dispatchToggle: async () => ({ ok: true }),
    dispatchAdopt: async () => ({ ok: true }),
    dispatchSync: async () => ({ ok: true }),
    loadDashboardData: async () => [],
    loadFindResults: async () => [],
    loadToggleComponents: async () => [],
    loadToggleNames: async () => [],
    loadAdoptCandidates: async () => [],
    ...overrides,
  };
}

describe("App", () => {
  test("renders dashboard by default", () => {
    const { lastFrame } = render(<App context={makeContext()} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Installed");
    expect(frame).toContain("Taps");
  });

  test("renders dashboard when initialScreen=dashboard", () => {
    const { lastFrame } = render(
      <App initialScreen="dashboard" context={makeContext()} />,
    );
    expect(lastFrame()).toContain("Installed");
  });

  test("renders find screen when initialScreen=find", () => {
    const { lastFrame } = render(
      <App initialScreen="find" context={makeContext()} />,
    );
    expect(lastFrame()).toContain("Search");
  });

  test("renders toggle screen when initialScreen=toggle", () => {
    const { lastFrame } = render(
      <App initialScreen="toggle" context={makeContext()} />,
    );
    const frame = lastFrame() ?? "";
    expect(frame).toContain("skill");
    expect(frame).toContain("plugin");
  });

  test("renders adopt screen when initialScreen=adopt", () => {
    const { lastFrame } = render(
      <App initialScreen="adopt" context={makeContext()} />,
    );
    expect(lastFrame()).toContain("Adopt");
  });
});

// ─── Unit 3.17 — TUI bug fixes ────────────────────────────────────────────────

const ARROW_UP = "[A";
const ARROW_DOWN = "[B";

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe("Dashboard tab keys (Unit 3.17 bug 1)", () => {
  test("pressing 1-4 on Dashboard switches the active tab without re-mounting", async () => {
    const calls: string[] = [];
    const { stdin, lastFrame } = render(
      <App
        context={makeContext({
          loadDashboardData: async (tab) => {
            calls.push(tab);
            return [];
          },
        })}
      />,
    );

    // Wait for initial mount + first dashboard load.
    await sleep(20);
    expect(calls[0]).toBe("installed");

    // Press "2" → should dispatch dashboard:tab to "taps", not navigate.
    stdin.write("2");
    await sleep(20);
    const frame2 = lastFrame() ?? "";
    expect(frame2).toContain("Taps");
    expect(calls).toContain("taps");

    stdin.write("3");
    await sleep(20);
    expect(calls).toContain("updates");

    stdin.write("4");
    await sleep(20);
    expect(calls).toContain("drift");

    stdin.write("1");
    await sleep(20);
    // After 1, we should have re-loaded "installed" (already in calls; check
    // we hit it at least twice since the first time was the initial mount).
    expect(calls.filter((c) => c === "installed").length).toBeGreaterThanOrEqual(2);
  });
});

describe("Adopt Enter handler (Unit 3.17 bug 2)", () => {
  test("pressing Enter on a focused Adopt candidate calls dispatchAdopt", async () => {
    const adoptCalls: Array<{ kind: string; name: string; mode: string }> = [];
    const { stdin } = render(
      <App
        initialScreen="adopt"
        context={makeContext({
          loadAdoptCandidates: async () => [
            {
              kind: "skill",
              name: "alpha",
              source: "/tmp/alpha",
            },
            {
              kind: "plugin",
              name: "beta",
              source: "claude-code",
            },
          ],
          dispatchAdopt: async (kind, name, mode) => {
            adoptCalls.push({ kind, name, mode });
            return { ok: true };
          },
        })}
      />,
    );

    // Wait for candidates to load.
    await sleep(50);

    // focusIndex starts at 0 (first candidate). Press Enter.
    stdin.write("\r");
    await sleep(20);

    expect(adoptCalls).toHaveLength(1);
    expect(adoptCalls[0]).toEqual({
      kind: "skill",
      name: "alpha",
      mode: "track-in-place",
    });

    // Move down and press Enter again.
    stdin.write(ARROW_DOWN);
    await sleep(10);
    stdin.write("\r");
    await sleep(20);

    expect(adoptCalls).toHaveLength(2);
    expect(adoptCalls[1]).toEqual({
      kind: "plugin",
      name: "beta",
      mode: "track-in-place",
    });
  });
});

describe("Toggle name step (Unit 3.17 bug 3)", () => {
  test("entering the name step calls loadToggleNames and renders the list", async () => {
    const namesCalls: string[] = [];
    const { stdin, lastFrame } = render(
      <App
        initialScreen="toggle"
        context={makeContext({
          loadToggleNames: async (type) => {
            namesCalls.push(type);
            if (type === "skill") return ["alpha", "beta", "gamma"];
            return [];
          },
        })}
      />,
    );

    // Initial frame: type step (skill highlighted because focusIndex=0).
    await sleep(10);
    let frame = lastFrame() ?? "";
    expect(frame).toContain("Select type");

    // Press Enter to confirm "skill".
    stdin.write("\r");
    // Allow the effect to call loadToggleNames + re-render.
    await sleep(50);

    expect(namesCalls).toEqual(["skill"]);
    frame = lastFrame() ?? "";
    expect(frame).toContain("Select skill to toggle");
    expect(frame).toContain("alpha");
    expect(frame).toContain("beta");
    expect(frame).toContain("gamma");
    // No longer stuck on the (loading…) placeholder.
    expect(frame).not.toContain("(loading…)");
  });

  test("up/down on the type step adjust focusIndex", async () => {
    const { stdin, lastFrame } = render(
      <App initialScreen="toggle" context={makeContext()} />,
    );
    await sleep(10);

    stdin.write(ARROW_DOWN);
    await sleep(10);
    stdin.write(ARROW_DOWN);
    await sleep(10);
    // Focus is now on "mcp" — press Enter, the name step should ask for mcp.
    const namesCalls: string[] = [];
    // Re-render with a context that records the name request — easier path:
    // verify by checking the title after Enter.
    stdin.write("\r");
    await sleep(30);

    const frame = lastFrame() ?? "";
    expect(frame).toContain("Select mcp to toggle");
    void namesCalls;
  });
});
