import { PRESET_VALUES, SECURITY_PRESETS } from "../schemas/config";
import type { SecurityMode } from "../schemas/config";

export type SecurityPreset = (typeof SECURITY_PRESETS)[number];

/**
 * Return the preset name if the mode exactly matches a preset, or null.
 */
export function matchPreset(mode: SecurityMode): SecurityPreset | null {
  for (const preset of SECURITY_PRESETS) {
    const p = PRESET_VALUES[preset];
    if (p.scan === mode.scan && p.on_warn === mode.on_warn && p.require_scan === mode.require_scan) {
      return preset;
    }
  }
  return null;
}

/**
 * Return a human-friendly label for a security mode configuration.
 * Matches against known presets first, falls back to "custom (...)" description.
 *
 * Examples:
 *   { scan: "static", on_warn: "prompt", require_scan: false } → "standard (static + prompt)"
 *   { scan: "semantic", on_warn: "fail", require_scan: true }  → "strict (semantic + fail + require scan)"
 *   { scan: "static", on_warn: "fail", require_scan: false }   → "custom (static + fail)"
 */
export function describeSecurityMode(mode: SecurityMode): string {
  const preset = matchPreset(mode);

  const parts: string[] = [mode.scan, mode.on_warn];
  if (mode.require_scan) parts.push("require scan");
  const detail = parts.join(" + ");

  if (preset !== null) {
    return `${preset} (${detail})`;
  }
  return `custom (${detail})`;
}
