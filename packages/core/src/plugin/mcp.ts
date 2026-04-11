import { err, ok, type Result, UserError } from "../types";
import { McpHttpServerSchema, McpStdioServerSchema, type McpServerEntry } from "../schemas/plugin";

function parseServerObject(
  serverName: string,
  serverConfig: unknown,
): { entry: McpServerEntry } | { error: string } {
  if (typeof serverConfig !== "object" || serverConfig === null) {
    return { error: `"${serverName}": expected object, got ${typeof serverConfig}` };
  }
  const raw = { name: serverName, ...(serverConfig as Record<string, unknown>) };

  if ("type" in raw && raw.type === "http") {
    const result = McpHttpServerSchema.safeParse(raw);
    if (!result.success) {
      return { error: `"${serverName}": invalid HTTP server` };
    }
    return { entry: result.data };
  }

  // Inject type: "stdio" if not explicitly set to "http"
  if (!("type" in raw) || raw.type !== "http") {
    raw.type = "stdio";
  }
  const result = McpStdioServerSchema.safeParse(raw);
  if (!result.success) {
    return { error: `"${serverName}": invalid stdio server — missing required field "command"` };
  }
  return { entry: result.data };
}

function parseServersRecord(
  servers: Record<string, unknown>,
): Result<McpServerEntry[], UserError> {
  const entries: McpServerEntry[] = [];
  const errors: string[] = [];

  for (const [name, config] of Object.entries(servers)) {
    const parsed = parseServerObject(name, config);
    if ("error" in parsed) {
      errors.push(parsed.error);
    } else {
      entries.push(parsed.entry);
    }
  }

  if (errors.length > 0) {
    return err(new UserError(`Invalid MCP server entries:\n${errors.join("\n")}`));
  }
  return ok(entries);
}

/**
 * Parse a .mcp.json file and return normalized MCP server entries.
 *
 * Handles two on-disk formats:
 * - Flat: { "name": { command, args } }
 * - Wrapped: { "mcpServers": { "name": { command, args } } }
 */
export async function parseMcpJson(
  filePath: string,
): Promise<Result<McpServerEntry[], UserError>> {
  const file = Bun.file(filePath);
  const exists = await file.exists();
  if (!exists) return ok([]);

  let text: string;
  try {
    text = await file.text();
  } catch {
    return err(new UserError(`Could not read MCP config file: ${filePath}`));
  }

  if (!text.trim()) return ok([]);

  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return err(new UserError(`Invalid JSON in MCP config file: ${filePath}`));
  }

  if (typeof parsed !== "object" || parsed === null) {
    return err(new UserError(`MCP config file must be a JSON object: ${filePath}`));
  }

  const obj = parsed as Record<string, unknown>;

  // Detect wrapped format: { "mcpServers": { ... } }
  if ("mcpServers" in obj && typeof obj.mcpServers === "object" && obj.mcpServers !== null && !Array.isArray(obj.mcpServers)) {
    return parseServersRecord(obj.mcpServers as Record<string, unknown>);
  }

  // Flat format: { "server-name": { ... } }
  return parseServersRecord(obj);
}

/**
 * Parse an inline mcpServers object from plugin.json.
 * Same format as the wrapped .mcp.json but passed directly.
 */
export function parseMcpObject(
  servers: Record<string, unknown>,
): Result<McpServerEntry[], UserError> {
  return parseServersRecord(servers);
}
