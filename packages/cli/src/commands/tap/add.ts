import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "add",
    description: "Add a tap",
  },
  args: {
    name: {
      type: "positional",
      description: "Local name for this tap",
      required: true,
    },
    url: {
      type: "positional",
      description: "Git URL of the tap repo",
      required: true,
    },
  },
  async run({ args }) {
    console.log("skilltap tap add: not yet implemented")
  },
})
