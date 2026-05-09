import type { AgentAdapter } from "../agents/types";
import type { DiffStat } from "../git";
import type { OnOrphansFound } from "../orphan";
import type { StaticWarning } from "../security";
import type { SemanticWarning } from "../security/semantic";
import type { resolveTrust } from "../trust";

export type UpdateOptions = {
  /** Specific skill to update; undefined = update all */
  name?: string;
  /** Auto-accept clean updates without prompting */
  yes?: boolean;
  /** Skip skills that have security warnings in their diff */
  strict?: boolean;
  /** Project root — also processes project-scoped skills from {projectRoot}/.agents/state.json */
  projectRoot?: string;
  onProgress?: (
    skillName: string,
    status:
      | "checking"
      | "upToDate"
      | "updated"
      | "skipped"
      | "linked"
      | "local"
      | "removed-upstream",
  ) => void;
  /** Called when orphan records are detected before the update pass. Return names to purge. */
  onOrphansFound?: OnOrphansFound;
  /** Called when a multi-skill's subdirectory is gone from the cache after pull. */
  onSkillRemovedUpstream?: (
    skillName: string,
    repoUrl: string,
  ) => Promise<"remove" | "skip">;
  onDiff?: (
    skillName: string,
    stat: DiffStat,
    fromSha: string,
    toSha: string,
    rawDiff: string,
  ) => void;
  /** Called when warnings are found. Return value only matters in non-strict mode: true = proceed. */
  onShowWarnings?: (warnings: StaticWarning[], skillName: string) => void;
  /** Called when user confirmation is needed. true = apply. */
  onConfirm?: (skillName: string, hasWarnings: boolean) => Promise<boolean>;
  /** Pre-resolved agent adapter for semantic scanning. */
  agent?: AgentAdapter;
  /** Whether to run semantic scan. */
  semantic?: boolean;
  /** Score threshold for semantic warnings (default 5). */
  threshold?: number;
  /** Called when semantic warnings are found. */
  onSemanticWarnings?: (warnings: SemanticWarning[], skillName: string) => void;
  /** Called before starting semantic scan for a skill. */
  onSemanticScanStart?: (skillName: string) => void;
  /** Called with progress during semantic scan. */
  onSemanticProgress?: (
    completed: number,
    total: number,
    score: number,
    reason: string,
  ) => void;
  /** Force re-apply the update even if the skill appears up to date (same SHA / version). */
  force?: boolean;
};

export type UpdateResult = {
  updated: string[];
  skipped: string[];
  upToDate: string[];
};

export type ResolveTrustFn = typeof resolveTrust;
