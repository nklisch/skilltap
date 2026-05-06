import { discoverPublishablePlugins } from "../../manifest/publish";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkPluginManifests(projectRoot?: string): Promise<DoctorCheck> {
  if (!projectRoot) {
    return {
      name: "plugin manifests",
      status: "pass",
      detail: "n/a (no project root)",
    };
  }

  const result = await discoverPublishablePlugins(projectRoot);

  const issues: DoctorIssue[] = result.rejected
    .filter((r) => !r.reason.startsWith("publish = false"))
    .map((r) => ({
      message: `${r.path}: ${r.reason}`,
      fixable: false,
    }));

  if (result.publishable.length === 0 && issues.length === 0) {
    return {
      name: "plugin manifests",
      status: "pass",
      detail: "n/a (no .skilltap/ publish manifests)",
    };
  }

  if (issues.length === 0) {
    return {
      name: "plugin manifests",
      status: "pass",
      detail: `${result.publishable.length} valid`,
    };
  }

  return {
    name: "plugin manifests",
    status: "warn",
    detail: `${result.publishable.length} valid, ${issues.length} invalid`,
    issues,
  };
}
