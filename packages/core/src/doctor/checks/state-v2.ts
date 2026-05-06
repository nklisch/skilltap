import { copyFile, writeFile } from "node:fs/promises";
import { z } from "zod/v4";
import { fileExists } from "../../fs";
import { type State, StateSchema } from "../../state/schema";
import { getStatePath } from "../../state/paths";
import type { DoctorCheck, DoctorIssue } from "../types";

const DEFAULT_STATE: State = { version: 2, skills: [], plugins: [], mcpServers: [] };

async function readStateFile(
  file: string,
  label: string,
  issues: DoctorIssue[],
): Promise<State | null> {
  if (!(await fileExists(file))) return null;

  let raw: unknown;
  try {
    raw = await Bun.file(file).json();
  } catch (e) {
    issues.push({
      message: `${label} is corrupt: ${e}`,
      fixable: true,
      fixDescription: `backed up to ${label}.bak, created fresh`,
      fix: async () => {
        await copyFile(file, `${file}.bak`).catch(() => {});
        await writeFile(file, JSON.stringify(DEFAULT_STATE, null, 2));
      },
    });
    return null;
  }

  const result = StateSchema.safeParse(raw);
  if (!result.success) {
    issues.push({
      message: `${label} is invalid: ${z.prettifyError(result.error)}`,
      fixable: true,
      fixDescription: `backed up to ${label}.bak, created fresh`,
      fix: async () => {
        await copyFile(file, `${file}.bak`).catch(() => {});
        await writeFile(file, JSON.stringify(DEFAULT_STATE, null, 2));
      },
    });
    return null;
  }

  return result.data;
}

export async function checkStateV2(projectRoot?: string): Promise<{
  check: DoctorCheck;
  state: State | null;
}> {
  const issues: DoctorIssue[] = [];
  const globalFile = getStatePath();
  const projectFile = projectRoot ? getStatePath(projectRoot) : null;

  const globalState = await readStateFile(globalFile, "state.json", issues);
  const projectState = projectFile
    ? await readStateFile(projectFile, ".agents/state.json", issues)
    : null;

  const merged: State | null =
    globalState || projectState
      ? {
          version: 2,
          skills: [...(globalState?.skills ?? []), ...(projectState?.skills ?? [])],
          plugins: [...(globalState?.plugins ?? []), ...(projectState?.plugins ?? [])],
          mcpServers: [
            ...(globalState?.mcpServers ?? []),
            ...(projectState?.mcpServers ?? []),
          ],
        }
      : null;

  if (issues.length > 0) {
    return { check: { name: "state.json", status: "fail", issues }, state: merged };
  }

  if (!merged) {
    return {
      check: {
        name: "state.json",
        status: "pass",
        detail: "n/a (no v2 state — run 'skilltap migrate' to upgrade)",
      },
      state: null,
    };
  }

  const skillCount = merged.skills.length;
  const pluginCount = merged.plugins.length;
  const mcpCount = merged.mcpServers.length;
  const detail = `${skillCount} skill${skillCount === 1 ? "" : "s"}, ${pluginCount} plugin${pluginCount === 1 ? "" : "s"}, ${mcpCount} standalone MCP${mcpCount === 1 ? "" : "s"}`;
  return { check: { name: "state.json", status: "pass", detail }, state: merged };
}
