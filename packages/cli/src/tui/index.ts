import { render } from "ink";
import React from "react";
import { App } from "./App";
import { createAppContext } from "./context";
import type { Screen } from "./state/types";

export async function mountTui(
  initialScreen: Screen = "dashboard",
): Promise<void> {
  const context = await createAppContext();
  const { waitUntilExit, unmount } = render(
    React.createElement(App, { initialScreen, context }),
  );
  try {
    await waitUntilExit();
  } finally {
    unmount();
  }
}
