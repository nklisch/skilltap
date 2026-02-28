import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "remove",
    description: "Remove a tap",
  },
  args: {
    name: {
      type: "positional",
      description: "Tap name to remove",
      required: true,
    },
  },
  async run({ args }) {
    console.log("skilltap tap remove: not yet implemented")
  },
})
