type SecurityModeFields = { scan: string; on_warn: string };

/**
 * Return a human-friendly label for a security mode configuration.
 * Examples:
 *   { scan: "static", on_warn: "install" } → "static + install"
 *   { scan: "semantic", on_warn: "fail" }  → "semantic + fail"
 */
export function describeSecurityMode(mode: SecurityModeFields): string {
  return `${mode.scan} + ${mode.on_warn}`;
}
