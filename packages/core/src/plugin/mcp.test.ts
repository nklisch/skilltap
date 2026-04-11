import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { parseMcpJson, parseMcpObject } from "./mcp";

async function writeTmpJson(dir: string, name: string, content: unknown): Promise<string> {
  const path = join(dir, name);
  await Bun.write(path, JSON.stringify(content));
  return path;
}

describe("parseMcpJson", () => {
  test("parses flat format (server entries at top level)", async () => {
    const dir = await makeTmpDir();
    try {
      const path = await writeTmpJson(dir, ".mcp.json", {
        db: { command: "npx", args: ["-y", "db-mcp"] },
      });
      const result = await parseMcpJson(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(1);
      const entry = result.value[0]!;
      expect(entry.type).toBe("stdio");
      expect(entry.name).toBe("db");
      if (entry.type === "stdio") {
        expect(entry.command).toBe("npx");
        expect(entry.args).toEqual(["-y", "db-mcp"]);
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("parses wrapped format (under mcpServers key)", async () => {
    const dir = await makeTmpDir();
    try {
      const path = await writeTmpJson(dir, ".mcp.json", {
        mcpServers: {
          db: { command: "npx", args: ["-y", "db-mcp"] },
        },
      });
      const result = await parseMcpJson(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(1);
      expect(result.value[0]?.name).toBe("db");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("handles mixed stdio and http servers", async () => {
    const dir = await makeTmpDir();
    try {
      const path = await writeTmpJson(dir, ".mcp.json", {
        "local-db": { command: "node", args: ["server.js"] },
        "remote-api": { type: "http", url: "https://api.example.com/mcp" },
      });
      const result = await parseMcpJson(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(2);
      const types = result.value.map((e) => e.type).sort();
      expect(types).toEqual(["http", "stdio"]);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns ok([]) for non-existent file", async () => {
    const result = await parseMcpJson("/tmp/this-file-does-not-exist-skilltap.json");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });

  test("returns err for invalid JSON", async () => {
    const dir = await makeTmpDir();
    try {
      const path = join(dir, ".mcp.json");
      await Bun.write(path, "{ not valid json }");
      const result = await parseMcpJson(path);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("preserves env dict", async () => {
    const dir = await makeTmpDir();
    try {
      const path = await writeTmpJson(dir, ".mcp.json", {
        db: { command: "npx", args: [], env: { TOKEN: "secret", DB_URL: "postgres://..." } },
      });
      const result = await parseMcpJson(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const entry = result.value[0]!;
      if (entry.type === "stdio") {
        expect(entry.env).toEqual({ TOKEN: "secret", DB_URL: "postgres://..." });
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns ok([]) for empty object {}", async () => {
    const dir = await makeTmpDir();
    try {
      const path = await writeTmpJson(dir, ".mcp.json", {});
      const result = await parseMcpJson(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toEqual([]);
    } finally {
      await removeTmpDir(dir);
    }
  });
});

describe("parseMcpObject", () => {
  test("parses server entries from object", () => {
    const result = parseMcpObject({
      db: { command: "npx", args: ["-y", "db-mcp"] },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(1);
    expect(result.value[0]?.name).toBe("db");
    expect(result.value[0]?.type).toBe("stdio");
  });

  test("handles empty object", () => {
    const result = parseMcpObject({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });

  test("returns err for invalid server entry", () => {
    // A server with type "http" but no url is invalid
    const result = parseMcpObject({
      broken: { type: "http" },
    });
    expect(result.ok).toBe(false);
  });
});
