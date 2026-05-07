import { loadLockfile, loadManifest, manifestExists } from "../../manifest";
import { recoverManifest } from "../../manifest/recover";
import { type Lockfile, LockfileSchema } from "../../manifest/schemas";
import type { State } from "../../state/schema";
import { detectDrift } from "../../sync/drift";
import type { DoctorCheck, DoctorIssue } from "../types";

const EMPTY_LOCKFILE: Lockfile = LockfileSchema.parse({ version: 1 });

export async function checkManifestDrift(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  if (!state) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "n/a (no v2 state)",
    };
  }
  if (!projectRoot) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "n/a (no project root)",
    };
  }
  if (!(await manifestExists(projectRoot))) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "n/a (no skilltap.toml)",
    };
  }

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) {
    return {
      name: "manifest drift",
      status: "fail",
      issues: [
        {
          message: `Failed to load skilltap.toml: ${manifestResult.error.message}`,
          fixable: true,
          fixDescription:
            "backed up to skilltap.toml.bak, created fresh empty manifest",
          fix: async () => {
            await recoverManifest(projectRoot);
          },
        },
      ],
    };
  }

  const lockfileResult = await loadLockfile(projectRoot);
  const lockfile = lockfileResult.ok ? lockfileResult.value : EMPTY_LOCKFILE;

  const drift = detectDrift(manifestResult.value, lockfile, state);

  const items = drift.items.filter(
    (i) => i.kind === "add" || i.kind === "remove" || i.kind === "ref-mismatch",
  );

  if (items.length === 0) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "in sync",
    };
  }

  const issues: DoctorIssue[] = items.map((item) => ({
    message: `${item.kind}: ${item.target} ${item.source}${item.reason ? ` — ${item.reason}` : ""}`,
    fixable: false,
  }));

  return {
    name: "manifest drift",
    status: "warn",
    detail: `${items.length} drift item${items.length === 1 ? "" : "s"} — run 'skilltap sync' for details`,
    issues,
  };
}
