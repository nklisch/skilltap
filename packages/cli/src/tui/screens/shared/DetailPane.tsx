import { Box, Text } from "ink";
import type React from "react";

interface Props {
  title: string;
  body: string | string[];
  width?: number;
}

export const DetailPane: React.FC<Props> = ({ title, body, width }) => {
  const lines = Array.isArray(body) ? body : [body];
  return (
    <Box flexDirection="column" width={width} paddingLeft={2}>
      <Text bold>{title}</Text>
      <Box marginTop={1} flexDirection="column">
        {lines.map((line, i) => (
          <Text key={i}>{line}</Text>
        ))}
      </Box>
    </Box>
  );
};
