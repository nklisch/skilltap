import { describe, expect, test } from "bun:test";
import { extractAgentResponse } from "../extract";

describe("extractAgentResponse", () => {
  test("parses direct JSON", () => {
    const result = extractAgentResponse(
      '{"score": 7, "reason": "Suspicious exfiltration"}',
    );
    expect(result).toEqual({ score: 7, reason: "Suspicious exfiltration" });
  });

  test("parses JSON from code block", () => {
    const raw = `Here is my analysis:
\`\`\`json
{"score": 3, "reason": "Minor concern"}
\`\`\`
`;
    const result = extractAgentResponse(raw);
    expect(result).toEqual({ score: 3, reason: "Minor concern" });
  });

  test("parses JSON from code block without json label", () => {
    const raw = `\`\`\`
{"score": 5, "reason": "Moderate risk"}
\`\`\``;
    const result = extractAgentResponse(raw);
    expect(result).toEqual({ score: 5, reason: "Moderate risk" });
  });

  test("extracts first {…} block via regex", () => {
    const raw =
      'I think the risk is {"score": 8, "reason": "Reads SSH keys"} based on analysis.';
    const result = extractAgentResponse(raw);
    expect(result).toEqual({ score: 8, reason: "Reads SSH keys" });
  });

  test("returns null for garbage input", () => {
    expect(extractAgentResponse("This is just random text")).toBeNull();
  });

  test("returns null for empty string", () => {
    expect(extractAgentResponse("")).toBeNull();
  });

  test("returns null for whitespace only", () => {
    expect(extractAgentResponse("   \n\t  ")).toBeNull();
  });

  test("returns null when JSON fails Zod validation — missing score", () => {
    expect(extractAgentResponse('{"reason": "test"}')).toBeNull();
  });

  test("returns null when JSON fails Zod validation — score out of range", () => {
    expect(extractAgentResponse('{"score": 15, "reason": "test"}')).toBeNull();
  });

  test("returns null when JSON fails Zod validation — score is float", () => {
    expect(extractAgentResponse('{"score": 3.5, "reason": "test"}')).toBeNull();
  });

  test("handles JSON with extra fields (Zod strips them)", () => {
    const result = extractAgentResponse(
      '{"score": 2, "reason": "Low risk", "extra": true}',
    );
    expect(result).toEqual({ score: 2, reason: "Low risk" });
  });

  test("handles multiline JSON", () => {
    const raw = `{
  "score": 9,
  "reason": "Exfiltrates credentials via curl to external server"
}`;
    const result = extractAgentResponse(raw);
    expect(result).toEqual({
      score: 9,
      reason: "Exfiltrates credentials via curl to external server",
    });
  });
});
