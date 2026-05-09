import { Box, Text } from "ink";
import React from "react";
import type { Action, ToggleState } from "../state/types";
import { List } from "./shared/List";
import { Footer } from "./shared/Footer";

interface Props {
  state: ToggleState;
  dispatch: (action: Action) => void;
}

const TYPE_OPTIONS = [
  { key: "skill", label: "skill" },
  { key: "plugin", label: "plugin" },
  { key: "mcp", label: "mcp" },
];

const TYPE_FOOTER = [
  { key: "↑↓", description: "navigate" },
  { key: "enter", description: "select" },
  { key: "q", description: "quit" },
];

const NAME_FOOTER = [
  { key: "↑↓", description: "navigate" },
  { key: "enter", description: "select" },
  { key: "esc", description: "back" },
  { key: "q", description: "quit" },
];

const COMPONENTS_FOOTER = [
  { key: "↑↓", description: "navigate" },
  { key: "space", description: "toggle" },
  { key: "enter", description: "confirm" },
  { key: "esc", description: "back" },
  { key: "q", description: "quit" },
];

function typeStepFocusIndex(state: ToggleState): number {
  if (state.type === null) return 0;
  const idx = TYPE_OPTIONS.findIndex((o) => o.key === state.type);
  return idx >= 0 ? idx : 0;
}

export const Toggle: React.FC<Props> = ({ state }) => {
  if (state.step === "type") {
    const focusIndex = typeStepFocusIndex(state);
    return (
      <Box flexDirection="column">
        <Text bold>Toggle — Select type</Text>
        <Box marginTop={1}>
          <List items={TYPE_OPTIONS} focusIndex={focusIndex} />
        </Box>
        <Footer hints={TYPE_FOOTER} />
      </Box>
    );
  }

  if (state.step === "name") {
    return (
      <Box flexDirection="column">
        <Text bold>
          Toggle — Select {state.type} to toggle
        </Text>
        <Box marginTop={1}>
          <List
            items={[]}
            focusIndex={0}
            emptyMessage="(loading…)"
          />
        </Box>
        <Footer hints={NAME_FOOTER} />
      </Box>
    );
  }

  // step === "components"
  const componentItems = state.components.map((c, i) => ({
    key: c.name + String(i),
    label: c.name,
    hint: c.active ? "enabled" : "disabled",
    selected: state.selectedComponentIndices.includes(i),
  }));

  const focusIndex = 0;

  return (
    <Box flexDirection="column">
      <Text bold>
        Toggle — {state.selectedName} — select components
      </Text>
      <Box marginTop={1}>
        <List
          items={componentItems}
          focusIndex={focusIndex}
          emptyMessage="(no components)"
        />
      </Box>
      <Footer hints={COMPONENTS_FOOTER} />
    </Box>
  );
};
