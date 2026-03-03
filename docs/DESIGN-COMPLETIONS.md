# Design: Shell Completions

Adds tab-completion for bash, zsh, and fish. Completes commands, subcommands, flags, and dynamic values (installed skill names, tap names, agent identifiers).

## Command

```
skilltap completions <shell>
```

**Arguments:**

| Argument | Required | Description |
|---|---|---|
| `shell` | Yes | `bash`, `zsh`, or `fish` |

**Options:**

| Flag | Type | Default | Description |
|---|---|---|---|
| `--install` | boolean | false | Write the completion script to the shell's standard location and source it |

### Output

Without `--install`, prints the completion script to stdout for manual installation:

```bash
$ skilltap completions bash
# skilltap bash completions
# Add to ~/.bashrc:
#   eval "$(skilltap completions bash)"
_skilltap() {
  ...
}
complete -F _skilltap skilltap
```

With `--install`, writes to the shell-standard location and prints instructions:

```bash
$ skilltap completions bash --install
✓ Wrote completions to ~/.local/share/bash-completion/completions/skilltap
  Restart your shell or run:
    source ~/.local/share/bash-completion/completions/skilltap

$ skilltap completions zsh --install
✓ Wrote completions to ~/.zfunc/_skilltap
  Add to ~/.zshrc (if not already present):
    fpath=(~/.zfunc $fpath)
    autoload -Uz compinit && compinit
  Then restart your shell.

$ skilltap completions fish --install
✓ Wrote completions to ~/.config/fish/completions/skilltap.fish
  Completions are available immediately in new fish sessions.
```

## What Completes

### Static Completions

Commands and subcommands:

```
skilltap <TAB>
  install  remove  list  update  find  link  unlink  info
  create  publish  config  tap  doctor  completions

skilltap tap <TAB>
  add  remove  list  update  init

skilltap config <TAB>
  agent-mode
```

Flags per command:

```
skilltap install --<TAB>
  --project  --global  --also  --ref  --yes  --strict
  --no-strict  --semantic  --skip-scan

skilltap find --<TAB>
  --json  -i

skilltap list --<TAB>
  --global  --project  --json

skilltap doctor --<TAB>
  --json  --fix

skilltap verify --<TAB>
  --json

skilltap create --<TAB>
  --template  --dir

skilltap completions <TAB>
  bash  zsh  fish
```

Flag values:

```
skilltap install --also <TAB>
  claude-code  cursor  codex  gemini  windsurf

skilltap create --template <TAB>
  basic  npm  multi

skilltap publish --access <TAB>
  public  restricted
```

### Dynamic Completions

These require reading state at completion time. The completion script calls `skilltap` with a hidden `--completions` flag to get dynamic values.

```
skilltap remove <TAB>
  → lists installed skill names (from installed.json)

skilltap update <TAB>
  → lists installed skill names

skilltap unlink <TAB>
  → lists linked skill names only

skilltap info <TAB>
  → lists installed skill names + tap skill names

skilltap install <TAB>
  → lists tap skill names (not installed ones — those would error)

skilltap tap remove <TAB>
  → lists configured tap names

skilltap tap update <TAB>
  → lists configured tap names
```

### Hidden Completion Helper

The CLI adds a hidden subcommand used only by completion scripts:

```bash
# Returns installed skill names, one per line
skilltap --get-completions installed-skills

# Returns linked skill names only
skilltap --get-completions linked-skills

# Returns tap skill names
skilltap --get-completions tap-skills

# Returns configured tap names
skilltap --get-completions tap-names
```

This is fast — reads `installed.json` or `config.toml` and prints names. No git operations, no network.

## Completion Scripts

### Bash

Uses `complete -F` with a function that reads `COMP_WORDS` and `COMP_CWORD`:

```bash
_skilltap() {
  local cur prev words cword
  _init_completion || return

  local commands="install remove list update find link unlink info create publish config tap doctor completions"
  local tap_commands="add remove list update init"
  local agents="claude-code cursor codex gemini windsurf"
  local templates="basic npm multi"

  case "${words[1]}" in
    install)
      case "$prev" in
        --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
        --ref) return ;;  # no completion for arbitrary refs
      esac
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--project --global --also --ref --yes --strict --no-strict --semantic --skip-scan" -- "$cur"))
      else
        local tap_skills
        tap_skills=$(skilltap --get-completions tap-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$tap_skills" -- "$cur"))
      fi
      ;;
    remove)
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--project --yes" -- "$cur"))
      else
        local skills
        skills=$(skilltap --get-completions installed-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    # ... remaining commands ...
    tap)
      case "${words[2]}" in
        remove|update)
          local taps
          taps=$(skilltap --get-completions tap-names 2>/dev/null)
          COMPREPLY=($(compgen -W "$taps" -- "$cur"))
          ;;
        "") COMPREPLY=($(compgen -W "$tap_commands" -- "$cur")) ;;
      esac
      ;;
    "") COMPREPLY=($(compgen -W "$commands" -- "$cur")) ;;
  esac
}
complete -F _skilltap skilltap
```

### Zsh

Uses `compdef` with `_arguments`:

```zsh
#compdef skilltap

_skilltap() {
  local -a commands
  commands=(
    'install:Install a skill'
    'remove:Remove an installed skill'
    'list:List installed skills'
    'update:Update installed skills'
    'find:Search for skills'
    'link:Symlink a local skill'
    'unlink:Remove a linked skill'
    'info:Show skill details'
    'create:Create a new skill'
    'publish:Validate and publish a skill'
    'config:Interactive setup wizard'
    'tap:Manage taps'
    'doctor:Check environment and state'
    'completions:Generate shell completions'
  )

  _arguments -C \
    '1:command:->command' \
    '*::arg:->args'

  case $state in
    command) _describe 'command' commands ;;
    args)
      case $words[1] in
        install)
          _arguments \
            '--project[Install to project scope]' \
            '--global[Install to global scope]' \
            '*--also[Symlink to agent dir]:agent:(claude-code cursor codex gemini windsurf)' \
            '--ref[Branch or tag]:ref:' \
            '--yes[Auto-accept]' \
            '--strict[Abort on warnings]' \
            '--no-strict[Override strict config]' \
            '--semantic[Force semantic scan]' \
            '--skip-scan[Skip security scan]' \
            '1:source:->tap-skills'
          [[ $state == tap-skills ]] && {
            local -a skills
            skills=(${(f)"$(skilltap --get-completions tap-skills 2>/dev/null)"})
            _describe 'skill' skills
          }
          ;;
        remove)
          _arguments \
            '--project[Remove from project scope]' \
            '--yes[Skip confirmation]' \
            '1:skill:->installed-skills'
          [[ $state == installed-skills ]] && {
            local -a skills
            skills=(${(f)"$(skilltap --get-completions installed-skills 2>/dev/null)"})
            _describe 'skill' skills
          }
          ;;
        # ... remaining commands ...
      esac
      ;;
  esac
}

_skilltap
```

### Fish

Uses `complete` built-in:

```fish
# Commands
complete -c skilltap -n __fish_use_subcommand -a install -d "Install a skill"
complete -c skilltap -n __fish_use_subcommand -a remove -d "Remove an installed skill"
complete -c skilltap -n __fish_use_subcommand -a list -d "List installed skills"
complete -c skilltap -n __fish_use_subcommand -a update -d "Update installed skills"
complete -c skilltap -n __fish_use_subcommand -a find -d "Search for skills"
complete -c skilltap -n __fish_use_subcommand -a link -d "Symlink a local skill"
complete -c skilltap -n __fish_use_subcommand -a unlink -d "Remove a linked skill"
complete -c skilltap -n __fish_use_subcommand -a info -d "Show skill details"
complete -c skilltap -n __fish_use_subcommand -a create -d "Create a new skill"
complete -c skilltap -n __fish_use_subcommand -a publish -d "Validate and publish a skill"
complete -c skilltap -n __fish_use_subcommand -a config -d "Interactive setup wizard"
complete -c skilltap -n __fish_use_subcommand -a tap -d "Manage taps"
complete -c skilltap -n __fish_use_subcommand -a doctor -d "Check environment"
complete -c skilltap -n __fish_use_subcommand -a completions -d "Generate shell completions"

# install flags
complete -c skilltap -n "__fish_seen_subcommand_from install" -l project -d "Project scope"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l global -d "Global scope"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l also -xa "claude-code cursor codex gemini windsurf" -d "Agent symlink"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l ref -x -d "Branch or tag"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l yes -d "Auto-accept"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l strict -d "Abort on warnings"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l semantic -d "Force semantic scan"
complete -c skilltap -n "__fish_seen_subcommand_from install" -l skip-scan -d "Skip security scan"

# install — dynamic skill names from taps
complete -c skilltap -n "__fish_seen_subcommand_from install; and not __fish_seen_subcommand_from --" \
  -xa "(skilltap --get-completions tap-skills 2>/dev/null)"

# remove — dynamic installed skill names
complete -c skilltap -n "__fish_seen_subcommand_from remove" \
  -xa "(skilltap --get-completions installed-skills 2>/dev/null)"

# ... remaining commands ...

# tap subcommands
complete -c skilltap -n "__fish_seen_subcommand_from tap; and not __fish_seen_subcommand_from add remove list update init" \
  -a "add remove list update init"

# tap remove/update — dynamic tap names
complete -c skilltap -n "__fish_seen_subcommand_from tap; and __fish_seen_subcommand_from remove update" \
  -xa "(skilltap --get-completions tap-names 2>/dev/null)"
```

## Install Locations

| Shell | `--install` writes to | Standard? |
|---|---|---|
| bash | `~/.local/share/bash-completion/completions/skilltap` | Yes — `bash-completion` standard user dir |
| zsh | `~/.zfunc/_skilltap` | Common convention. Requires `fpath` setup. |
| fish | `~/.config/fish/completions/skilltap.fish` | Yes — fish auto-loads from this dir |

If the directory doesn't exist, create it. If a file already exists at the target, overwrite it (completions are regenerated, not hand-edited).

## Implementation

### Completion Script Generation

Completion scripts are generated from the command tree at build time. Since the command tree is defined declaratively in citty, we can walk it programmatically:

```typescript
// packages/cli/src/completions/
  generate.ts        // generateBashCompletions(), generateZshCompletions(), generateFishCompletions()
  bash.ts            // bash template + dynamic completion wiring
  zsh.ts             // zsh template + compdef wiring
  fish.ts            // fish template
```

Each generator takes the command tree and produces a shell script string. The scripts are generated at runtime (not pre-built) so they always reflect the current command set.

### Hidden Completion Endpoint

```typescript
// In packages/cli/src/index.ts, before runMain():
if (process.argv.includes("--get-completions")) {
  const type = process.argv[process.argv.indexOf("--get-completions") + 1];
  await printCompletions(type);
  process.exit(0);
}
```

```typescript
// packages/cli/src/completions/dynamic.ts
async function printCompletions(type: string): Promise<void> {
  switch (type) {
    case "installed-skills": {
      const installed = await loadInstalled();
      if (installed.ok) {
        for (const s of installed.value.skills) console.log(s.name);
      }
      break;
    }
    case "linked-skills": {
      const installed = await loadInstalled();
      if (installed.ok) {
        for (const s of installed.value.skills) {
          if (s.scope === "linked") console.log(s.name);
        }
      }
      break;
    }
    case "tap-skills": {
      const taps = await loadTaps();
      if (taps.ok) {
        for (const entry of taps.value) console.log(entry.skill.name);
      }
      break;
    }
    case "tap-names": {
      const config = await loadConfig();
      if (config.ok) {
        for (const tap of config.value.taps) console.log(tap.name);
      }
      break;
    }
  }
}
```

This must be fast — it runs on every tab press. Reading `installed.json` and `config.toml` is sub-millisecond with Bun's file I/O.

## New Files

```
packages/cli/src/commands/completions.ts     # skilltap completions command
packages/cli/src/completions/
  generate.ts                                # orchestrator
  bash.ts                                    # bash completion script generator
  zsh.ts                                     # zsh completion script generator
  fish.ts                                    # fish completion script generator
  dynamic.ts                                 # --get-completions handler
```

## Testing

- **Unit tests**: completion script generation for each shell (verify output is syntactically valid)
- **Unit tests**: `--get-completions` handler returns correct names for each type
- **Integration test**: `skilltap completions bash` outputs non-empty script
- **Integration test**: `skilltap completions zsh --install` writes to correct path
- **Manual test**: source bash completions, verify tab-completion works for commands, flags, and dynamic values
