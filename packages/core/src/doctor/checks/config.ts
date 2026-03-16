import { join } from "node:path";
import { parse } from "smol-toml";
import { z } from "zod/v4";
import { getConfigDir, migrateSecurityConfig } from "../../config";
import { fileExists } from "../../fs";
import type { Config } from "../../schemas/config";
import { ConfigSchema } from "../../schemas/config";
import type { DoctorCheck } from "../types";

export async function checkConfig(): Promise<{
  check: DoctorCheck;
  config: Config | null;
}> {
  const configDir = getConfigDir();
  const configFile = join(configDir, "config.toml");

  if (!(await fileExists(configFile))) {
    const check: DoctorCheck = {
      name: "config",
      status: "warn",
      issues: [
        {
          message: "No config.toml found. Run 'skilltap config' to create one.",
          fixable: true,
          fixDescription: "created default config",
          fix: async () => {
            const { loadConfig } = await import("../../config");
            await loadConfig();
          },
        },
      ],
    };
    return { check, config: null };
  }

  let text: string;
  try {
    text = await Bun.file(configFile).text();
  } catch (e) {
    return {
      check: {
        name: "config",
        status: "fail",
        issues: [{ message: `Cannot read config.toml: ${e}`, fixable: false }],
      },
      config: null,
    };
  }

  let raw: unknown;
  try {
    raw = parse(text);
  } catch (e) {
    return {
      check: {
        name: "config",
        status: "fail",
        issues: [
          {
            message: `config.toml is invalid TOML: ${e}`,
            fixable: false,
          },
        ],
      },
      config: null,
    };
  }

  const migrated = migrateSecurityConfig(raw as Record<string, unknown>);
  const result = ConfigSchema.safeParse(migrated);
  if (!result.success) {
    return {
      check: {
        name: "config",
        status: "fail",
        issues: [
          {
            message: `config.toml has invalid values: ${z.prettifyError(result.error)}`,
            fixable: false,
          },
        ],
      },
      config: null,
    };
  }

  return {
    check: { name: "config", status: "pass", detail: configFile },
    config: result.data,
  };
}
