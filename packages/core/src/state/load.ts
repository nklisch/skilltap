import { loadJsonState } from "../json-state";
import type { Result, UserError } from "../types";
import { getStatePath } from "./paths";
import { type State, StateSchema } from "./schema";

const DEFAULT_STATE: State = {
  version: 2,
  skills: [],
  plugins: [],
  mcpServers: [],
};

export async function loadState(projectRoot?: string): Promise<Result<State, UserError>> {
  return loadJsonState(getStatePath(projectRoot), StateSchema, "state.json", DEFAULT_STATE);
}
