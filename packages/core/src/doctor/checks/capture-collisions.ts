/**
 * Defensive canary: a skill name appearing both as a `state.skills[].name` and
 * as a `state.plugins[].components[]` (skill type) entry should be impossible
 * — plugin capture transfers ownership atomically.
 *
 * If this check fires, something has bypassed capture: a manual state.json
 * edit, a bug in the install path, or pre-capture historical data. The fix
 * hint points the user at remove-the-standalone or remove-the-plugin.
 */

import type { State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkCaptureCollisions(
  state: State | null,
): Promise<DoctorCheck> {
  if (!state) {
    return { name: "Capture collisions", status: "pass" };
  }

  const standaloneNames = new Set(state.skills.map((s) => s.name));
  const issues: DoctorIssue[] = [];

  for (const plugin of state.plugins) {
    for (const component of plugin.components) {
      if (component.type !== "skill") continue;
      if (!standaloneNames.has(component.name)) continue;

      issues.push({
        message: `Skill "${component.name}" appears as both a standalone (state.skills[]) and a component of plugin "${plugin.name}" (state.plugins[].components[]). This is a capture bypass — plugin capture should prevent it.`,
        fixable: false,
        fixDescription: `run \`skilltap remove ${component.name}\` to release the standalone, or \`skilltap remove ${plugin.name}\` to remove the plugin and its component`,
      });
    }
  }

  if (issues.length === 0) {
    return { name: "Capture collisions", status: "pass" };
  }

  return {
    name: "Capture collisions",
    status: "warn",
    issues,
  };
}
