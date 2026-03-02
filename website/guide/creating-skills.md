# Creating Skills

A skill is a directory with a `SKILL.md` file. There is no build step, no manifest, no special tooling. If it has a `SKILL.md`, it's a skill.

## Scaffolding with `skilltap create`

The fastest way to start is with the interactive scaffolding command:

```bash
skilltap create
```

This prompts for a name, template, author, and description, then generates all the starter files.

For non-interactive use (scripts, CI):

```bash
skilltap create my-skill --template basic
```

### Templates

| Template | Description |
|----------|-------------|
| `basic` | Single skill: `SKILL.md`, `README.md`, `.gitignore` |
| `npm` | npm package: adds `package.json` (with `agent-skill` keyword) and a GitHub Actions publish workflow with provenance attestation |
| `multi` | Multi-skill repo: `.agents/skills/` structure with multiple skills |

After scaffolding, the command prints next steps — how to test locally, verify the skill, and publish it.

### The `npm` template

The npm template is for publishing skills to the npm registry. It generates a `package.json` pre-configured with the `agent-skill` keyword so your skill is discoverable via `skilltap find --npm`, and a `.github/workflows/publish.yml` that publishes with npm provenance on every release tag:

```bash
skilltap create my-skill --template npm
cd my-skill
# edit SKILL.md, then:
npm publish --provenance   # or push a tag to trigger the workflow
```

Skills published this way get a `✓ provenance` trust tier when installed.

### The `multi` template

The multi template generates a `.agents/skills/` layout for a repo that ships multiple skills. You'll be prompted for the skill names during create:

```bash
skilltap create my-project --template multi
# creates:
#   .agents/skills/my-skill-a/SKILL.md
#   .agents/skills/my-skill-b/SKILL.md
```

## Verifying a skill

Before publishing, run `skilltap verify` to validate your skill:

```bash
skilltap verify
```

Or point it at a specific path:

```bash
skilltap verify ./path/to/skill
```

This checks:
- `SKILL.md` exists
- Frontmatter is valid (required fields present, name format correct)
- `name` field matches the directory name
- No static security warnings (same checks as `skilltap install`)
- Directory is within the size limit

Output on success:

```
✓ commit-helper is valid

  SKILL.md   ✓
  name       ✓ matches directory
  security   ✓ no issues
  size       ✓ 4.2 KB (3 files)

tap.json snippet (to list this skill in a tap):

  {
    "name": "commit-helper",
    "description": "Generates conventional commit messages",
    "repo": "https://github.com/user/commit-helper",
    "tags": []
  }
```

Exit code `0` = valid, `1` = errors found.

### Using verify in CI

`skilltap verify --json` outputs machine-readable results:

```bash
skilltap verify --json
```

```json
{
  "valid": true,
  "issues": [],
  "fileCount": 3,
  "totalBytes": 4301
}
```

As a pre-push git hook (`.git/hooks/pre-push`):

```bash
#!/bin/sh
skilltap verify
```

## SKILL.md format

Every skill needs a `SKILL.md` file with YAML frontmatter:

```markdown
---
name: commit-helper
description: Generates conventional commit messages from staged changes.
license: MIT
---

## Instructions

When the user asks you to commit, analyze the staged changes with `git diff --cached`
and generate a commit message following the Conventional Commits specification.

Always use imperative mood in the subject line. Keep it under 72 characters.
```

### Required fields

| Field | Description |
|---|---|
| `name` | Unique skill identifier. Lowercase alphanumeric and hyphens only, 1-64 characters. |
| `description` | What the skill does. 1-1024 characters. |

### Optional fields

| Field | Description |
|---|---|
| `license` | License identifier (e.g. `MIT`, `Apache-2.0`). |
| `compatibility` | Free-text compatibility notes (e.g. `Requires Python 3.8+`). Max 500 characters. |
| `metadata` | Arbitrary key-value pairs for extra info. |

Full example with all fields:

```yaml
---
name: code-reviewer
description: Thorough code review with security focus and performance analysis.
license: MIT
compatibility: Works best with TypeScript and JavaScript projects
metadata:
  author: nathan
  version: "2.0"
  tags: review, security
---
```

### Name rules

The `name` field must:

- Be lowercase alphanumeric with hyphens (`a-z`, `0-9`, `-`)
- Be 1-64 characters long
- Not start or end with a hyphen
- Not contain consecutive hyphens

Valid: `commit-helper`, `code-review`, `my-skill-v2`

Invalid: `Commit-Helper`, `commit_helper`, `-bad-name`, `name--oops`

For multi-skill repos, the name must match the parent directory name (e.g., a skill at `.agents/skills/my-skill/SKILL.md` must have `name: my-skill`).

## Skill content

Everything after the frontmatter is the skill's instructions. Write whatever Markdown your target agents understand. Most skills include:

- **Instructions** -- what the agent should do when the skill is active
- **Rules** -- constraints, formatting requirements, conventions
- **Reference material** -- API docs, schema definitions, examples

You can include additional files alongside `SKILL.md`:

```
commit-helper/
  SKILL.md              # required
  REFERENCE.md          # optional supporting docs
  scripts/              # optional helper scripts
  templates/            # optional templates
  examples/             # optional examples
```

Agents typically read `SKILL.md` as the entry point. Some agents also scan for additional Markdown files in the skill directory.

## Standalone vs multi-skill repos

### Standalone skill repo

The simplest structure. `SKILL.md` at the repo root:

```
commit-helper/
  SKILL.md
  scripts/
    generate.sh
```

When someone runs `skilltap install user/commit-helper`, the entire repo becomes the installed skill. Git history is preserved, and `skilltap update` runs `git pull` directly.

### Multi-skill repo

A repo can contain multiple skills inside `.agents/skills/`:

```
my-project/
  src/
  tests/
  .agents/skills/
    my-project-dev/
      SKILL.md
    my-project-review/
      SKILL.md
```

This is useful for skills that live alongside the project they're designed for. When someone installs from this repo, skilltap finds both skills and prompts them to choose.

skilltap also discovers skills at agent-specific paths (`.claude/skills/*/SKILL.md`, `.cursor/skills/*/SKILL.md`, etc.), but `.agents/skills/` is the preferred location.

## Testing locally

Use `skilltap link` to symlink your skill into the install path during development:

```bash
cd ~/dev/my-skill
skilltap link . --also claude-code
```

```
✓ Linked my-skill → ~/.agents/skills/my-skill/
✓ Symlinked → ~/.claude/skills/my-skill/
```

This creates a symlink, not a copy. Any changes you make to your skill are immediately visible to the agent. Iterate on the `SKILL.md`, test with your agent, and repeat.

For project-scoped testing:

```bash
skilltap link .agents/skills/my-project-dev --project --also claude-code
```

When you're done developing, remove the link:

```bash
skilltap unlink my-skill
```

## Publishing

Push your skill to any git host:

```bash
cd ~/dev/my-skill
git init
git add -A
git commit -m "Initial skill"
git remote add origin https://gitea.example.com/user/my-skill
git push -u origin main
```

Others can now install it:

```bash
skilltap install https://gitea.example.com/user/my-skill
```

If it's on GitHub, they can use shorthand:

```bash
skilltap install user/my-skill
```

That's all there is to it. No registration, no publishing step, no approval process. If it's in a git repo and has a `SKILL.md`, it's installable.

## Adding to a tap

A tap is a curated index that lets people discover and install your skill by name instead of URL. To add your skill to a tap:

1. Create a tap (or use an existing one):

```bash
skilltap tap init my-tap
cd my-tap
```

2. Edit `tap.json` to list your skill:

```json
{
  "name": "my skills",
  "description": "My curated skill collection",
  "skills": [
    {
      "name": "commit-helper",
      "description": "Generates conventional commit messages",
      "repo": "https://gitea.example.com/user/commit-helper",
      "tags": ["git", "productivity"]
    }
  ]
}
```

3. Push the tap to a git host:

```bash
git add -A
git commit -m "Add commit-helper"
git remote add origin https://gitea.example.com/user/my-tap
git push -u origin main
```

Anyone who adds your tap can now install by name:

```bash
skilltap tap add friend https://gitea.example.com/user/my-tap
skilltap install commit-helper
```

For more on taps, see the [Taps](/guide/taps) guide.

## Security considerations

skilltap scans all files in your skill directory before anyone installs it. Keep these in mind to avoid false positives:

- Avoid unnecessary base64 content, hidden HTML, or `<details>` blocks
- Don't include binary files unless truly needed (they'll be flagged)
- Keep skill directories small -- the default size warning threshold is 50KB
- If you reference external URLs, prefer well-known domains

See the [Security](/guide/security) guide for details on what the scanner checks.
