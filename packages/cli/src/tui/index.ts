import { render } from "ink";
import React from "react";
import { App } from "./App";
import { createAppContext } from "./context";
import type { Screen } from "./state/types";

export async function mountTui(
  initialScreen: Screen = "dashboard",
): Promise<void> {
  process.stderr.write("[DEBUG TUI]: Calling createAppContext\n");
  const context = await createAppContext();
  process.stderr.write(
    "[DEBUG TUI]: createAppContext finished, rendering App\n",
  );
  const { waitUntilExit, unmount } = render(
    React.createElement(App, { initialScreen, context }),
  );
  process.stderr.write("[DEBUG TUI]: App rendered, waiting for exit\n");
  try {
    await waitUntilExit();
    process.stderr.write("[DEBUG TUI]: waitUntilExit finished\n");
  } finally {
    process.stderr.write("[DEBUG TUI]: Unmounting App\n");
    unmount();
  }
}
