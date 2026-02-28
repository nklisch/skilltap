import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "link",
    description: "Symlink a local skill directory into the install path",
  },
  args: {
    path: {
      type: "positional",
      description: "Path to local skill directory (must contain SKILL.md)",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Link to project scope instead of global",
      default: false,
    },
    also: {
      description: "Also symlink to agent-specific directory",
      valueHint: "agent",
    },
  },
  async run({ args }) {
    console.log("skilltap link: not yet implemented")
  },
})
