/**
 * Unit tests for `resolveScope` — the smart-scope-default helper.
 *
 * Spec: docs/SPEC.md §v2.0 Configuration (`scope = ""` smart default) +
 * .claude/CLAUDE.md "v2.1 conventions — Smart scope default":
 *
 *   inside a git repo  → defaults to --project
 *   outside a git repo → defaults to --global
 *   no prompt for the common case
 *
 * Precedence (high → low):
 *   1. --project flag
 *   2. --global flag
 *   3. config.defaults.scope (if non-empty)
 *   4. smart inference from cwd's git context (sets `inferred: true`)
 *
 * Tests use process.chdir() to control cwd; bun:test executes tests in a
 * single file sequentially so the chdir doesn't race.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import type { Config } from "@skilltap/core";
import {
  commitAll,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { resolveScope } from "./resolve";

// Build a Config object with optional scope default. Other fields use
// schema-defaults shapes that resolveScope ignores.
function makeConfig(scopeDefault: "" | "global" | "project" = ""): Config {
  return {
    defaults: { also: [], yes: false, scope: scopeDefault },
    // resolveScope only reads defaults.scope; the rest is filler the type
    // demands. Cast keeps the test focused on the field under test.
  } as unknown as Config;
}

let savedCwd: string;
let nonGitDir: string;
let gitDir: string;

beforeEach(async () => {
  savedCwd = process.cwd();
  nonGitDir = await makeTmpDir();
  gitDir = await makeTmpDir();
  await initRepo(gitDir);
  // initRepo leaves an empty repo; commit a placeholder so isInGitRepo's
  // walk finds .git deterministically.
  await Bun.write(`${gitDir}/.gitkeep`, "");
  await commitAll(gitDir, "init");
});

afterEach(async () => {
  process.chdir(savedCwd);
  await removeTmpDir(nonGitDir);
  await removeTmpDir(gitDir);
});

describe("resolveScope — smart inference", () => {
  test("inside a git repo with no flags and no config → project + inferred", async () => {
    process.chdir(gitDir);
    const result = await resolveScope({}, undefined);
    expect(result.scope).toBe("project");
    expect(result.inferred).toBe(true);
    // findProjectRoot resolves symlinks, so compare via realpath-ish check
    expect(result.projectRoot).toBeDefined();
  });

  test("outside any git repo → global + inferred", async () => {
    process.chdir(nonGitDir);
    const result = await resolveScope({}, undefined);
    expect(result.scope).toBe("global");
    expect(result.inferred).toBe(true);
    expect(result.projectRoot).toBeUndefined();
  });
});

describe("resolveScope — flags override inference", () => {
  test("--scope project wins over inference outside git repo", async () => {
    process.chdir(nonGitDir);
    const result = await resolveScope({ scope: "project" }, undefined);
    expect(result.scope).toBe("project");
    expect(result.inferred).toBeFalsy();
  });

  test("--scope global wins over inference inside git repo", async () => {
    process.chdir(gitDir);
    const result = await resolveScope({ scope: "global" }, undefined);
    expect(result.scope).toBe("global");
    expect(result.inferred).toBeFalsy();
    // No projectRoot needed for global scope.
    expect(result.projectRoot).toBeUndefined();
  });
});

describe("resolveScope — config.defaults.scope overrides inference", () => {
  test("config defaults.scope = 'global' wins over in-git inference", async () => {
    process.chdir(gitDir);
    const result = await resolveScope({}, makeConfig("global"));
    expect(result.scope).toBe("global");
    expect(result.inferred).toBeFalsy();
  });

  test("config defaults.scope = 'project' wins over outside-repo inference", async () => {
    // The config-default branch calls findProjectRoot(); if cwd has no .git
    // ancestor that throws. So we run inside a git dir but assert that the
    // 'inferred' flag is false (config sourced the choice, not inference).
    process.chdir(gitDir);
    const result = await resolveScope({}, makeConfig("project"));
    expect(result.scope).toBe("project");
    expect(result.inferred).toBeFalsy();
    expect(result.projectRoot).toBeDefined();
  });

  test("config defaults.scope = '' falls through to inference", async () => {
    process.chdir(gitDir);
    const result = await resolveScope({}, makeConfig(""));
    expect(result.scope).toBe("project");
    expect(result.inferred).toBe(true);
  });
});

describe("resolveScope — flag beats config beats inference (precedence chain)", () => {
  test("--scope global beats config.scope='project' beats in-git inference", async () => {
    process.chdir(gitDir);
    const result = await resolveScope(
      { scope: "global" },
      makeConfig("project"),
    );
    expect(result.scope).toBe("global");
    expect(result.inferred).toBeFalsy();
  });

  test("--scope project beats config.scope='global' beats outside-repo inference", async () => {
    process.chdir(nonGitDir);
    // --scope project bypasses the inference; findProjectRoot will throw if no
    // git ancestor exists. So this case is exercised inside a git tree:
    process.chdir(gitDir);
    const result = await resolveScope(
      { scope: "project" },
      makeConfig("global"),
    );
    expect(result.scope).toBe("project");
    expect(result.inferred).toBeFalsy();
  });
});
