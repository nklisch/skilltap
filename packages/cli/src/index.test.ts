import { describe, expect, test } from "bun:test";

describe("skilltap CLI", () => {
  test("--help exits with code 0", async () => {
    const proc = Bun.spawn(["bun", "run", "--bun", "src/index.ts", "--help"], {
      cwd: `${import.meta.dir}/..`,
      stdout: "pipe",
      stderr: "pipe",
    });
    const exitCode = await proc.exited;
    const stdout = await new Response(proc.stdout).text();
    expect(exitCode).toBe(0);
    expect(stdout).toContain("skilltap");
    expect(stdout).toContain("install");
  });
});
