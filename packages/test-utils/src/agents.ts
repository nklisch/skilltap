import { join } from "node:path";
import { $ } from "bun";
import { makeTmpDir, removeTmpDir } from "./tmp";

/**
 * Creates a mock agent binary (shell script) that echoes a fixed response and
 * exits with the given code. The binary is named "mock-agent" inside a unique
 * temp directory, so its directory can be prepended to PATH for detect() tests.
 *
 * Returns binaryPath and a cleanup function that removes the temp directory.
 */
export async function createMockAgentBinary(
  response: string,
  exitCode = 0,
): Promise<{ binaryPath: string; cleanup: () => Promise<void> }> {
  const tmpDir = await makeTmpDir();
  const responsePath = join(tmpDir, "response.txt");
  const binaryPath = join(tmpDir, "mock-agent");

  // Write response to a file so the script can cat it — avoids shell quoting issues.
  await Bun.write(responsePath, response);
  await Bun.write(
    binaryPath,
    `#!/bin/sh\ncat '${responsePath}'\nexit ${exitCode}\n`,
  );
  await $`chmod +x ${binaryPath}`.quiet();

  return {
    binaryPath,
    cleanup: () => removeTmpDir(tmpDir),
  };
}
