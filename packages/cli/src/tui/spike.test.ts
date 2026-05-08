import { describe, expect, test } from "bun:test";
import { runInteractive } from "@skilltap/test-utils";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const CLI_DIR = dirname(fileURLToPath(import.meta.url));
const SPIKE_PATH = join(CLI_DIR, "spike.tsx");

describe("Ink-on-Bun Spike", () => {
  test("renders hello-world Ink output", async () => {
    const session = await runInteractive(["bun", "run", SPIKE_PATH], {});
    await session.waitForText("Ink renders under Bun", 5000);
    session.send("q");
    const { exitCode } = await session.finish(5000);
    expect(exitCode).toBe(0);
  });
});
