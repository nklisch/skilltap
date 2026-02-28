import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "init",
    description: "Create a new tap repo",
  },
  args: {
    name: {
      type: "positional",
      description: "Directory name for the new tap",
      required: true,
    },
  },
  async run({ args }) {
    console.log("skilltap tap init: not yet implemented")
  },
})
