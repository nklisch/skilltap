import { defineCommand } from "citty"

export default defineCommand({
  meta: {
    name: "agent-mode",
    description: "Enable or disable agent mode (interactive only)",
  },
  async run({ args }) {
    console.log("skilltap config agent-mode: not yet implemented")
  },
})
