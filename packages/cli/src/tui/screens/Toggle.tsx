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

export const Toggle: React.FC<Props> = ({ state }) => {
  if (state.step === "type") {
    return (
      <Box flexDirection="column">
        <Text bold>Toggle — Select type</Text>
        <Box marginTop={1}>
          <List items={TYPE_OPTIONS} focusIndex={state.focusIndex} />
        </Box>
        <Footer hints={TYPE_FOOTER} />
      </Box>
    );
  }

  if (state.step === "name") {
    const items = state.names.map((name, i) => ({
      key: `${name}-${i}`,
      label: name,
    }));
    return (
      <Box flexDirection="column">
        <Text bold>
          Toggle — Select {state.type} to toggle
        </Text>
        <Box marginTop={1}>
          <List
            items={items}
            focusIndex={state.focusIndex}
            emptyMessage={state.namesLoading ? "(loading…)" : `(no ${state.type}s installed)`}
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

  return (
    <Box flexDirection="column">
      <Text bold>
        Toggle — {state.selectedName} — select components
      </Text>
      <Box marginTop={1}>
        <List
          items={componentItems}
          focusIndex={state.focusIndex}
          emptyMessage="(no components)"
        />
      </Box>
      <Footer hints={COMPONENTS_FOOTER} />
    </Box>
  );
};
