#!/usr/bin/env bun
import { render, Box, Text, useApp, useInput } from "ink";
import React from "react";

const SpikeApp: React.FC = () => {
  const { exit } = useApp();
  useInput((input, key) => {
    if (input === "q" || (key.ctrl && input === "c")) {
      exit();
    }
  });
  return (
    <Box flexDirection="column" padding={1}>
      <Text color="green">✓ Ink renders under Bun</Text>
      <Text dimColor>Press q to exit cleanly</Text>
    </Box>
  );
};

const { unmount, waitUntilExit } = render(<SpikeApp />);
waitUntilExit().then(() => unmount());
