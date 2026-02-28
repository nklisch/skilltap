import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "update",
    description: "Update tap repo(s)",
  },
  args: {
    name: {
      type: "positional",
      description: "Specific tap to update (omit to update all)",
    },
  },
  async run({ args }) {
    console.log("skilltap tap update: not yet implemented")
  },
})
