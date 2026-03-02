---
sitemap: false
---

# skilltap website

VitePress site for [skilltap.dev](https://skilltap.dev).

## Development

```bash
bun install          # from repo root
bun run --cwd website dev
```

Dev server starts at `http://localhost:5173`.

## Build & Preview

```bash
bun run --cwd website build
bun run --cwd website preview
```

## Deployment

Pushes to `main` that touch `website/**` auto-deploy to GitHub Pages via `.github/workflows/deploy.yml`.

## Structure

```
.vitepress/
  config.ts              Site config (nav, sidebar, meta)
  theme/
    index.ts             Theme entry (extends default)
    Layout.vue           Routes landing page vs docs
    custom.css            Amber palette, dark mode overrides
    components/           Landing page components
public/
  favicon.svg
  CNAME                  skilltap.dev
guide/                   User-facing docs
reference/               Technical reference
index.md                 Landing page (layout: landing)
```
