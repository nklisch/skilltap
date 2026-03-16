import { detectAgents } from "../../agents/detect";
import { fileExists } from "../../fs";
import type { Config } from "../../schemas/config";
import type { DoctorCheck } from "../types";

export async function checkAgents(config: Config): Promise<DoctorCheck> {
  const available = await detectAgents();
  const configuredAgent = config.security.agent_cli;

  if (configuredAgent) {
    const isAbsPath = configuredAgent.startsWith("/");
    if (isAbsPath) {
      const exists = await fileExists(configuredAgent);
      if (!exists) {
        return {
          name: "agents",
          status: "warn",
          detail:
            available.length > 0
              ? `${available.length} detected`
              : "none detected",
          issues: [
            {
              message: `Configured agent '${configuredAgent}' not found on disk. Semantic scan will fail.`,
              fixable: false,
            },
          ],
        };
      }
    } else {
      const found = available.find(
        (a) =>
          a.cliName === configuredAgent ||
          a.name.toLowerCase() === configuredAgent,
      );
      if (!found) {
        return {
          name: "agents",
          status: "warn",
          detail:
            available.length > 0
              ? `${available.length} detected`
              : "none detected",
          issues: [
            {
              message: `Configured agent '${configuredAgent}' not found on PATH. Semantic scan will fail.`,
              fixable: false,
            },
          ],
        };
      }
    }
  }

  if (available.length === 0) {
    return {
      name: "agents",
      status: "pass",
      detail: "none detected (semantic scanning unavailable)",
    };
  }

  const names = available.map((a) => a.cliName).join(", ");
  return {
    name: "agents",
    status: "pass",
    detail: `${available.length} detected (${names})`,
  };
}
