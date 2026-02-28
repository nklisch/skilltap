import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "list",
    description: "List configured taps",
  },
  async run({ args }) {
    console.log("skilltap tap list: not yet implemented")
  },
})
