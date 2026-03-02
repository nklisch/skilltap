# Shell Completions

skilltap supports tab-completion for **bash**, **zsh**, and **fish**. Completions cover commands, subcommands, flags, and dynamic values — installed skill names, tap names, and available tap skills all complete at the cursor.

## Quick Setup

The fastest way to enable completions is `--install`, which writes the script to your shell's standard location:

::: code-group

```bash [bash]
skilltap completions bash --install
# then restart your shell, or:
source ~/.local/share/bash-completion/completions/skilltap
```

```zsh [zsh]
skilltap completions zsh --install
# Add to ~/.zshrc (if not already there):
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit
# Then restart your shell.
```

```fish [fish]
skilltap completions fish --install
# Available immediately in new fish sessions.
```

:::

`--install` writes to the standard location for each shell and prints the exact activation command if any shell config change is needed.

## Manual Install

If you prefer to manage completion files yourself, print the script to stdout and pipe it where you want:

```bash
# Evaluate inline (add to ~/.bashrc)
eval "$(skilltap completions bash)"

# Or write to a file manually
skilltap completions zsh > ~/.zfunc/_skilltap
```

## Install Locations

| Shell | `--install` writes to |
|-------|----------------------|
| bash | `~/.local/share/bash-completion/completions/skilltap` |
| zsh | `~/.zfunc/_skilltap` |
| fish | `~/.config/fish/completions/skilltap.fish` |

The directory is created if it doesn't exist. If a completion file already exists it is overwritten.

## What Completes

### Commands and Subcommands

```
skilltap <TAB>
  install  remove  list  update  find  link  unlink  info
  create  verify  config  tap  doctor  completions

skilltap tap <TAB>
  add  remove  list  update  init

skilltap config <TAB>
  agent-mode
```

### Flags

Every command's flags complete. A few examples:

```
skilltap install --<TAB>
  --project  --global  --also  --ref  --yes  --strict  --no-strict
  --semantic  --agent  --skip-scan

skilltap doctor --<TAB>
  --json  --fix

skilltap list --<TAB>
  --global  --project  --json
```

### Flag Values

```
skilltap install --also <TAB>
  claude-code  cursor  codex  gemini  windsurf

skilltap create --template <TAB>
  basic  npm  multi

skilltap completions <TAB>
  bash  zsh  fish
```

### Dynamic Values

These are read from your local state at completion time, so they stay current:

| Command | Completes |
|---------|-----------|
| `remove <TAB>` | Installed skill names |
| `update <TAB>` | Installed skill names |
| `unlink <TAB>` | Linked skill names only |
| `info <TAB>` | Installed skill names + tap skill names |
| `install <TAB>` | Tap skill names |
| `tap remove <TAB>` | Configured tap names |
| `tap update <TAB>` | Configured tap names |

Dynamic completions are fast — they read `installed.json` and `config.toml` synchronously via a hidden `--get-completions` endpoint. No network calls, no git operations.

## Troubleshooting

**Completions aren't working after `--install`**

Restart your shell or source the file directly. For bash: `source ~/.local/share/bash-completion/completions/skilltap`.

**zsh: "command not found: compdef"**

Your `~/.zshrc` needs to initialize the completion system. Add these lines before sourcing any completion scripts:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

Then restart your shell.

**fish: completions aren't loading**

Fish auto-loads from `~/.config/fish/completions/`. Open a new terminal session (not just a new tab in the same session).

**Dynamic completions show nothing**

If `remove <TAB>` shows nothing but you have skills installed, check that `skilltap` is on PATH in your shell — the completion script runs `skilltap --get-completions installed-skills` as a subprocess.
