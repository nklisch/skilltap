# Pattern: Skill-as-Prompt

Agent behavior defined by markdown templates with YAML frontmatter, not code classes.

## Rationale

Keeping agent behavior in markdown files means prompts can be iterated on without recompiling, shipped as part of the npm package, and used both by the orchestrator (automated) and by developers interactively (as Claude Code slash commands). The YAML frontmatter provides structured metadata (model tier, allowed tools, context mode) without requiring a separate config file.

## Examples

### Example 1: Skill loading and interpolation
**File**: `src/prompts.ts:27-65`

`loadPrompt(name, vars)` reads `.claude/skills/{name}/SKILL.md`, strips YAML frontmatter, and replaces `{{variable}}` placeholders with context values.

### Example 2: Frontmatter metadata parsing
**File**: `src/prompts.ts:72-105`

`loadSkillMeta(name)` extracts `model`, `allowed-tools`, `context`, and `disable-model-invocation` from the YAML frontmatter using regex (no YAML library).

### Example 3: Dual-mode usage
**File**: `.claude/skills/*/SKILL.md`

All 21 skills serve double duty: the orchestrator loads them as automated agent prompts, and developers can invoke them interactively as `/skill-name` slash commands in Claude Code.

## When to Use

- Defining new agent behaviors for the orchestration pipeline
- Adding interactive tools for developers working inside the container
- Any behavior that should be configurable without code changes
