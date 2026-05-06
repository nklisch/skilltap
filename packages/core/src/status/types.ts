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

  /** Skills loaded from v2 state.json if present, else v1 installed.json. */
  skills: StatusSkill[];
  /** Plugins loaded from v2 state.json if present, else v1 plugins.json. */
  plugins: StatusPlugin[];
  /** True if state.json was read (post-migration); false if reading v1 fallback. */
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
