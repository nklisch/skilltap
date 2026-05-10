import type { AgentAdapter } from "../agents/types";
import type { OnOrphansFound } from "../orphan";
import type { Output } from "../output/types";
import type { ScannedSkill } from "../scanner";
import type { InstalledSkill } from "../schemas/installed";
import type { PluginManifest } from "../schemas/plugin";
import type { PluginRecord } from "../schemas/plugins";
import type { StaticWarning } from "../security";
import type { SemanticWarning } from "../security/semantic";
import type { TapEntry } from "../taps";

// Re-export the capture callback signatures so callers don't have to import
// from plugin/capture directly.
export type PluginCaptureConfirm = (
  bucket: import("../plugin/capture").CaptureBucket,
) => Promise<boolean>;
export type PluginCaptureConflict = (
  bucket: import("../plugin/capture").CaptureBucket,
) => Promise<"abort" | "force" | "skip">;

export type InstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  skillNames?: string[];
  also?: string[];
  ref?: string;
  tap?: string | null;
  /** Default git host for owner/repo shorthand resolution. */
  gitHost?: string;
  skipScan?: boolean;
  /**
   * Optional progress reporter. When provided, core calls out.progress(label)
   * for long-running phases (clone, scan, semantic). Replaces the removed
   * onStaticScanStart / onSemanticScanStart / onSemanticProgress callbacks.
   */
  out?: Output;
  /**
   * Called before placement if warnings are found. Return true to proceed, false to abort.
   * Unified callback for skill-static, plugin-static, and skill-semantic warnings.
   */
  onWarnings?: (
    warnings: StaticWarning[] | SemanticWarning[],
    kind: "skill-static" | "plugin-static" | "skill-semantic",
    name: string,
  ) => Promise<boolean>;
  /** Called after scan, before placement. Returns skill names to install. If omitted, installs all. */
  onSelectSkills?: (skills: ScannedSkill[]) => Promise<string[]>;
  /** Called when source resolves to multiple taps. Return chosen entry or null to cancel. */
  onSelectTap?: (matches: TapEntry[]) => Promise<TapEntry | null>;
  /** Pre-resolved agent adapter for semantic scanning (or undefined to skip). */
  agent?: AgentAdapter;
  /** Whether to run semantic scan (--semantic flag or config). */
  semantic?: boolean;
  /** Score threshold for semantic warnings (default 5). */
  threshold?: number;
  /**
   * Called after all scans pass cleanly, before placement. Return false to cancel.
   * Unified callback for skill confirmation (kind="skill", names=string[]) and
   * plugin confirmation (kind="plugin", names=PluginManifest).
   */
  onConfirmInstall?: (
    kind: "skill" | "plugin",
    names: string[] | PluginManifest,
  ) => Promise<boolean>;
  /** Called when a skill is already installed. Return "update" to update it instead, or "abort" to cancel. */
  onAlreadyInstalled?: (name: string) => Promise<"update" | "abort">;
  /** Called when deep scan is triggered (no SKILL.md at standard paths). Return false to cancel. */
  onDeepScan?: (count: number) => Promise<boolean>;
  /** Called when orphan records are detected before the install. Return names to purge. */
  onOrphansFound?: OnOrphansFound;
  /** Called when a plugin manifest is detected after cloning. Return "plugin" to install as plugin,
   *  "skills-only" to ignore the plugin and install skills normally, or "cancel" to abort. */
  onPluginDetected?: (
    manifest: PluginManifest,
  ) => Promise<"plugin" | "skills-only" | "cancel">;
  /**
   * Called when a plugin install matches same-source standalones for capture.
   * Threaded through to installPlugin's onCaptureConfirm. Return true to
   * capture, false to abort. Omitted → auto-capture.
   */
  onPluginCaptureConfirm?: PluginCaptureConfirm;
  /**
   * Called when a plugin install matches cross-source standalones (different
   * canonical source, or no recorded source). Threaded through to
   * installPlugin's onCaptureConflict. Return "abort" to fail or "force" to
   * override. Omitted with non-empty cross-source matches → install fails.
   */
  onPluginCaptureConflict?: PluginCaptureConflict;
  /**
   * Bypass capture entirely — install the plugin side-by-side with any
   * existing standalones (same-source or cross-source). Threaded through to
   * `installPlugin`'s `skipCapture`. Used by the CLI's `--no-capture` flag.
   */
  pluginSkipCapture?: boolean;
  /**
   * Override which plugin to select from a multi-plugin `.skilltap/` repo.
   * When set, takes precedence over any selector parsed from the source string
   * (e.g. `owner/repo:auth`). The CLI uses this to drive `:*` expansion: it
   * detects all available plugins, then loops calling `installSkill` once per
   * name with `selectName` set to the chosen plugin.
   */
  selectName?: string;
};

export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
  semanticWarnings: SemanticWarning[];
  /** Names of skills that were already installed and the user chose to update instead. */
  updates: string[];
  /** If a plugin was installed, the plugin record. */
  pluginRecord?: PluginRecord;
  /**
   * Components transferred from standalone state into the plugin (only set
   * when a plugin was installed and capture occurred).
   */
  captured?: {
    skills: string[];
    mcpServers: string[];
    forcedCrossSource: { skills: string[]; mcpServers: string[] };
  };
};
