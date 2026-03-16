export interface DoctorIssue {
  message: string;
  fixable: boolean;
  /** Describes what the fix action did, shown after ` — ` when fixed. */
  fixDescription?: string;
  fix?: () => Promise<void>;
  /** Set to true after fix() completes successfully. */
  fixed?: boolean;
}

export interface DoctorCheck {
  name: string;
  status: "pass" | "warn" | "fail";
  detail?: string;
  issues?: DoctorIssue[];
  /** Informational per-item status lines shown after issues (e.g. per-tap health). */
  info?: string[];
}

export interface DoctorResult {
  ok: boolean;
  checks: DoctorCheck[];
}

export interface DoctorOptions {
  fix?: boolean;
  projectRoot?: string;
  onCheck?: (check: DoctorCheck) => void;
}
