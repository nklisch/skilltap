# SKILL.md Format

SKILL.md is the file format for agent skills. It is a Markdown file with YAML frontmatter that provides structured metadata, followed by a body of instructions for the AI agent.

Every installable skill must contain a `SKILL.md` file. This is how skilltap identifies, validates, and describes skills.

## Structure

A SKILL.md file has two parts:

1. **Frontmatter** -- YAML metadata between `---` delimiters
2. **Body** -- Markdown instructions for the AI agent

```markdown
---
name: commit-helper
description: Generates conventional commit messages from staged changes.
license: MIT
compatibility: Works with any git repository
metadata:
  author: nathan
  version: "1.0"
---

## Instructions

When the user asks you to commit, examine the staged changes with
`git diff --cached` and generate a conventional commit message...
```

## Frontmatter Schema

The frontmatter is YAML between `---` delimiters at the top of the file. It is validated against the following schema:

| Field | Type | Required | Constraints | Description |
|-------|------|----------|-------------|-------------|
| `name` | string | Yes | 1-64 chars, lowercase alphanumeric + hyphens, no leading/trailing/consecutive hyphens. Must match `^[a-z0-9]+(-[a-z0-9]+)*$` | Unique identifier for the skill |
| `description` | string | Yes | 1-1024 chars | Human-readable summary of what the skill does |
| `license` | string | No | -- | License identifier (e.g., `MIT`, `Apache-2.0`) |
| `compatibility` | string | No | Max 500 chars | Runtime requirements or agent compatibility notes |
| `metadata` | object | No | String keys, any values | Arbitrary key-value pairs for additional metadata |

### Validation

- If frontmatter is missing or Zod validation fails, the skill is flagged with a warning but still offered for installation
- When validation fails, the directory name is used as the skill name
- The `name` field must match its parent directory name (e.g., a skill at `.agents/skills/commit-helper/SKILL.md` must have `name: commit-helper`)

### Valid Name Examples

```
commit-helper        # valid
code-review          # valid
my-skill-v2          # valid
a                    # valid (single char)
```

### Invalid Name Examples

```
Commit-Helper        # uppercase not allowed
commit_helper        # underscores not allowed
-commit-helper       # leading hyphen
commit--helper       # consecutive hyphens
commit-helper-       # trailing hyphen
```

## Body

Everything after the closing `---` is the body. This is Markdown content that the AI agent reads as instructions. There are no constraints on body format -- write whatever instructions your agent needs.

The body typically includes:

- When and how the skill should be used
- Step-by-step instructions for the agent
- Code examples, templates, or patterns to follow
- Constraints or rules the agent should observe

## Skill Discovery Algorithm

When skilltap clones a repo, it scans for SKILL.md files using the following priority order:

### 1. Root SKILL.md (standalone skill)

If `SKILL.md` exists at the repo root, the entire repo is treated as a single standalone skill. Steps 2-4 are skipped.

### 2. Standard path

```
.agents/skills/*/SKILL.md
```

Each match is a separate skill, named by its parent directory. This is the canonical location for multi-skill repos.

### 3. Agent-specific paths

```
.claude/skills/*/SKILL.md
.cursor/skills/*/SKILL.md
.codex/skills/*/SKILL.md
.gemini/skills/*/SKILL.md
.windsurf/skills/*/SKILL.md
```

Skills in agent-specific directories are also discovered. If a skill with the same name was already found in `.agents/skills/`, the `.agents/skills/` version takes precedence.

### 4. Deep scan

```
**/SKILL.md
```

If no skills were found in steps 1-3, skilltap performs a deep scan for any SKILL.md file in the repo tree. This requires user confirmation.

### Deduplication

If the same skill name is found via multiple paths, skilltap deduplicates by name. The `.agents/skills/` path is preferred over agent-specific paths.

## Examples

### Standalone Skill Repo

A repo with a single skill at the root:

```
commit-helper/
  SKILL.md
  scripts/
    helper.sh
  .git/
```

```markdown
---
name: commit-helper
description: Generates conventional commit messages from staged changes.
license: MIT
---

## When to use

When the user asks you to commit their changes or create a commit message.

## Instructions

1. Run `git diff --cached` to see staged changes
2. Analyze the diff to understand the nature of the change
3. Generate a commit message following the Conventional Commits format
4. Present the message to the user for approval
```

### Multi-Skill Repo

A repo with multiple skills under `.agents/skills/`:

```
termtube/
  .agents/
    skills/
      termtube-dev/
        SKILL.md
      termtube-review/
        SKILL.md
  README.md
  .git/
```

Each skill directory contains its own `SKILL.md` with independent frontmatter:

```markdown
---
name: termtube-dev
description: Development workflow for the termtube project.
---

## Instructions
...
```

```markdown
---
name: termtube-review
description: Code review checklist for termtube contributions.
---

## Instructions
...
```

When a user installs from this repo, skilltap discovers both skills and prompts the user to choose which to install (or install all with `--yes`).

## Installation Paths

After installation, skills are placed at:

| Scope | Path |
|-------|------|
| Global | `~/.agents/skills/{name}/` |
| Project | `{project}/.agents/skills/{name}/` |

Agent-specific symlinks (via `--also` or config) point to the canonical `.agents/skills/` location:

| Agent | Global Symlink | Project Symlink |
|-------|---------------|-----------------|
| `claude-code` | `~/.claude/skills/{name}/` | `.claude/skills/{name}/` |
| `cursor` | `~/.cursor/skills/{name}/` | `.cursor/skills/{name}/` |
| `codex` | `~/.codex/skills/{name}/` | `.codex/skills/{name}/` |
| `gemini` | `~/.gemini/skills/{name}/` | `.gemini/skills/{name}/` |
| `windsurf` | `~/.windsurf/skills/{name}/` | `.windsurf/skills/{name}/` |
