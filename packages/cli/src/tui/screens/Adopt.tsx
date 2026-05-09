import { Box, Text } from "ink";
import React from "react";
import type { Action, AdoptState } from "../state/types";
import { List } from "./shared/List";
import { Footer } from "./shared/Footer";

interface Props {
  state: AdoptState;
  dispatch: (action: Action) => void;
}

const FOOTER_HINTS = [
  { key: "↑↓", description: "navigate" },
  { key: "space", description: "select" },
  { key: "m", description: "toggle mode" },
  { key: "enter", description: "adopt selected" },
  { key: "q", description: "quit" },
];

export const Adopt: React.FC<Props> = ({ state }) => {
  const items = state.candidates.map((c, i) => {
    const mode = state.perItemMode.get(c.name) ?? "track-in-place";
    return {
      key: c.name + String(i),
      label: `${c.kind}: ${c.name}`,
      hint: mode,
      selected: state.selectedIndices.includes(i),
    };
  });

  return (
    <Box flexDirection="column">
      <Text bold>Adopt</Text>
      <Box marginTop={1} flexGrow={1}>
        {state.loading ? (
          <Text dimColor>Scanning for adoptable items…</Text>
        ) : (
          <List
            items={items}
            focusIndex={state.focusIndex}
            emptyMessage="(no adoptable items found)"
          />
        )}
      </Box>
      <Footer hints={FOOTER_HINTS} />
    </Box>
  );
};
