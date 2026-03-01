import { $ } from "bun";
import { createCliAdapter } from "./factory";

export const claudeAdapter = createCliAdapter(
  "Claude Code",
  "claude",
  (prompt) =>
    $`claude --print -p ${prompt} --no-tools --output-format json`.quiet(),
);

export const geminiAdapter = createCliAdapter(
  "Gemini CLI",
  "gemini",
  (prompt) => $`echo ${prompt} | gemini --non-interactive`.quiet(),
);

export const codexAdapter = createCliAdapter(
  "Codex CLI",
  "codex",
  (prompt) => $`codex --prompt ${prompt} --no-tools`.quiet(),
);

export const opencodeAdapter = createCliAdapter(
  "OpenCode",
  "opencode",
  (prompt) => $`opencode --prompt ${prompt}`.quiet(),
);
