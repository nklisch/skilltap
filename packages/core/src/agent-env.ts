// Single source of truth for the SKILLTAP_AGENT=1 env-var check.
// Keep the literal "1" here so the convention is searchable and consistent.
export function isAgentEnv(): boolean {
  return process.env.SKILLTAP_AGENT === "1";
}
