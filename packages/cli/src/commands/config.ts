import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "config",
    description: "Interactive setup wizard",
  },
  args: {
    reset: {
      type: "boolean",
      description: "Overwrite existing config",
      default: false,
    },
  },
  subCommands: {
    "agent-mode": () =>
      import("./config/agent-mode").then((m) => m.default),
  },
  async run({ args }) {
    console.log("skilltap config: not yet implemented")
  },
})
