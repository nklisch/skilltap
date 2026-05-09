import { Box, Text } from "ink";
import React from "react";

interface ListItem {
  key: string;
  label: string;
  hint?: string;
  selected?: boolean;
}

interface Props {
  items: ListItem[];
  focusIndex: number;
  emptyMessage?: string;
}

export const List: React.FC<Props> = ({ items, focusIndex, emptyMessage }) => {
  if (items.length === 0) {
    return <Text dimColor>{emptyMessage ?? "(empty)"}</Text>;
  }
  return (
    <Box flexDirection="column">
      {items.map((item, i) => {
        const focused = i === focusIndex;
        const checkbox =
          item.selected !== undefined ? (item.selected ? "[x] " : "[ ] ") : "";
        return (
          <Text key={item.key} color={focused ? "cyan" : undefined} bold={focused}>
            {focused ? "▶ " : "  "}
            {checkbox}
            {item.label}
            {item.hint ? <Text dimColor> {item.hint}</Text> : null}
          </Text>
        );
      })}
    </Box>
  );
};
