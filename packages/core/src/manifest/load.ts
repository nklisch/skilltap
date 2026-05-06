import { parse } from "smol-toml";
import { parseWithResult } from "../schemas";
import { err, ok, type Result, UserError } from "../types";
import { manifestPath } from "./paths";
import { type ProjectManifest, ProjectManifestSchema } from "./schemas";

export async function manifestExists(projectRoot: string): Promise<boolean> {
  return await Bun.file(manifestPath(projectRoot)).exists();
}

const DEFAULT_MANIFEST: ProjectManifest = ProjectManifestSchema.parse({});

export async function loadManifest(
  projectRoot: string,
): Promise<Result<ProjectManifest, UserError>> {
  const path = manifestPath(projectRoot);
  const f = Bun.file(path);
  if (!(await f.exists())) return ok(DEFAULT_MANIFEST);

  let text: string;
  try {
    text = await f.text();
  } catch (e) {
    return err(new UserError(`Failed to read ${path}: ${e}`));
  }

  let raw: unknown;
  try {
    raw = parse(text);
  } catch (e) {
    return err(new UserError(`Invalid TOML in ${path}: ${e}`));
  }

  return parseWithResult(ProjectManifestSchema, raw, "skilltap.toml");
}
