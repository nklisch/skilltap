---
id: idea-vitepress-security-upgrade
created: 2026-07-11
updated: 2026-07-11
tags: [security, dependencies]
---

Revisit the website's VitePress dependency when an audit-clean compatible
release is available. VitePress 1.6.4 currently resolves Vite and esbuild
versions covered by development-server advisories, and `npm audit` reports no
available fix. The deployed skilltap website is static; this follow-up is for
the local and CI website toolchain rather than the shipped Rust binary.
