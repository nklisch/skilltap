import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "install",
    description: "Install a skill from a URL, tap name, or local path",
  },
  args: {
    source: {
      type: "positional",
      description: "Git URL, github:owner/repo, tap skill name, or local path",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Install to .agents/skills/ in current project",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Install to ~/.agents/skills/",
      default: false,
    },
    also: {
      description: "Create symlink in agent-specific directory",
      valueHint: "agent",
    },
    ref: {
      description: "Branch or tag to install",
      valueHint: "ref",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept prompts",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Abort on any security warning",
    },
    "no-strict": {
      type: "boolean",
      description: "Override config on_warn=fail for this invocation",
    },
    semantic: {
      type: "boolean",
      description: "Force semantic scan",
      default: false,
    },
  },
  async run({ args }) {
    console.log("skilltap install: not yet implemented")
  },
})
