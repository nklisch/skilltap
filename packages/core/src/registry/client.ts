import { z } from "zod/v4";
import { err, NetworkError, ok, type Result, UserError } from "../types";
import {
  RegistryDetailResponseSchema,
  RegistryListResponseSchema,
} from "./types";
import type { RegistryDetailResponse, RegistrySkill } from "./types";

export interface RegistryAuth {
  token?: string;
  envVar?: string;
}

function resolveToken(auth: RegistryAuth): string | undefined {
  if (auth.token) return auth.token;
  if (auth.envVar) return process.env[auth.envVar];
  return undefined;
}

function makeHeaders(auth: RegistryAuth): Record<string, string> {
  const token = resolveToken(auth);
  const headers: Record<string, string> = { Accept: "application/json" };
  if (token) headers.Authorization = `Bearer ${token}`;
  return headers;
}

function handleHttpError(
  status: number,
  tapName: string,
): Result<never, UserError | NetworkError> {
  if (status === 401) {
    return err(
      new UserError(
        `Authentication required for registry '${tapName}'.`,
        `Set auth_token in config or auth_env for environment variable.`,
      ),
    );
  }
  if (status === 403) {
    return err(
      new UserError(`Authentication failed for registry '${tapName}'. Check your token.`),
    );
  }
  if (status === 429) {
    return err(
      new NetworkError(`Rate limited by registry '${tapName}'. Try again later.`),
    );
  }
  return err(new NetworkError(`Registry '${tapName}' returned HTTP ${status}.`));
}

export type FetchSkillListResult = {
  skills: RegistrySkill[];
  total?: number;
  cursor?: string;
};

/** Fetch skill list from an HTTP registry. Supports optional search and pagination. */
export async function fetchSkillList(
  baseUrl: string,
  tapName: string,
  auth: RegistryAuth,
  params?: { q?: string; limit?: number; cursor?: string },
): Promise<Result<FetchSkillListResult, UserError | NetworkError>> {
  const url = new URL(`${baseUrl.replace(/\/$/, "")}/skills`);
  if (params?.q) url.searchParams.set("q", params.q);
  if (params?.limit != null) url.searchParams.set("limit", String(params.limit));
  if (params?.cursor) url.searchParams.set("cursor", params.cursor);

  let response: Response;
  try {
    response = await fetch(url.toString(), { headers: makeHeaders(auth) });
  } catch {
    return err(
      new NetworkError(
        `Could not reach registry at '${baseUrl}'. Check your connection.`,
      ),
    );
  }

  if (!response.ok) {
    return handleHttpError(response.status, tapName);
  }

  let raw: unknown;
  try {
    raw = await response.json();
  } catch {
    return err(
      new NetworkError(
        `Registry at '${baseUrl}' returned invalid JSON. Expected skills list.`,
      ),
    );
  }

  const parsed = RegistryListResponseSchema.safeParse(raw);
  if (!parsed.success) {
    return err(
      new NetworkError(
        `Registry at '${baseUrl}' returned invalid JSON. Expected skills list.`,
      ),
    );
  }

  return ok(parsed.data);
}

/** Fetch skill detail from an HTTP registry. */
export async function fetchSkillDetail(
  baseUrl: string,
  tapName: string,
  skillName: string,
  auth: RegistryAuth,
): Promise<Result<RegistryDetailResponse, UserError | NetworkError>> {
  const url = `${baseUrl.replace(/\/$/, "")}/skills/${encodeURIComponent(skillName)}`;

  let response: Response;
  try {
    response = await fetch(url, { headers: makeHeaders(auth) });
  } catch {
    return err(
      new NetworkError(
        `Could not reach registry at '${baseUrl}'. Check your connection.`,
      ),
    );
  }

  if (response.status === 404) {
    return err(
      new UserError(`Skill '${skillName}' not found in registry '${tapName}'.`),
    );
  }

  if (!response.ok) {
    return handleHttpError(response.status, tapName);
  }

  let raw: unknown;
  try {
    raw = await response.json();
  } catch {
    return err(
      new NetworkError(
        `Registry at '${baseUrl}' returned invalid JSON for skill '${skillName}'.`,
      ),
    );
  }

  const parsed = RegistryDetailResponseSchema.safeParse(raw);
  if (!parsed.success) {
    const details = z.prettifyError(parsed.error);
    return err(
      new NetworkError(
        `Registry at '${baseUrl}' returned invalid response for skill '${skillName}': ${details}`,
      ),
    );
  }

  return ok(parsed.data);
}

/**
 * Auto-detect whether a URL points to an HTTP registry or a git repo.
 * Tries GET {url}/skills?limit=1. If the response is valid JSON with a skills array, it's HTTP.
 */
export async function detectTapType(url: string): Promise<"http" | "git"> {
  try {
    const response = await fetch(
      `${url.replace(/\/$/, "")}/skills?limit=1`,
      { headers: { Accept: "application/json" } },
    );
    if (!response.ok) return "git";
    const raw = await response.json();
    const parsed = RegistryListResponseSchema.safeParse(raw);
    return parsed.success ? "http" : "git";
  } catch {
    return "git";
  }
}
