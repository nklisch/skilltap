import { ensureDirs } from "../dirs";
import { saveJsonState } from "../json-state";
import type { Result, UserError } from "../types";
import { getStatePath } from "./paths";
import type { State } from "./schema";

export async function saveState(
  state: State,
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  return saveJsonState(getStatePath(projectRoot), state, "state.json", projectRoot, ensureDirs);
}
