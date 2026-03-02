import type { Config } from "@skilltap/core";

// Replace these with your GA4 Measurement ID and API secret after setting up
// a GA4 property at analytics.google.com → Admin → Data Streams →
// Measurement Protocol API secrets
const GA4_MEASUREMENT_ID = "G-XXXXXXXXXX";
const GA4_API_SECRET = "REPLACE_WITH_SECRET";
const GA4_ENDPOINT = `https://www.google-analytics.com/mp/collect?measurement_id=${GA4_MEASUREMENT_ID}&api_secret=${GA4_API_SECRET}`;

export function isTelemetryEnabled(config: Config): boolean {
  if (process.env.DO_NOT_TRACK === "1") return false;
  if (process.env.SKILLTAP_TELEMETRY_DISABLED === "1") return false;
  return config.telemetry.enabled && !!config.telemetry.anonymous_id;
}

export function sendEvent(
  config: Config,
  name: string,
  params: Record<string, string | number | boolean>,
): void {
  const debug = process.env.SKILLTAP_TELEMETRY_DEBUG === "1";
  if (!isTelemetryEnabled(config) && !debug) return;

  if (debug) {
    process.stderr.write(`[telemetry] ${name}: ${JSON.stringify(params)}\n`);
    if (!isTelemetryEnabled(config)) return;
  }

  const payload = JSON.stringify({
    client_id: config.telemetry.anonymous_id,
    events: [{ name, params: { ...params, engagement_time_msec: 1 } }],
  });

  // Fire and forget — never await, never throw
  fetch(GA4_ENDPOINT, {
    method: "POST",
    body: payload,
    signal: AbortSignal.timeout(3000),
  }).catch(() => {});
}

export function inferAdapter(source: string): string {
  if (source.startsWith("npm:")) return "npm";
  if (source.startsWith("github:")) return "github";
  if (source.startsWith("gitlab:")) return "gitlab";
  if (source.startsWith("bitbucket:")) return "bitbucket";
  if (source.startsWith("http://") || source.startsWith("https://")) return "git";
  if (source.startsWith(".") || source.startsWith("/") || source.startsWith("~")) return "local";
  return "tap";
}
