/**
 * Reserved v2 policy module — built in Phase 31a as the planned target for
 * Phase 32 (full agent-mode cutover). Phase 32 was effectively subsumed by
 * Phase 31c-c-2c, which extended the v1 `composePolicy` in `core/src/policy.ts`
 * with agent-mode precedence (`flags.agent > SKILLTAP_AGENT > config`)
 * directly. As a result, `composeV2`/`composeV2ForSource` are not currently
 * wired into any CLI command — only its own unit tests reference them.
 *
 * The module is retained because it cleanly models a few v2-only concepts
 * (`--no-agent`, source-level `trust` glob matching, `EnvV2` separation) that
 * the v1 `composePolicy` does not support. A future v2.x phase can pick this
 * up if/when those concepts ship to the CLI surface.
 *
 * Until then, **do not import from `policy-v2/` in production code paths**.
 * The single source of truth for resolved policy is `core/src/policy.ts`.
 */
export { composeV2, composeV2ForSource } from "./compose";
export { isTrusted, trustMatches } from "./trust-glob";
export type {
  CliFlagsV2,
  EffectivePolicyV2,
  EnvV2,
  SourceForPolicy,
} from "./types";
