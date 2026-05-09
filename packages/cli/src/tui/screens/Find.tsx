import { Box, Text } from "ink";
import type React from "react";
import type { Action, FindState } from "../state/types";
import { DetailPane } from "./shared/DetailPane";
import { Footer } from "./shared/Footer";
import { List } from "./shared/List";

interface Props {
  state: FindState;
  dispatch: (action: Action) => void;
}

const FOOTER_HINTS = [
  { key: "type", description: "search" },
  { key: "↑↓", description: "navigate" },
  { key: "enter", description: "install" },
  { key: "esc", description: "clear" },
  { key: "q", description: "quit" },
];

export const Find: React.FC<Props> = ({ state }) => {
  const selected = state.results[state.selectedIndex] ?? null;

  const listItems = state.results.map((r, i) => ({
    key: r.name + String(i),
    label: r.name,
    hint: r.type,
  }));

  const detailBody = selected
    ? [
        `Type: ${selected.type}`,
        `Source: ${selected.source}`,
        selected.description ? `Description: ${selected.description}` : "",
      ].filter(Boolean)
    : ["Select a result to see details."];

  return (
    <Box flexDirection="column">
      <Box borderStyle="single" paddingX={1}>
        <Text>
          <Text dimColor>Search: </Text>
          {state.query.length > 0 ? (
            <Text color="white">{state.query}</Text>
          ) : (
            <Text dimColor>type to search…</Text>
          )}
          <Text color="cyan">█</Text>
        </Text>
      </Box>
      <Box flexDirection="row" marginTop={1} flexGrow={1}>
        <Box flexGrow={1}>
          {state.loading ? (
            <Text dimColor>Searching…</Text>
          ) : (
            <List
              items={listItems}
              focusIndex={state.selectedIndex}
              emptyMessage={
                state.query.length > 0
                  ? "(no results)"
                  : "(start typing to search)"
              }
            />
          )}
        </Box>
        {selected && (
          <DetailPane title={selected.name} body={detailBody} width={40} />
        )}
      </Box>
      <Footer hints={FOOTER_HINTS} />
    </Box>
  );
};
