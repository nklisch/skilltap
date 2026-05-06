import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdtemp, rm, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parse } from "smol-toml";
import {
  addPluginToManifest,
  addSkillToManifest,
  canonicalizeSourceKey,
  removePluginFromManifest,
  removeSkillFromManifest,
} from "./update";

let projectRoot: string;
beforeEach(async () => {
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-mfst-update-"));
});
afterEach(async () => {
  await rm(projectRoot, { recursive: true, force: true });
});

describe("canonicalizeSourceKey", () => {
  test("https github → github: shorthand", () => {
    expect(canonicalizeSourceKey("https://github.com/n/r")).toBe("github:n/r");
  });

  test("https github with .git → github: shorthand", () => {
    expect(canonicalizeSourceKey("https://github.com/n/r.git")).toBe("github:n/r");
  });

  test("ssh github with .git → github: shorthand", () => {
    expect(canonicalizeSourceKey("git@github.com:n/r.git")).toBe("github:n/r");
  });

  test("ssh github without .git → github: shorthand", () => {
    expect(canonicalizeSourceKey("git@github.com:n/r")).toBe("github:n/r");
  });

  test("npm: passthrough", () => {
    expect(canonicalizeSourceKey("npm:@scope/code-review")).toBe("npm:@scope/code-review");
  });

  test("npm versioned: passthrough", () => {
    expect(canonicalizeSourceKey("npm:@scope/code-review@1.2.3")).toBe(
      "npm:@scope/code-review@1.2.3",
    );
  });

  test("non-github URL: passthrough", () => {
    expect(canonicalizeSourceKey("https://gitlab.com/n/r.git")).toBe(
      "https://gitlab.com/n/r.git",
    );
  });

  test("local path: passthrough", () => {
    expect(canonicalizeSourceKey("/Users/n/skill")).toBe("/Users/n/skill");
  });

  test("ssh non-github: passthrough", () => {
    expect(canonicalizeSourceKey("git@example.com:n/r.git")).toBe("git@example.com:n/r.git");
  });
});

describe("addSkillToManifest", () => {
  test("no-op when skilltap.toml is absent", async () => {
    const result = await addSkillToManifest(projectRoot, {
      source: "https://github.com/n/r",
      ref: "v1.0",
      sha: "abc123",
    });
    expect(result.ok).toBe(true);
    // No file should have been created
    const file = Bun.file(join(projectRoot, "skilltap.toml"));
    expect(await file.exists()).toBe(false);
  });

  test("appends to fresh manifest", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), `[targets]\nalso = []\n`);
    const result = await addSkillToManifest(projectRoot, {
      source: "https://github.com/n/r",
      ref: "v1.2.0",
      sha: "abc123def",
    });
    expect(result.ok).toBe(true);
    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    const parsed = parse(text) as { skills?: Record<string, string> };
    expect(parsed.skills?.["github:n/r"]).toBe("*");

    const lockText = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
    const lock = parse(lockText) as {
      skill?: Array<{ source: string; ref: string; sha?: string; range: string }>;
    };
    expect(lock.skill).toHaveLength(1);
    expect(lock.skill?.[0]).toMatchObject({
      source: "github:n/r",
      ref: "v1.2.0",
      sha: "abc123def",
      range: "*",
    });
  });

  test("re-running updates existing lockfile entry (no duplicate)", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    const input1 = {
      source: "https://github.com/n/r",
      ref: "v1.0.0",
      sha: "old-sha",
    };
    const input2 = {
      source: "https://github.com/n/r",
      ref: "v2.0.0",
      sha: "new-sha",
    };
    expect((await addSkillToManifest(projectRoot, input1)).ok).toBe(true);
    expect((await addSkillToManifest(projectRoot, input2)).ok).toBe(true);

    const lockText = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
    const lock = parse(lockText) as {
      skill?: Array<{ source: string; ref: string; sha?: string }>;
    };
    expect(lock.skill).toHaveLength(1);
    expect(lock.skill?.[0]?.ref).toBe("v2.0.0");
    expect(lock.skill?.[0]?.sha).toBe("new-sha");
  });

  test("uses canonical source key in manifest + lockfile", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await addSkillToManifest(projectRoot, {
      source: "git@github.com:n/r.git",
      ref: "main",
      sha: "abc",
    });
    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    expect(text).toContain("github:n/r");
    expect(text).not.toContain("git@github.com:n/r.git");
  });

  test("custom range overrides default '*'", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await addSkillToManifest(projectRoot, {
      source: "https://github.com/n/r",
      ref: "v1.5.0",
      sha: "abc",
      range: "^1.0",
    });
    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    expect(text).toMatch(/github:n\/r"?\s*=\s*"\^1\.0"/);
  });
});

describe("removeSkillFromManifest", () => {
  test("no-op when skilltap.toml is absent", async () => {
    const result = await removeSkillFromManifest(projectRoot, "https://github.com/n/r");
    expect(result.ok).toBe(true);
  });

  test("drops entry from [skills] table and lockfile", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await addSkillToManifest(projectRoot, {
      source: "https://github.com/n/r",
      ref: "v1",
      sha: "abc",
    });
    const before = parse(
      await readFile(join(projectRoot, "skilltap.toml"), "utf8"),
    ) as { skills?: Record<string, string> };
    expect(before.skills?.["github:n/r"]).toBe("*");

    const result = await removeSkillFromManifest(projectRoot, "https://github.com/n/r");
    expect(result.ok).toBe(true);

    const after = parse(
      await readFile(join(projectRoot, "skilltap.toml"), "utf8"),
    ) as { skills?: Record<string, string> };
    expect(after.skills?.["github:n/r"]).toBeUndefined();

    const lockText = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
    const lock = parse(lockText) as { skill?: Array<{ source: string }> };
    expect(lock.skill).toEqual([]);
  });

  test("removing a non-existent entry is a no-op (no error)", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    const result = await removeSkillFromManifest(projectRoot, "https://github.com/missing/x");
    expect(result.ok).toBe(true);
  });

  test("uses canonical source key for lookup", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await addSkillToManifest(projectRoot, {
      source: "https://github.com/n/r",
      ref: "v1",
      sha: "abc",
    });
    const result = await removeSkillFromManifest(projectRoot, "git@github.com:n/r.git");
    expect(result.ok).toBe(true);
    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    expect(text).not.toContain("github:n/r");
  });
});

describe("removePluginFromManifest", () => {
  test("drops entry from [plugins] and lockfile.plugin[]", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await addPluginToManifest(projectRoot, {
      source: "https://github.com/c/dev-toolkit",
      ref: "v2",
      sha: "abc",
    });
    const result = await removePluginFromManifest(
      projectRoot,
      "https://github.com/c/dev-toolkit",
    );
    expect(result.ok).toBe(true);

    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    expect(text).not.toContain("github:c/dev-toolkit");

    const lockText = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
    const lock = parse(lockText) as { plugin?: Array<unknown> };
    expect(lock.plugin).toEqual([]);
  });
});

describe("addPluginToManifest", () => {
  test("appends to [plugins] table and lockfile.plugin[]", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    const result = await addPluginToManifest(projectRoot, {
      source: "https://github.com/c/dev-toolkit",
      ref: "v2.1",
      sha: "feedface",
    });
    expect(result.ok).toBe(true);

    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    const parsed = parse(text) as { plugins?: Record<string, string> };
    expect(parsed.plugins?.["github:c/dev-toolkit"]).toBe("*");

    const lockText = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
    const lock = parse(lockText) as {
      skill?: Array<unknown>;
      plugin?: Array<{ source: string; ref: string; sha?: string }>;
    };
    expect(lock.skill).toEqual([]);
    expect(lock.plugin).toHaveLength(1);
    expect(lock.plugin?.[0]).toMatchObject({
      source: "github:c/dev-toolkit",
      ref: "v2.1",
      sha: "feedface",
    });
  });

  test("plugin and skill entries don't conflict", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await addSkillToManifest(projectRoot, {
      source: "https://github.com/n/skill",
      ref: "v1",
      sha: "a",
    });
    await addPluginToManifest(projectRoot, {
      source: "https://github.com/c/plugin",
      ref: "main",
      sha: "b",
    });

    const text = await readFile(join(projectRoot, "skilltap.toml"), "utf8");
    const parsed = parse(text) as {
      skills?: Record<string, string>;
      plugins?: Record<string, string>;
    };
    expect(Object.keys(parsed.skills ?? {})).toEqual(["github:n/skill"]);
    expect(Object.keys(parsed.plugins ?? {})).toEqual(["github:c/plugin"]);

    const lockText = await readFile(join(projectRoot, "skilltap.lock"), "utf8");
    const lock = parse(lockText) as {
      skill?: Array<{ source: string }>;
      plugin?: Array<{ source: string }>;
    };
    expect(lock.skill).toHaveLength(1);
    expect(lock.plugin).toHaveLength(1);
  });
});
