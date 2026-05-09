import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { Dashboard } from "./Dashboard";
import { initialDashboardState } from "../state/dashboard";

describe("Dashboard", () => {
  test("renders all tab labels", () => {
    const { lastFrame } = render(
      <Dashboard state={initialDashboardState} dispatch={() => {}} data={null} />,
    );
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Installed");
    expect(frame).toContain("Taps");
    expect(frame).toContain("Updates");
    expect(frame).toContain("Drift");
  });

  test("renders footer with key hints", () => {
    const { lastFrame } = render(
      <Dashboard state={initialDashboardState} dispatch={() => {}} data={null} />,
    );
    const frame = lastFrame() ?? "";
    expect(frame).toContain("quit");
    expect(frame).toContain("install");
  });

  test("renders loading state", () => {
    const state = { ...initialDashboardState, loading: true };
    const { lastFrame } = render(
      <Dashboard state={state} dispatch={() => {}} data={null} />,
    );
    expect(lastFrame()).toContain("Loading");
  });

  test("renders data items when provided", () => {
    const data = [{ name: "my-skill", source: "github.com/user/repo" }];
    const { lastFrame } = render(
      <Dashboard state={initialDashboardState} dispatch={() => {}} data={data} />,
    );
    expect(lastFrame()).toContain("my-skill");
  });

  test("renders empty state when no data", () => {
    const { lastFrame } = render(
      <Dashboard state={initialDashboardState} dispatch={() => {}} data={[]} />,
    );
    expect(lastFrame()).toContain("nothing here");
  });
});
