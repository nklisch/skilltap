import type { AgentResponse } from "../schemas/agent";
import { AgentResponseSchema } from "../schemas/agent";

/**
 * Extract a valid AgentResponse from raw agent output.
 * 4-step pipeline: direct JSON → code block → regex {…} → null on failure.
 */
export function extractAgentResponse(raw: string): AgentResponse | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  // Step 1: Try direct JSON.parse
  const direct = tryParse(trimmed);
  if (direct) return direct;

  // Step 2: Extract from ```json ... ``` code block
  const codeBlockMatch = /```(?:json)?\s*\n?([\s\S]*?)```/.exec(trimmed);
  if (codeBlockMatch?.[1]) {
    const parsed = tryParse(codeBlockMatch[1].trim());
    if (parsed) return parsed;
  }

  // Step 3: Extract first {…} block via regex
  const braceMatch = /\{[\s\S]*?\}/.exec(trimmed);
  if (braceMatch?.[0]) {
    const parsed = tryParse(braceMatch[0]);
    if (parsed) return parsed;
  }

  // Step 4: Failure
  return null;
}

function tryParse(text: string): AgentResponse | null {
  try {
    const parsed = JSON.parse(text);
    const result = AgentResponseSchema.safeParse(parsed);
    if (result.success) return result.data;
  } catch {
    // Not valid JSON
  }
  return null;
}
