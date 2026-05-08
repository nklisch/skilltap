import type { DriftReport } from "../sync/types";

export interface StatusReport {
  /** Project root if found, else null. */
  projectRoot: string | null;
  /** Whether the project root contains a skilltap.toml. */
  hasManifest: boolean;
  /** Scope inferred from context (smart-default: project in git repo, else global). */
  scope: "global" | "project";
  /** Resolved [defaults.also] list (from project manifest if present, else config). */
  also: string[];

  /** Skills loaded from state.json. */
  skills: StatusSkill[];
  /** Plugins loaded from state.json. */
  plugins: StatusPlugin[];
  /** True if state.json exists on disk; false if using empty defaults. */
  fromV2State: boolean;

  /** Taps from config + the built-in tap if enabled. */
  taps: StatusTap[];

  /** Drift report — populated only if skilltap.toml exists. */
  drift: DriftReport | null;
}

export interface StatusSkill {
  name: string;
  scope: "global" | "project" | "linked";
  source: string | null;
  ref: string | null;
  also: string[];
  active: boolean;
}

export interface StatusPlugin {
  name: string;
  scope: "global" | "project";
  source: string | null;
  ref: string | null;
  componentCount: number;
  componentSummary: string;
  active: boolean;
}

export interface StatusTap {
  name: string;
  url: string;
  builtin: boolean;
  /** Reported as "git" — HTTP taps are filtered/labelled separately when present. */
  type: "git" | "http" | "builtin";
}
