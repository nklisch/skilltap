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
