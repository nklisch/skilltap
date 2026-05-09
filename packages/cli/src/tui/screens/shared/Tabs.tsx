import { Box, Text } from "ink";

interface TabSpec<T extends string> {
  id: T;
  label: string;
}

interface Props<T extends string> {
  current: T;
  tabs: TabSpec<T>[];
  onChange?: (id: T) => void;
}

export function Tabs<T extends string>({
  current,
  tabs,
}: Props<T>): JSX.Element {
  return (
    <Box flexDirection="row">
      {tabs.map((t, i) => (
        <Box key={t.id} marginRight={2}>
          <Text
            bold={t.id === current}
            color={t.id === current ? "cyan" : "white"}
          >
            {`${i + 1} ${t.label}`}
          </Text>
        </Box>
      ))}
    </Box>
  );
}
