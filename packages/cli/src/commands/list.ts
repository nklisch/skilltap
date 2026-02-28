import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "list",
    description: "List installed skills",
  },
  args: {
    global: {
      type: "boolean",
      description: "Show only global skills",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Show only project skills",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    console.log("skilltap list: not yet implemented")
  },
})
