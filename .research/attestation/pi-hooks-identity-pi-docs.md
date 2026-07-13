---
source_handle: pi-hooks-identity-pi-docs
fetched: 2026-07-12
source_url: https://pi.dev/docs/latest/packages
provenance: source-direct
substrate_confidence: source-direct
---

# Pi official documentation — packages, gallery, and hooks surface

Source-direct attestation of the official Pi documentation site
(`pi.dev`, published by Earendil Inc.) regarding package installation,
the community package gallery, and the (absent) native hooks surface. Read
off the fetched HTML.

## `https://pi.dev/docs/latest/hooks` — does not exist

Fetching `https://pi.dev/docs/latest/hooks` returns the Pi site's generic
"Page Not Found" body (title `Page Not Found · Pi`, with the line "There are
many pages, but this one does not exist"). Pi's official documentation
**does not document a native hook system** at this URL. The implication is
that Pi exposes lifecycle behavior through extension events consumed by
extensions, not through a Claude-style first-class `hooks` configuration
surface owned by core. There is no Pi-core hook contract for a third-party
package to be "compatible" with; compatibility is therefore always with
*Claude Code's* hook format, bridged onto Pi extension events.

## `https://pi.dev/docs/latest/packages` — exists

The "Pi Packages" page exists and documents:

- Install and Manage
- Package Sources: `npm`, `git`, `Local`
- (Package) Gallery
- Metadata
- Package Structure
- Convention Directories
- Dependencies
- Package Filtering
- Enable and Disable
- Resources
- Scope and Deduplication

The page text confirms `npm`, `git`, and local paths as the documented
package sources and describes a "Pi Package Gallery". It does **not** mention
`hooks`, `Claude`, `hsingjui`, or any recommended/official community package.
No package is singled out by Pi core documentation as the canonical
hook-compatibility extension.

## `https://pi.dev/packages` — community gallery (discovery, not authority)

The community package gallery exists and renders on the order of 101 package
entries on its first page (counted by the per-package "Copy" affordance).
The gallery surface includes package names, one-line descriptions,
publisher/maintainer handles, a downloads-per-month figure, and a
last-published-relative timestamp.

Substrings `hook` / `Hook` appear inside several *unrelated* package
descriptions on the gallery (for example, an entry whose description begins
"A coding agent CLI hook - block destructive git and filesystem commands
before execution" — a different package addressing destructive-command
blocking, not Claude Code hook compatibility).

The gallery's first page does **not** surface `@hsingjui/pi-hooks`, the
unscoped `pi-hooks`, the substring `claude-code-hooks`, or `@fyeeme/pi-hooks`.
The substring `ryan_nookpi` does appear, but for a different package from
that maintainer (a subagents/delegation package), not for
`@ryan_nookpi/pi-extension-claude-hooks-bridge`.

The gallery is a discovery surface with download-derived ordering, not an
authoritative registry. Absence from the first page is not evidence of
nonexistence; it is evidence that the package is not among the
highest-downloaded entries and is not editorially featured.

## Synthesis of the official surface

The official Pi surface establishes three things relevant to package identity:

1. **npm is a documented install source** — Pi core documents `npm` as a
   first-class package source, so `pi install npm:@hsingjui/pi-hooks` is a
   Pi-supported installation path and the npm registry is the canonical
   install identity.
2. **No Pi-core hook contract exists** — there is no native hooks
   documentation page; a "hook-compatibility" package is by definition
   bridging Claude Code's format onto Pi extension events, and Pi core
   endorses no specific bridge.
3. **No official endorsement** — neither the packages documentation nor the
   community gallery features any package as the canonical Claude
   hook-compatibility extension. Selection among community packages is a
   consumer decision, not a Pi-vouchsafed one.
