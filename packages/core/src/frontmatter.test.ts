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
