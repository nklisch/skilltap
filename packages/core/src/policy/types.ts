export type EffectivePolicy = {
  yes: boolean;
  scope: "global" | "project" | "";
  also: string[];
  scanMode: "semantic" | "static" | "none";
  onWarn: "prompt" | "fail" | "install";
  /** True if --skip-scan was passed (per-call escape hatch). */
  skipScan: boolean;
  /**
   * Set per-source by composePolicyForSource. True iff the source matched any
   * pattern in `[security].trust`. When trusted, scanMode is forced to
   * "none" — caller skips scanning entirely.
   */
  trusted: boolean;
};

export type CliFlags = {
  yes?: boolean;
  noYes?: boolean;
  strict?: boolean;
  skipScan?: boolean;
  /** Forces scanMode = "semantic" for this invocation. */
  deep?: boolean;
  scope?: "project" | "global";
};

export type SourceForPolicy = {
  tapName?: string;
  /** The source string as the user typed it, e.g. "github.com/corp/foo" or "npm:@corp/foo". */
  sourceUrl: string;
};
