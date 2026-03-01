import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "update",
    description: "Update installed skill(s)",
  },
  args: {
    name: {
      type: "positional",
      description: "Specific skill to update (omit to update all)",
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept clean updates",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Skip skills with security warnings in diff",
    },
  },
  async run(_ctx) {},
});
