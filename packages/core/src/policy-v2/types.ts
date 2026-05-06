export type EffectivePolicyV2 = {
  // Behavior
  /** True if --yes was passed, OR if agent mode is active. */
  yes: boolean;
  /** Resolved from --agent / --no-agent / SKILLTAP_AGENT env / config.agent.default. */
  agent: boolean;

  // Placement
  scope: "global" | "project" | "";
  also: string[];

  // Security
  scanMode: "semantic" | "static" | "none";
  onWarn: "prompt" | "fail" | "install";
  /** True if --skip-scan was passed (per-call escape hatch). */
  skipScan: boolean;

  /**
   * Set per-source by composeV2ForSource. True iff the source matched any
   * pattern in `[security].trust`. When trusted, scanMode is forced to
   * "none" — caller skips scanning entirely.
   */
  trusted: boolean;
};

export type CliFlagsV2 = {
  agent?: boolean;
  noAgent?: boolean;
  yes?: boolean;
  noYes?: boolean;
  strict?: boolean;
  noStrict?: boolean;
  skipScan?: boolean;
  /** Forces scanMode = "semantic" for this invocation. */
  deep?: boolean;
  project?: boolean;
  global?: boolean;
};

export type EnvV2 = {
  /** From SKILLTAP_AGENT env var (truthy → enable agent mode). */
  agent?: boolean;
};

export type SourceForPolicy = {
  tapName?: string;
  /** The source string as the user typed it, e.g. "github.com/corp/foo" or "npm:@corp/foo". */
  sourceUrl: string;
};
