#!/usr/bin/env bun
import { defineCommand, runMain } from "citty";

const main = defineCommand({
  meta: {
    name: "skilltap",
    version: "0.1.0",
    description: "Install agent skills from any git host",
  },
  subCommands: {
    install: () => import("./commands/install").then((m) => m.default),
    remove: () => import("./commands/remove").then((m) => m.default),
    list: () => import("./commands/list").then((m) => m.default),
    update: () => import("./commands/update").then((m) => m.default),
    find: () => import("./commands/find").then((m) => m.default),
    link: () => import("./commands/link").then((m) => m.default),
    unlink: () => import("./commands/unlink").then((m) => m.default),
    info: () => import("./commands/info").then((m) => m.default),
    config: () => import("./commands/config").then((m) => m.default),
    tap: defineCommand({
      meta: {
        name: "tap",
        description: "Manage taps",
      },
      subCommands: {
        add: () => import("./commands/tap/add").then((m) => m.default),
        remove: () => import("./commands/tap/remove").then((m) => m.default),
        list: () => import("./commands/tap/list").then((m) => m.default),
        update: () => import("./commands/tap/update").then((m) => m.default),
        init: () => import("./commands/tap/init").then((m) => m.default),
      },
    }),
  },
});

runMain(main);
