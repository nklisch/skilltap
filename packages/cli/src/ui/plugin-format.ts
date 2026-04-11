import type { PluginRecord } from "@skilltap/core";

export function componentSummary(record: PluginRecord): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of record.components) counts[c.type]++;
  const parts: string[] = [];
  if (counts.skill > 0) parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0) parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP" : "MCPs"}`);
  if (counts.agent > 0) parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.join(", ") || "no components";
}
