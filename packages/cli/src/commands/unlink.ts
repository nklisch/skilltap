import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "unlink",
    description: "Remove a linked skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Name of linked skill",
      required: true,
    },
  },
  async run({ args }) {
    console.log("skilltap unlink: not yet implemented")
  },
})
