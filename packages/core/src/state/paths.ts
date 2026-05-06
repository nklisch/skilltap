import { join } from "node:path";
import { getConfigDir } from "../dirs";

// state.json lives next to where v1.0 installed.json/plugins.json lived.
// Phase 27 only writes here; Phase 31 cuts over readers.
export function getStatePath(projectRoot?: string): string {
  return projectRoot
    ? join(projectRoot, ".agents", "state.json")
    : join(getConfigDir(), "state.json");
}
