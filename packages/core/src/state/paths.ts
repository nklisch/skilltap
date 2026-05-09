import { join } from "node:path";
import { getConfigDir } from "../dirs";

// state.json lives next to where v0.x installed.json/plugins.json lived.
export function getStatePath(projectRoot?: string): string {
  return projectRoot
    ? join(projectRoot, ".agents", "state.json")
    : join(getConfigDir(), "state.json");
}
