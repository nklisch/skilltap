import { Box, Text } from "ink";
import type React from "react";

interface KeyHint {
  key: string;
  description: string;
}

interface Props {
  hints: KeyHint[];
}

export const Footer: React.FC<Props> = ({ hints }) => {
  return (
    <Box marginTop={1}>
      {hints.map((h, i) => (
        <Text key={i}>
          <Text color="cyan">{h.key}</Text>
          <Text dimColor> {h.description}</Text>
          {i < hints.length - 1 ? <Text dimColor> · </Text> : null}
        </Text>
      ))}
    </Box>
  );
};
