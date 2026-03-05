const CLI_DIR = `${import.meta.dir}/../../cli`;

export async function runSkilltap(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", ...args],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        SKILLTAP_HOME: homeDir,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}
