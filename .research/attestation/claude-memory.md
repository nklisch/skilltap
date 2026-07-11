---
source_handle: claude-memory
fetched: 2026-07-10
source_url: https://code.claude.com/docs/en/memory
provenance: source-direct
substrate_confidence: source-direct
---

# Claude Code instructions and memory

## Summary

Anthropic's memory guide defines `CLAUDE.md` files as persistent user-authored instructions and distinguishes them from Claude-authored auto memory. User instructions live at `~/.claude/CLAUDE.md`; project instructions may be `./CLAUDE.md` or `./.claude/CLAUDE.md`; local project instructions may be `./CLAUDE.local.md`.

Claude Code does not read `AGENTS.md` directly. The guide explicitly recommends a `CLAUDE.md` import of `AGENTS.md`, or a symlink when no Claude-specific additions are needed. Instruction discovery walks upward from the working directory and discovers subordinate instructions on demand.

## Anchored excerpts

**AGENTS.md, line 194:**

> Claude Code reads `CLAUDE.md`, not `AGENTS.md`.

**AGENTS.md, line 203:**

> A symlink also works if you don’t need to add Claude-specific content.

## Key passages and anchors

- **CLAUDE.md vs auto memory, lines 105-131:** instructions and auto memory are separate mechanisms; instructions are context rather than enforced policy.
- **Choose where files live, lines 145-160:** managed, user, project, and local instruction locations and purposes are defined; user instructions live at `~/.claude/CLAUDE.md`.
- **Imports, lines 175-187:** `@path` imports are expanded at launch; relative and absolute paths are supported; imports can recurse up to four hops; external project imports require approval.
- **AGENTS.md, lines 192-207:** Claude Code reads `CLAUDE.md`, not `AGENTS.md`; an importing `CLAUDE.md` or symlink is the prescribed bridge.
- **Loading order, lines 210-214:** startup discovery walks from filesystem root toward the working directory and concatenates files; subordinate files load when Claude accesses their directories.
- **Instruction quality, lines 165-172:** instructions consume context and work best when concise, structured, concrete, and internally consistent.

## Structural metadata

- Publisher: Anthropic
- Document type: normative product guide
- Surface: Claude Code persistent instructions
- Retrieval depth: full page with targeted line reads
