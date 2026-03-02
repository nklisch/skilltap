import { describe, expect, test } from "bun:test";
import { basicTemplate } from "./basic";
import { npmTemplate } from "./npm";
import { multiTemplate } from "./multi";

const OPTS = {
  name: "my-skill",
  description: "A test skill",
  license: "MIT",
  author: "Test User",
};

describe("basicTemplate", () => {
  test("generates SKILL.md with frontmatter", () => {
    const files = basicTemplate(OPTS);
    expect(files["SKILL.md"]).toBeDefined();
    expect(files["SKILL.md"]).toContain("name: my-skill");
    expect(files["SKILL.md"]).toContain("description: A test skill");
    expect(files["SKILL.md"]).toContain("license: MIT");
    expect(files["SKILL.md"]).toContain("author: Test User");
  });

  test("generates .gitignore", () => {
    const files = basicTemplate(OPTS);
    expect(files[".gitignore"]).toBeDefined();
    expect(files[".gitignore"]).toContain("node_modules/");
  });

  test("includes exactly SKILL.md and .gitignore", () => {
    const files = basicTemplate(OPTS);
    expect(Object.keys(files).sort()).toEqual([".gitignore", "SKILL.md"]);
  });

  test("omits license line when license is None", () => {
    const files = basicTemplate({ ...OPTS, license: "None" });
    expect(files["SKILL.md"]).not.toContain("license:");
  });
});

describe("npmTemplate", () => {
  test("generates SKILL.md, package.json, .gitignore, and workflow", () => {
    const files = npmTemplate(OPTS);
    expect(files["SKILL.md"]).toBeDefined();
    expect(files["package.json"]).toBeDefined();
    expect(files[".gitignore"]).toBeDefined();
    expect(files[".github/workflows/publish.yml"]).toBeDefined();
  });

  test("package.json includes agent-skill keyword", () => {
    const files = npmTemplate(OPTS);
    const pkg = JSON.parse(files["package.json"]!);
    expect(pkg.keywords).toContain("agent-skill");
  });

  test("package.json sets correct name and version", () => {
    const files = npmTemplate(OPTS);
    const pkg = JSON.parse(files["package.json"]!);
    expect(pkg.name).toBe("my-skill");
    expect(pkg.version).toBe("0.1.0");
    expect(pkg.description).toBe("A test skill");
    expect(pkg.license).toBe("MIT");
  });

  test("package.json includes files array with SKILL.md", () => {
    const files = npmTemplate(OPTS);
    const pkg = JSON.parse(files["package.json"]!);
    expect(pkg.files).toContain("SKILL.md");
  });

  test("publish.yml includes provenance step", () => {
    const files = npmTemplate(OPTS);
    const yml = files[".github/workflows/publish.yml"]!;
    expect(yml).toContain("--provenance");
    expect(yml).toContain("attest-build-provenance");
    expect(yml).toContain("id-token: write");
  });

  test("uses UNLICENSED when license is None", () => {
    const files = npmTemplate({ ...OPTS, license: "None" });
    const pkg = JSON.parse(files["package.json"]!);
    expect(pkg.license).toBe("UNLICENSED");
  });
});

describe("multiTemplate", () => {
  test("generates SKILL.md for each skill name", () => {
    const files = multiTemplate({
      description: "A collection",
      license: "MIT",
      author: "Test User",
      skillNames: ["skill-a", "skill-b"],
    });
    expect(files[".agents/skills/skill-a/SKILL.md"]).toBeDefined();
    expect(files[".agents/skills/skill-b/SKILL.md"]).toBeDefined();
  });

  test("each SKILL.md has the skill name in frontmatter", () => {
    const files = multiTemplate({
      description: "A collection",
      license: "MIT",
      author: "Test User",
      skillNames: ["my-alpha", "my-beta"],
    });
    expect(files[".agents/skills/my-alpha/SKILL.md"]).toContain("name: my-alpha");
    expect(files[".agents/skills/my-beta/SKILL.md"]).toContain("name: my-beta");
  });

  test("generates .gitignore", () => {
    const files = multiTemplate({
      description: "A collection",
      license: "MIT",
      author: "Test User",
      skillNames: ["skill-a"],
    });
    expect(files[".gitignore"]).toBeDefined();
  });

  test("single skill name works", () => {
    const files = multiTemplate({
      description: "Single",
      license: "MIT",
      author: "Test User",
      skillNames: ["only-skill"],
    });
    expect(files[".agents/skills/only-skill/SKILL.md"]).toBeDefined();
  });
});
