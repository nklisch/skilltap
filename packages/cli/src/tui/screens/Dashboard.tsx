import { Box, Text } from "ink";
import type React from "react";
import type { Action, DashboardState } from "../state/types";
import { Footer } from "./shared/Footer";
import { List } from "./shared/List";
import { Tabs } from "./shared/Tabs";

interface Props {
  state: DashboardState;
  dispatch: (action: Action) => void;
  data: unknown;
}

const TABS = [
  { id: "installed" as const, label: "Installed" },
  { id: "taps" as const, label: "Taps" },
  { id: "updates" as const, label: "Updates" },
  { id: "drift" as const, label: "Drift" },
];

const FOOTER_HINTS = [
  { key: "1-4", description: "switch tabs" },
  { key: "↑↓", description: "navigate" },
  { key: "i", description: "install" },
  { key: "r", description: "remove" },
  { key: "t", description: "toggle" },
  { key: "f", description: "find" },
  { key: "a", description: "adopt" },
  { key: "q", description: "quit" },
];

function dataToItems(
  data: unknown,
): { key: string; label: string; hint?: string }[] {
  if (!Array.isArray(data)) return [];
  return data.map((item, i) => {
    if (typeof item === "string") return { key: String(i), label: item };
    if (item && typeof item === "object") {
      const obj = item as Record<string, unknown>;
      const label = String(obj.name ?? obj.label ?? obj.id ?? i);
      const hint = obj.source !== undefined ? String(obj.source) : undefined;
      return { key: label + String(i), label, hint };
    }
    return { key: String(i), label: String(item) };
  });
}

export const Dashboard: React.FC<Props> = ({ state, dispatch, data }) => {
  const items = dataToItems(data);

  return (
    <Box flexDirection="column">
      <Tabs
        current={state.tab}
        tabs={TABS}
        onChange={(tab) => dispatch({ type: "dashboard:tab", tab })}
      />
      <Box marginTop={1} flexGrow={1}>
        {state.loading ? (
          <Text dimColor>Loading…</Text>
        ) : (
          <List
            items={items}
            focusIndex={state.selectedIndex}
            emptyMessage="(nothing here)"
          />
        )}
      </Box>
      <Footer hints={FOOTER_HINTS} />
    </Box>
  );
};
