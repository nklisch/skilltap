import { describe, expect, test } from "bun:test";
import { version as pkgVersion } from "../package.json";
import {
  AgentModeSchema,
  AgentResponseSchema,
  // schemas
  ConfigSchema,
  EXIT_CANCELLED,
  EXIT_ERROR,
  EXIT_SUCCESS,
  // config functions
  ensureDirs,
  err,
  GitError,
  InstalledJsonSchema,
  InstalledSkillSchema,
  loadConfig,
  loadInstalled,
  NetworkError,
  // types
  ok,
  ResolvedSourceSchema,
  ScanError,
  SecurityConfigSchema,
  SkillFrontmatterSchema,
  SkilltapError,
  saveConfig,
  saveInstalled,
  TapSchema,
  TapSkillSchema,
  UserError,
  VERSION,
} from "./index";

describe("@skilltap/core", () => {
  test("exports VERSION", () => {
    expect(VERSION).toBe(pkgVersion);
  });

  test("exports Result helpers", () => {
    expect(ok("x").ok).toBe(true);
    expect(err(new Error("x")).ok).toBe(false);
  });

  test("exports error classes", () => {
    expect(new SkilltapError("x")).toBeInstanceOf(Error);
    expect(new UserError("x")).toBeInstanceOf(SkilltapError);
    expect(new GitError("x")).toBeInstanceOf(SkilltapError);
    expect(new ScanError("x")).toBeInstanceOf(SkilltapError);
    expect(new NetworkError("x")).toBeInstanceOf(SkilltapError);
  });

  test("exports exit codes", () => {
    expect(EXIT_SUCCESS).toBe(0);
    expect(EXIT_ERROR).toBe(1);
    expect(EXIT_CANCELLED).toBe(2);
  });

  test("exports all schemas", () => {
    expect(ConfigSchema).toBeDefined();
    expect(SecurityConfigSchema).toBeDefined();
    expect(AgentModeSchema).toBeDefined();
    expect(InstalledSkillSchema).toBeDefined();
    expect(InstalledJsonSchema).toBeDefined();
    expect(TapSchema).toBeDefined();
    expect(TapSkillSchema).toBeDefined();
    expect(SkillFrontmatterSchema).toBeDefined();
    expect(AgentResponseSchema).toBeDefined();
    expect(ResolvedSourceSchema).toBeDefined();
  });

  test("exports config functions", () => {
    expect(typeof ensureDirs).toBe("function");
    expect(typeof loadConfig).toBe("function");
    expect(typeof saveConfig).toBe("function");
    expect(typeof loadInstalled).toBe("function");
    expect(typeof saveInstalled).toBe("function");
  });
});
