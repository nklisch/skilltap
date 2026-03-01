import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "find",
    description: "Search taps for skills",
  },
  args: {
    query: {
      type: "positional",
      description:
        "Search term (fuzzy matched against name, description, tags)",
    },
    interactive: {
      type: "boolean",
      alias: "i",
      description: "Interactive fuzzy finder mode",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run(_ctx) {},
});
