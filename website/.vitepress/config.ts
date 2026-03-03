import { defineConfig, type HeadConfig } from "vitepress";

const SITE_URL = "https://skilltap.dev";
const OG_TITLE = "skilltap — Homebrew taps for AI agent skills";
const OG_DESC =
  "Install agent skills from any git host. Agent-agnostic, multi-source, secure.";
const OG_IMAGE = `${SITE_URL}/og-image.png`;

export default defineConfig({
  title: "skilltap",
  description:
    "Install agent skills from any git host. Homebrew taps for AI agent skills.",
  cleanUrls: true,
  lastUpdated: true,
  appearance: "force-dark",

  sitemap: {
    hostname: SITE_URL,
    transformItems: (items) => items.filter((item) => !item.url.includes("README")),
  },

  head: [
    ["link", { rel: "icon", href: "/favicon.svg" }],

    // Google Analytics
    ["script", { async: "", src: "https://www.googletagmanager.com/gtag/js?id=G-P45TWRKXC4" }],
    ["script", {}, `window.dataLayer = window.dataLayer || [];
function gtag(){dataLayer.push(arguments);}
gtag('js', new Date());
gtag('config', 'G-P45TWRKXC4');`],

    // Open Graph
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: "skilltap" }],
    ["meta", { property: "og:locale", content: "en_US" }],
    ["meta", { property: "og:title", content: OG_TITLE }],
    ["meta", { property: "og:description", content: OG_DESC }],
    ["meta", { property: "og:url", content: SITE_URL }],
    ["meta", { property: "og:image", content: OG_IMAGE }],
    ["meta", { property: "og:image:width", content: "1200" }],
    ["meta", { property: "og:image:height", content: "630" }],
    ["meta", { property: "og:image:alt", content: OG_TITLE }],

    // Twitter / X
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:title", content: OG_TITLE }],
    ["meta", { name: "twitter:description", content: OG_DESC }],
    ["meta", { name: "twitter:image", content: OG_IMAGE }],

    // JSON-LD structured data
    [
      "script",
      { type: "application/ld+json" },
      JSON.stringify({
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        name: "skilltap",
        applicationCategory: "DeveloperApplication",
        operatingSystem: "Linux, macOS",
        description: OG_DESC,
        url: SITE_URL,
        downloadUrl: "https://github.com/nklisch/skilltap/releases",
        license: "https://opensource.org/licenses/MIT",
        codeRepository: "https://github.com/nklisch/skilltap",
        offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
      }),
    ],
  ],

  transformHead({ pageData }) {
    const path = pageData.relativePath
      .replace(/\.md$/, "")
      .replace(/(\/index|^index)$/, "");
    const url = `${SITE_URL}/${path}`;
    const heads: HeadConfig[] = [
      ["link", { rel: "canonical", href: url }],
      ["meta", { property: "og:url", content: url }],
    ];
    if (path.startsWith("guide/") || path.startsWith("reference/")) {
      heads.push([
        "script",
        { type: "application/ld+json" },
        JSON.stringify({
          "@context": "https://schema.org",
          "@type": "TechArticle",
          headline: pageData.title,
          description: pageData.description,
          url,
          isPartOf: { "@type": "WebSite", name: "skilltap", url: SITE_URL },
          ...(pageData.lastUpdated
            ? { dateModified: new Date(pageData.lastUpdated).toISOString() }
            : {}),
        }),
      ]);
    }
    return heads;
  },

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
            { text: "Teams & Organizations", link: "/guide/teams" },
          ],
        },
        {
          text: "Configuration",
          items: [
            { text: "Security", link: "/guide/security" },
            { text: "Configuration", link: "/guide/configuration" },
          ],
        },
        {
          text: "Tooling",
          items: [
            { text: "Doctor", link: "/guide/doctor" },
            { text: "Shell Completions", link: "/guide/shell-completions" },
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
