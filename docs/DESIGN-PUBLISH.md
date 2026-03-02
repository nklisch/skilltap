# Design: `skilltap publish` and `skilltap create`

Two commands that close the author-side loop: `create` scaffolds a new skill, `publish` pushes it to npm or validates it for git distribution. Together they make skilltap a full authoring tool, not just a consumer.

## `skilltap create`

Scaffolds a new skill directory with the correct structure, valid frontmatter, and optional npm/CI boilerplate.

### Command

```
skilltap create [name] [flags]
```

**Arguments:**

| Argument | Required | Description |
|---|---|---|
| `name` | No | Skill name (kebab-case). Prompted if omitted. |

**Options:**

| Flag | Type | Default | Description |
|---|---|---|---|
| `--template` | string | `basic` | Template to use: `basic`, `npm`, `multi` |
| `--dir` | string | `./{name}` | Output directory |

### Templates

#### `basic` — Standalone git skill

The default. Creates a single-skill repo ready to push to any git host.

```
my-skill/
  SKILL.md
  .gitignore
```

`SKILL.md`:

```markdown
---
name: my-skill
description: [user-provided description]
license: MIT
metadata:
  author: [git user.name or prompted]
  version: "0.1.0"
---

## Instructions

[Describe what this skill does and when to use it.]

## Rules

- [Add rules for the agent to follow]
```

`.gitignore`:

```
node_modules/
.DS_Store
```

#### `npm` — npm-publishable skill

Creates a skill with `package.json` and GitHub Actions workflow for publishing with provenance.

```
my-skill/
  SKILL.md
  package.json
  .gitignore
  .github/
    workflows/
      publish.yml
```

`package.json`:

```json
{
  "name": "my-skill",
  "version": "0.1.0",
  "description": "[user-provided description]",
  "keywords": ["agent-skill"],
  "license": "MIT",
  "author": "[git user.name]",
  "files": ["SKILL.md", "skills/**"],
  "repository": {
    "type": "git",
    "url": ""
  }
}
```

The `keywords: ["agent-skill"]` ensures the package is discoverable via `skilltap find --npm` (see [DESIGN-NPM-ADAPTER.md](./DESIGN-NPM-ADAPTER.md)).

`.github/workflows/publish.yml`:

```yaml
name: Publish
on:
  release:
    types: [published]
permissions:
  id-token: write
  contents: read
  attestations: write
jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          registry-url: https://registry.npmjs.org
      - run: npm publish --provenance --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
      - uses: actions/attest-build-provenance@v2
        with:
          subject-path: SKILL.md
```

This workflow publishes to npm with provenance (for npm trust verification) and attests the SKILL.md (for GitHub attestation verification). See [DESIGN-TRUST.md](./DESIGN-TRUST.md).

#### `multi` — Multi-skill repo

Creates a repo with the `.agents/skills/` structure for hosting multiple skills alongside a codebase.

```
my-skills/
  .agents/
    skills/
      skill-a/
        SKILL.md
      skill-b/
        SKILL.md
  .gitignore
```

Prompted for the number of skills and their names. Each SKILL.md gets the same frontmatter template as `basic`.

### Interactive Flow

```
$ skilltap create

┌ Create a new skill
│
◇ Skill name?
│  my-review-skill
│
◇ Description?
│  Code review checklist for TypeScript projects
│
◇ Template?
│  ● Basic — standalone git repo (recommended)
│  ○ npm — publishable to npm with provenance
│  ○ Multi — multiple skills in one repo
│
◇ License?
│  ● MIT
│  ○ Apache-2.0
│  ○ None
│  ○ Other
│
└ ✓ Created my-review-skill/
    ├── SKILL.md
    └── .gitignore

  Next steps:
    cd my-review-skill
    # Edit SKILL.md with your skill instructions
    skilltap link . --also claude-code   # Test locally
    git init && git add -A && git commit -m "Initial skill"
    git remote add origin <your-git-url>
    git push -u origin main
```

With `--template npm`:

```
$ skilltap create my-skill --template npm

✓ Created my-skill/
    ├── SKILL.md
    ├── package.json
    ├── .gitignore
    └── .github/workflows/publish.yml

  Next steps:
    cd my-skill
    # Edit SKILL.md with your skill instructions
    # Edit package.json — set "name" to your npm scope (e.g. @yourname/my-skill)
    # Set repository.url in package.json
    skilltap link . --also claude-code   # Test locally
    git init && git add -A && git commit -m "Initial skill"
    # Push, then create a GitHub release to trigger publish
```

### Non-Interactive

All prompts can be skipped with flags:

```bash
skilltap create my-skill --template basic --dir ./skills/my-skill
```

If `name` is provided as an argument and `--template` is set, no prompts are shown — useful in scripts.

### Validation

The name argument is validated against the same rules as SKILL.md frontmatter:

- 1–64 characters
- Lowercase alphanumeric + hyphens
- No leading/trailing/consecutive hyphens
- Must match regex: `/^[a-z0-9]+(-[a-z0-9]+)*$/`

If the output directory already exists, error:

```
error: Directory 'my-skill/' already exists.

  hint: Use --dir to specify a different location.
```

---

## `skilltap publish`

Validates a skill and optionally publishes it to npm. For git-only distribution, `publish` acts as a pre-push validator — checks that SKILL.md is valid, frontmatter passes schema, and the skill is installable.

### Command

```
skilltap publish [path] [flags]
```

**Arguments:**

| Argument | Required | Description |
|---|---|---|
| `path` | No | Path to skill directory. Defaults to `.` |

**Options:**

| Flag | Type | Default | Description |
|---|---|---|---|
| `--npm` | boolean | false | Publish to npm (runs `npm publish --provenance`) |
| `--dry-run` | boolean | false | Validate only, don't publish |
| `--tag` | string | `latest` | npm dist-tag |
| `--access` | string | `public` | npm access level: `public` or `restricted` |

### Validation (always runs)

Before any publish action, `skilltap publish` validates the skill:

1. **SKILL.md exists** at the given path
2. **Frontmatter valid** — parses with `SkillFrontmatterSchema`, all required fields present
3. **Name matches directory** — frontmatter `name` matches the parent directory name
4. **Security self-scan** — run Layer 1 static scan on the skill directory. Warn if any issues found (author should fix before publishing)
5. **Size check** — warn if skill directory exceeds 50KB

```
$ skilltap publish --dry-run

┌ Validating my-review-skill
│
◇ SKILL.md found
◇ Frontmatter valid
│  name: my-review-skill
│  description: Code review checklist for TypeScript projects
◇ Name matches directory ✓
◇ Security scan: clean ✓
◇ Size: 4.2 KB (2 files) ✓
│
└ ✓ Skill is valid and ready to publish.
```

With issues:

```
$ skilltap publish --dry-run

┌ Validating my-review-skill
│
◇ SKILL.md found
✗ Frontmatter invalid
│  • description: Required
│
└ ✗ Fix 1 issue before publishing.
```

### npm Publish (`--npm`)

When `--npm` is passed, after validation succeeds:

1. **Check `package.json` exists** — required for npm publish
2. **Verify `keywords` includes `"agent-skill"`** — warn if missing (won't be discoverable via `skilltap find --npm`)
3. **Verify `files` field** — warn if SKILL.md might not be included in the tarball
4. **Verify npm auth** — check `npm whoami` succeeds
5. **Run `npm publish --provenance`** — publish with Sigstore attestation

```
$ skilltap publish --npm

┌ Publishing my-review-skill to npm
│
◇ Validating skill... ✓
◇ Checking package.json...
│  name: @nathan/my-review-skill
│  version: 0.1.0
│  keywords: ["agent-skill"] ✓
◇ Checking npm auth... logged in as nathan ✓
◇ Publishing with provenance...
│
└ ✓ Published @nathan/my-review-skill@0.1.0 to npm
    https://www.npmjs.com/package/@nathan/my-review-skill

  Install with:
    skilltap install npm:@nathan/my-review-skill
```

### npm Publish Errors

| Condition | Message |
|---|---|
| No `package.json` | `error: No package.json found. Run 'skilltap create --template npm' or create one manually.` |
| Missing `agent-skill` keyword | `warning: package.json is missing "agent-skill" keyword. The skill won't appear in 'skilltap find --npm'.` (continues) |
| npm not authenticated | `error: Not logged in to npm. Run 'npm login' first.` |
| Package name taken | `error: Package name '@scope/name' is already taken on npm.` |
| Provenance not available | `warning: Could not generate provenance. Publishing without it. For provenance, publish from GitHub Actions with id-token: write permission.` (continues — provenance is nice-to-have, not required) |

### Git Publish (no `--npm`)

Without `--npm`, `skilltap publish` is purely a validator. It runs all validation checks and exits. Useful as a pre-commit hook or CI step:

```bash
# In CI or pre-push hook
skilltap publish --dry-run || exit 1
```

A future enhancement could add `--tag` support for git (create and push a git tag), but for v0.2 the git workflow is manual: validate with `skilltap publish`, then `git tag v1.0.0 && git push --tags`.

### Adding to a Tap

After publishing, the author adds their skill to a tap by editing `tap.json`:

```bash
# In the tap repo
cat tap.json  # edit to add the new skill entry
git add tap.json && git commit -m "Add my-review-skill"
git push
```

`skilltap publish` prints a reminder:

```
  To make this discoverable via taps:
    Add an entry to your tap's tap.json:
    {
      "name": "my-review-skill",
      "description": "Code review checklist for TypeScript projects",
      "repo": "https://github.com/nathan/my-review-skill",
      "tags": ["review", "typescript"]
    }
```

For npm-published skills, the `repo` field in the tap entry should use `npm:@scope/name` format.

---

## Combined Workflow: Author → Publish → Install

### Git-only workflow

```bash
# Author
skilltap create my-skill
cd my-skill
# Edit SKILL.md
skilltap link . --also claude-code    # Test locally
skilltap publish --dry-run            # Validate
git init && git add -A && git commit -m "Initial"
git remote add origin https://github.com/me/my-skill
git push -u origin main

# Consumer
skilltap install me/my-skill          # GitHub shorthand
```

### npm workflow

```bash
# Author
skilltap create my-skill --template npm
cd my-skill
# Edit SKILL.md
# Edit package.json (set scope, repo URL)
skilltap link . --also claude-code    # Test locally
skilltap publish --dry-run            # Validate
git init && git add -A && git commit -m "Initial"
git remote add origin https://github.com/me/my-skill
git push -u origin main
# Create GitHub release → triggers publish workflow

# Consumer
skilltap install npm:@me/my-skill
```

### Tap workflow

```bash
# Author publishes skill (git or npm)
# Then adds to tap:
cd my-tap
# Edit tap.json — add entry
git add tap.json && git commit -m "Add my-skill"
git push

# Consumer
skilltap tap add friend https://github.com/me/my-tap
skilltap install my-skill             # Resolved from tap
```

---

## New Files

```
packages/cli/src/commands/create.ts    # skilltap create command
packages/cli/src/commands/publish.ts   # skilltap publish command
packages/core/src/validate.ts          # validateSkill() — shared validation logic
packages/core/src/templates/           # template content (embedded strings, not files)
  basic.ts
  npm.ts
  multi.ts
```

### Template Implementation

Templates are TypeScript functions that return file contents — no template files on disk:

```typescript
// packages/core/src/templates/basic.ts
export function basicTemplate(opts: {
  name: string;
  description: string;
  license: string;
  author: string;
}): Record<string, string> {
  return {
    "SKILL.md": `---\nname: ${opts.name}\n...`,
    ".gitignore": "node_modules/\n.DS_Store\n",
  };
}
```

This keeps templates embeddable in the compiled binary — no external files to ship.

### `validateSkill()`

Shared validation logic used by both `publish` and the install flow:

```typescript
interface ValidationResult {
  valid: boolean;
  issues: ValidationIssue[];
  frontmatter?: SkillFrontmatter;
}

interface ValidationIssue {
  severity: "error" | "warning";
  message: string;
}

function validateSkill(dir: string): Promise<Result<ValidationResult, UserError>>
```

Checks:
1. SKILL.md exists
2. Frontmatter parses and validates
3. Name matches directory name
4. Static scan (warnings become validation warnings, not errors)
5. Size within limits

---

## CLI Changes

### Command Tree Update

```
skilltap
├── install <source>
├── remove <name>
├── list
├── update [name]
├── find [query]
├── link <path>
├── unlink <name>
├── info <name>
├── create [name]            ← NEW
├── publish [path]           ← NEW
├── config
│   └── agent-mode
└── tap
    ├── add <name> <url>
    ├── remove <name>
    ├── list
    ├── update [name]
    └── init <name>
```

### UX Differences from `tap init`

`tap init` creates a tap repo (with `tap.json`). `create` creates a skill (with `SKILL.md`). They're complementary — `tap init` is for curators, `create` is for authors.

---

## Testing

### `create` tests

- **Unit tests**: template generation for all three templates (basic, npm, multi)
- **Unit tests**: name validation (valid, invalid, edge cases)
- **Integration test**: `skilltap create my-skill --template basic` → verify directory structure and file contents
- **Integration test**: `skilltap create my-skill --template npm` → verify package.json, workflow file
- **Integration test**: `skilltap create my-skill --template multi` → verify .agents/skills structure
- **Integration test**: create + link + install roundtrip

### `publish` tests

- **Unit tests**: `validateSkill()` with valid and invalid skills
- **Integration test**: `skilltap publish --dry-run` on valid skill
- **Integration test**: `skilltap publish --dry-run` on skill with issues
- **CLI test**: `skilltap publish --npm --dry-run` checks package.json validation (without actually publishing)
