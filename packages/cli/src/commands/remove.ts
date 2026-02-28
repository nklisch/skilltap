import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "remove",
    description: "Remove an installed skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Name of installed skill",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Remove from project scope instead of global",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
  },
  async run({ args }) {
    console.log("skilltap remove: not yet implemented")
  },
})
