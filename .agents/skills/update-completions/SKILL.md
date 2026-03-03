---
name: update-completions
description: >
  Keep shell completion scripts in sync with the CLI after adding a command, removing a command,
  adding/removing flags, or adding subcommands. Use after any change to index.ts subCommands,
  any command's args definition, or config/agent/tap subcommands. Invoke proactively — do not
  wait for the user to ask.
---

# Update Completions

> **Run inline — do NOT spawn a subagent.** You already know what changed; delegating forces
> a lossy re-briefing and creates gaps.

## File Map

| File | Owns |
|------|------|
| `packages/cli/src/completions/bash.ts` | Bash completion function — `commands=` string, per-command `case` blocks, flag lists |
| `packages/cli/src/completions/zsh.ts` | Zsh `#compdef` function — `commands=()` array, per-command `_arguments` blocks |
| `packages/cli/src/completions/fish.ts` | Fish `complete -c skilltap` lines — top-level `-a cmd` lines, per-command flag/subcommand lines |
| `packages/cli/src/completions/dynamic.ts` | Runtime `--get-completions` handler — add new dynamic types here if needed |
| `packages/cli/src/commands/completions.test.ts` | "script covers all top-level commands" tests in bash/zsh/fish describe blocks |
| `packages/cli/src/index.ts` | **Source of truth** — `subCommands` object lists every registered command |

## Sync Rules

**1. index.ts is the source of truth.**
The `subCommands` object in `index.ts` is the authoritative command list. After any change, diff
it against the `commands=` / `commands=()` / top-level `-a` lines in all three shell files.

**2. Every command needs an entry in all three shells.**
- **bash**: entry in the `commands="..."` string + a `case` block (even if just no-op or empty)
- **zsh**: entry in the `commands=()` array with description + optional `case` block under `$words[1]`
- **fish**: `complete -c skilltap -n '__fish_use_subcommand' -a <cmd> -d '...'` line

**3. Flags must match the command's `args` definition.**
Read the command file (`packages/cli/src/commands/<cmd>.ts`) to get the definitive flag list.
Boolean flags → `--flag`. String flags that take a value → annotate appropriately per shell.

**4. Subcommands (tap, config, telemetry) need nested handling in each shell.**
- **bash**: `case "${COMP_WORDS[2]}"` inside the parent command's block
- **zsh**: nested `_arguments -C` + `case $state` inside the parent command's block
- **fish**: `__fish_seen_subcommand_from <parent>; and not __fish_seen_subcommand_from <subs>` guard

**5. Dynamic completions use `--get-completions`.**
Commands that complete against live data (skill names, tap names) call
`skilltap --get-completions <type>` at completion time. Types defined in `dynamic.ts`:
- `installed-skills` — all installed skill names
- `linked-skills` — skills with `scope: "linked"` only
- `tap-skills` — skill names from all configured taps
- `tap-names` — tap names from config

**6. Update tests.**
Three "script covers all top-level commands" tests in `completions.test.ts` enumerate every
command. Add/remove entries there to match.

## Shell Patterns

### bash — adding a command
```bash
# 1. Add to commands string
local commands="... <newcmd>"

# 2. Add case block (flags only)
<newcmd>)
  COMPREPLY=($(compgen -W "--flag1 --flag2" -- "$cur"))
  ;;

# 3. If it takes a skill/tap name as positional:
<newcmd>)
  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--flag1" -- "$cur"))
  else
    local skills
    skills=$(skilltap --get-completions installed-skills 2>/dev/null)
    COMPREPLY=($(compgen -W "$skills" -- "$cur"))
  fi
  ;;
```

### zsh — adding a command
```zsh
# 1. Add to commands array
'<newcmd>:Short description'

# 2. Add case block
<newcmd>)
  _arguments \
    '--flag1[Description]' \
    '--flag2[Description]'
  ;;

# 3. If it takes a dynamic positional:
<newcmd>)
  local -a skills
  skills=(${(f)"$(skilltap --get-completions installed-skills 2>/dev/null)"})
  _arguments \
    '--flag1[Description]' \
    "1:skill:($skills)"
  ;;
```

### fish — adding a command
```fish
# 1. Top-level entry
complete -c skilltap -n '__fish_use_subcommand' -a <newcmd> -d 'Short description'

# 2. Flag completions
complete -c skilltap -n '__fish_seen_subcommand_from <newcmd>' -l flag1 -d 'Description'

# 3. Dynamic positional
complete -c skilltap -n '__fish_seen_subcommand_from <newcmd>' -xa '(skilltap --get-completions installed-skills 2>/dev/null)'
```

## Common Changes

| Change | Files to update |
|--------|----------------|
| New top-level command | All 3 shell files (command list + case/block) + test lists |
| New flag on existing command | All 3 shell files (that command's block only) |
| New subcommand (e.g. `tap foo`) | All 3 shell files (nested subcommand handling) |
| New `--get-completions` type | `dynamic.ts` + whichever command uses it in all 3 shells |
| Command removed | All 3 shell files (remove from list + case block) + test lists |
