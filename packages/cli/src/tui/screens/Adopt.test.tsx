import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import { initialAdoptState } from "../state/adopt";
import { Adopt } from "./Adopt";

describe("Adopt", () => {
  test("renders heading", () => {
    const { lastFrame } = render(
      <Adopt state={initialAdoptState} dispatch={() => {}} />,
    );
    expect(lastFrame()).toContain("Adopt");
  });

  test("renders empty state message", () => {
    const { lastFrame } = render(
      <Adopt state={initialAdoptState} dispatch={() => {}} />,
    );
    expect(lastFrame()).toContain("no adoptable items");
  });

  test("renders loading state", () => {
    const state = { ...initialAdoptState, loading: true };
    const { lastFrame } = render(<Adopt state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("Scanning");
  });

  test("renders candidates with kind and name", () => {
    const state = {
      ...initialAdoptState,
      candidates: [
        { kind: "skill" as const, name: "foo", source: "/path/foo" },
        { kind: "plugin" as const, name: "bar", source: "marketplace" },
      ],
    };
    const { lastFrame } = render(<Adopt state={state} dispatch={() => {}} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("skill: foo");
    expect(frame).toContain("plugin: bar");
  });

  test("renders mode hint per item", () => {
    const state = {
      ...initialAdoptState,
      candidates: [{ kind: "skill" as const, name: "foo", source: "/path" }],
      perItemMode: new Map([["foo", "move" as const]]),
    };
    const { lastFrame } = render(<Adopt state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("move");
  });

  test("renders checkboxes with selected state", () => {
    const state = {
      ...initialAdoptState,
      candidates: [
        { kind: "skill" as const, name: "foo", source: "/path" },
        { kind: "skill" as const, name: "bar", source: "/path" },
      ],
      selectedIndices: [0],
    };
    const { lastFrame } = render(<Adopt state={state} dispatch={() => {}} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("[x]");
    expect(frame).toContain("[ ]");
  });

  test("renders footer hints", () => {
    const { lastFrame } = render(
      <Adopt state={initialAdoptState} dispatch={() => {}} />,
    );
    const frame = lastFrame() ?? "";
    expect(frame).toContain("toggle mode");
    expect(frame).toContain("adopt selected");
  });
});
