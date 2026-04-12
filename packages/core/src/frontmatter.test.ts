import { describe, test, expect } from "bun:test";
import { parseSkillFrontmatter } from "./frontmatter";

describe("parseSkillFrontmatter", () => {
  test("parses simple single-line values", () => {
    const content = `---
name: my-skill
description: A short description
license: MIT
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result).toEqual({
      name: "my-skill",
      description: "A short description",
      license: "MIT",
    });
  });

  test("returns null when no frontmatter", () => {
    expect(parseSkillFrontmatter("No frontmatter here")).toBeNull();
  });

  test("coerces boolean values", () => {
    const result = parseSkillFrontmatter("---\nfoo: true\nbar: false\n---\n");
    expect(result?.foo).toBe(true);
    expect(result?.bar).toBe(false);
  });

  test("coerces numeric values", () => {
    const result = parseSkillFrontmatter("---\ncount: 42\n---\n");
    expect(result?.count).toBe(42);
  });

  test("parses folded block scalar (>)", () => {
    const content = `---
name: my-skill
description: >
  This is a long description
  that spans multiple lines.
license: MIT
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result?.description).toBe("This is a long description that spans multiple lines.");
    expect(result?.license).toBe("MIT");
  });

  test("parses literal block scalar (|)", () => {
    const content = `---
name: my-skill
description: |
  Line one.
  Line two.
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result?.description).toBe("Line one.\nLine two.");
  });

  test("folded block scalar with single line", () => {
    const content = `---
description: >
  Single line text.
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result?.description).toBe("Single line text.");
  });

  test("parses nested metadata object", () => {
    const content = `---
name: my-skill
description: A short description
metadata:
  author: John Doe
  version: 1.0.0
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result).toEqual({
      name: "my-skill",
      description: "A short description",
      metadata: {
        author: "John Doe",
        version: "1.0.0",
      },
    });
  });

  test("nested object followed by another key", () => {
    const content = `---
name: my-skill
metadata:
  author: Jane
license: MIT
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result?.metadata).toEqual({ author: "Jane" });
    expect(result?.license).toBe("MIT");
  });

  test("nested object with boolean and numeric values", () => {
    const content = `---
config:
  enabled: true
  count: 5
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result?.config).toEqual({ enabled: true, count: 5 });
  });

  test("block scalar followed by another key", () => {
    const content = `---
name: my-skill
description: >
  Multi-line
  description here.
license: Apache-2.0
---
`;
    const result = parseSkillFrontmatter(content);
    expect(result?.description).toBe("Multi-line description here.");
    expect(result?.license).toBe("Apache-2.0");
    expect(result?.name).toBe("my-skill");
  });
});
