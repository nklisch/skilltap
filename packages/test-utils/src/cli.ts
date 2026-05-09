const CLI_ENTRY = `${import.meta.dir}/../../cli/src/index.ts`;

/**
 * Returns the argv prefix used to invoke skilltap from a test.
 *
 * - If `SKILLTAP_TEST_BIN` is set to a path, tests run against that compiled
 *   binary (used by the binary smoke + integration suite).
 * - Otherwise tests run `bun run --bun packages/cli/src/index.ts` against
 *   live source (the default; matches dev workflow).
 */
export function cliCmd(): string[] {
  const bin = process.env.SKILLTAP_TEST_BIN;
  if (bin && bin.length > 0) return [bin];
  return ["bun", "run", "--bun", CLI_ENTRY];
}

export async function runSkilltap(
  args: string[],
  homeDir: string,
  configDir: string,
  cwd: string = homeDir,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn([...cliCmd(), ...args], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      SKILLTAP_HOME: homeDir,
      XDG_CONFIG_HOME: configDir,
      SKILLTAP_NO_STARTUP: "1",
    },
  });
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}
