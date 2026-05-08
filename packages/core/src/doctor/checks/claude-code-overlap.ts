/**
 * Defensive: a Claude Code plugin's name matches a skilltap-managed standalone
 * skill or plugin (where the skilltap record is NOT itself an adopted Claude
 * Code plugin). This signals a potential collision a user might want to
 * resolve via `skilltap adopt --source claude-code`.
 */

import { scanAllAgentPlugins } from "../../agent-plugins/registry";
import type { State } from "../../state/schema";
import type { DoctorCheck } from "../types";

export async function checkClaudeCodeOverlap(
  state: State | null,
): Promise<DoctorCheck> {
  if (!state) return { name: "Claude Code overlaps", status: "pass" };

  const scanResult = await scanAllAgentPlugins();
  if (!scanResult.ok) {
    return {
      name: "Claude Code overlaps",
      status: "warn",
      issues: [
        {
          message: `Could not scan Claude Code plugins: ${scanResult.error.message}`,
          fixable: false,
        },
      ],
    };
  }
  const claudePlugins = scanResult.value.plugins.filter(
    (p) => p.scannerName === "claude-code",
  );
  if (claudePlugins.length === 0) {
    return { name: "Claude Code overlaps", status: "pass" };
  }

  const adoptedSourceMarker = "claude-code:";
  const issues: DoctorCheck["issues"] = [];

  for (const plugin of claudePlugins) {
    // Skill collision
    const skillCollision = state.skills.find((s) => s.name === plugin.name);
    if (skillCollision) {
      issues.push({
        message: `Claude Code plugin "${plugin.name}" overlaps with skilltap standalone skill "${skillCollision.name}".`,
        fixable: false,
        fixDescription: `Run \`skilltap adopt ${plugin.name}\` to bring the Claude Code plugin under skilltap, or \`skilltap remove skill ${skillCollision.name}\` if Claude Code's version should win.`,
      });
    }

    // Plugin collision (only flag if the existing record is NOT itself adopted)
    const pluginCollision = state.plugins.find(
      (p) => p.name === plugin.name && !p.repo?.startsWith(adoptedSourceMarker),
    );
    if (pluginCollision) {
      issues.push({
        message: `Claude Code plugin "${plugin.name}" overlaps with skilltap-installed plugin (different source).`,
        fixable: false,
        fixDescription: `Run \`skilltap remove plugin ${plugin.name}\` then \`skilltap adopt ${plugin.name}\` if you want Claude Code's version.`,
      });
    }
  }

  return {
    name: "Claude Code overlaps",
    status: issues.length > 0 ? "warn" : "pass",
    issues,
  };
}
