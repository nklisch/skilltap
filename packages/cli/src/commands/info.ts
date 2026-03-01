import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "info",
    description: "Show details about an installed or available skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name",
      required: true,
    },
  },
  async run(_ctx) {},
});
