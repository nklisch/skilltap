import { Box, useApp, useInput } from "ink";
import React, { useEffect, useReducer, useState } from "react";
import { initialAppState, appReducer } from "./state/app";
import { GLOBAL_KEYS } from "./keys";
import type { Action, AppContext, AppState } from "./state/types";
import { Dashboard } from "./screens/Dashboard";
import { Find } from "./screens/Find";
import { Toggle } from "./screens/Toggle";
import { Adopt } from "./screens/Adopt";
import type { Key } from "ink";

interface AppProps {
  initialScreen?: AppState["screen"];
  context: AppContext;
}

function handleScreenKey(
  state: AppState,
  dispatch: (action: Action) => void,
  input: string,
  key: Key,
): void {
  const isUp = key.upArrow;
  const isDown = key.downArrow;
  const isEnter = key.return;
  const isEscape = key.escape;
  const isSpace = input === " ";

  switch (state.screen) {
    case "dashboard": {
      if (isUp) dispatch({ type: "dashboard:cursor", delta: -1 });
      else if (isDown) dispatch({ type: "dashboard:cursor", delta: 1 });
      else if (input === "f") dispatch({ type: "navigate", screen: "find" });
      else if (input === "t") dispatch({ type: "navigate", screen: "toggle" });
      else if (input === "a") dispatch({ type: "navigate", screen: "adopt" });
      break;
    }
    case "find": {
      if (isUp) dispatch({ type: "find:cursor", delta: -1 });
      else if (isDown) dispatch({ type: "find:cursor", delta: 1 });
      else if (isEscape) dispatch({ type: "find:query", query: "" });
      else if (key.backspace || key.delete) {
        const q = state.state.query;
        dispatch({ type: "find:query", query: q.slice(0, -1) });
      } else if (input && !key.ctrl && !key.meta && input.length === 1) {
        dispatch({ type: "find:query", query: state.state.query + input });
      }
      break;
    }
    case "toggle": {
      if (isEscape || (key.ctrl && input === "[")) {
        dispatch({ type: "toggle:step-back" });
      } else if (isUp) {
        // cursor nav handled by parent holding focusIndex — no action yet
      } else if (isDown) {
        // cursor nav handled by parent holding focusIndex — no action yet
      } else if (isEnter) {
        const s = state.state;
        if (s.step === "type") {
          const types: Array<"skill" | "plugin" | "mcp"> = ["skill", "plugin", "mcp"];
          const focusIndex = s.type === null ? 0 : types.indexOf(s.type as "skill" | "plugin" | "mcp");
          const chosen = types[Math.max(0, focusIndex)] ?? "skill";
          dispatch({ type: "toggle:set-type", value: chosen });
        }
      }
      break;
    }
    case "adopt": {
      if (isUp) dispatch({ type: "adopt:cursor", delta: -1 });
      else if (isDown) dispatch({ type: "adopt:cursor", delta: 1 });
      else if (isSpace) dispatch({ type: "adopt:select-toggle" });
      else if (input === "m") dispatch({ type: "adopt:mode-toggle" });
      break;
    }
  }
}

export const App: React.FC<AppProps> = ({ initialScreen = "dashboard", context }) => {
  const { exit } = useApp();
  const [state, dispatch] = useReducer(appReducer, initialAppState(initialScreen));
  const [data, setData] = useState<unknown>(null);

  useInput((input, key) => {
    for (const binding of GLOBAL_KEYS) {
      if (binding.key === input) {
        if (binding.action.type === "exit") {
          exit();
          return;
        }
        dispatch(binding.action as Action);
        return;
      }
    }
    handleScreenKey(state, dispatch, input, key);
  });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      switch (state.screen) {
        case "dashboard": {
          const dashData = await context.loadDashboardData(state.state.tab);
          if (!cancelled) setData(dashData);
          break;
        }
        case "find": {
          if (state.state.query.length > 0) {
            const results = await context.loadFindResults(state.state.query);
            if (!cancelled) dispatch({ type: "find:results", results });
          } else {
            if (!cancelled) dispatch({ type: "find:results", results: [] });
          }
          break;
        }
        case "toggle": {
          if (state.state.step === "components" && state.state.type && state.state.selectedName) {
            const components = await context.loadToggleComponents(
              state.state.type,
              state.state.selectedName,
            );
            if (!cancelled) dispatch({ type: "toggle:components-loaded", components });
          }
          break;
        }
        case "adopt": {
          if (state.state.candidates.length === 0 && !state.state.loading) {
            const candidates = await context.loadAdoptCandidates();
            if (!cancelled) dispatch({ type: "adopt:candidates-loaded", candidates });
          }
          break;
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [
    state.screen,
    state.screen === "dashboard" ? state.state.tab : null,
    state.screen === "find" ? state.state.query : null,
    state.screen === "toggle" ? state.state.step : null,
  ]);

  switch (state.screen) {
    case "dashboard":
      return (
        <Box flexDirection="column">
          <Dashboard state={state.state} dispatch={dispatch} data={data} />
        </Box>
      );
    case "find":
      return (
        <Box flexDirection="column">
          <Find state={state.state} dispatch={dispatch} />
        </Box>
      );
    case "toggle":
      return (
        <Box flexDirection="column">
          <Toggle state={state.state} dispatch={dispatch} />
        </Box>
      );
    case "adopt":
      return (
        <Box flexDirection="column">
          <Adopt state={state.state} dispatch={dispatch} />
        </Box>
      );
  }
};
