import { defineConfig } from "vitepress";

export default defineConfig({
  title: "skilltap",
  description:
    "Install agent skills from any git host. Homebrew taps for AI agent skills.",
  cleanUrls: true,
  lastUpdated: true,
  appearance: "force-dark",

  head: [
    ["link", { rel: "icon", href: "/favicon.svg" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:title", content: "skilltap" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "Install agent skills from any git host. Agent-agnostic, multi-source, secure.",
      },
    ],
    ["meta", { property: "og:url", content: "https://skilltap.dev" }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    [
      "link",
      {
        rel: "preconnect",
        href: "https://fonts.googleapis.com",
      },
    ],
    [
      "link",
      {
        rel: "preconnect",
        href: "https://fonts.gstatic.com",
        crossorigin: "",
      },
    ],
    [
      "link",
      {
        rel: "stylesheet",
        href: "https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap",
      },
    ],
  ],

  themeConfig: {
    logo: "/favicon.svg",
    siteTitle: "skilltap",

    nav: [
      {
        text: "Guide",
        link: "/guide/what-is-skilltap",
        activeMatch: "/guide/",
      },
      {
        text: "Reference",
        link: "/reference/cli",
        activeMatch: "/reference/",
      },
      {
        text: "GitHub",
        link: "https://github.com/nklisch/skilltap",
      },
    ],

    sidebar: {
      "/guide/": [
        {
          text: "Introduction",
          items: [
            { text: "What is skilltap?", link: "/guide/what-is-skilltap" },
            { text: "Getting Started", link: "/guide/getting-started" },
            { text: "Installation", link: "/guide/installation" },
          ],
        },
        {
          text: "Using skilltap",
          items: [
            { text: "Installing Skills", link: "/guide/installing-skills" },
            { text: "Creating Skills", link: "/guide/creating-skills" },
            { text: "Taps", link: "/guide/taps" },
          ],
        },
        {
          text: "Configuration",
          items: [
            { text: "Security", link: "/guide/security" },
            { text: "Configuration", link: "/guide/configuration" },
          ],
        },
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "CLI Commands", link: "/reference/cli" },
            { text: "SKILL.md Format", link: "/reference/skill-format" },
            { text: "tap.json Format", link: "/reference/tap-format" },
            { text: "Config Options", link: "/reference/config-options" },
          ],
        },
      ],
    },

    search: {
      provider: "local",
    },

    socialLinks: [
      { icon: "github", link: "https://github.com/nklisch/skilltap" },
    ],

    footer: {
      message: "Released under the MIT License.",
    },

    editLink: {
      pattern:
        "https://github.com/nklisch/skilltap/edit/main/website/:path",
      text: "Edit this page on GitHub",
    },
  },
});
