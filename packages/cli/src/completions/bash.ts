export function generateBashCompletions(): string {
  return `# skilltap bash completions
# Add to ~/.bashrc:
#   eval "$(skilltap completions bash)"
_skilltap() {
  local cur prev
  COMPREPLY=()
  cur="\${COMP_WORDS[COMP_CWORD]}"
  prev="\${COMP_WORDS[COMP_CWORD-1]}"

  local commands="status install remove list update find link unlink info create verify config tap doctor completions self-update"
  local tap_commands="add remove list update init install"
  local agents="claude-code cursor codex gemini windsurf"
  local templates="basic npm multi"

  case "\${COMP_WORDS[1]}" in
    install)
      case "$prev" in
        --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
        --ref) return ;;
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
    list)
      COMPREPLY=($(compgen -W "--global --project --json" -- "$cur"))
      ;;
    update)
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--yes --strict --no-strict --semantic --skip-scan --check" -- "$cur"))
      else
        local skills
        skills=$(skilltap --get-completions installed-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    find)
      COMPREPLY=($(compgen -W "--json -i -l --local" -- "$cur"))
      ;;
    link)
      case "$prev" in
        --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
      esac
      COMPREPLY=($(compgen -W "--global --project --also" -- "$cur"))
      ;;
    unlink)
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--yes" -- "$cur"))
      else
        local skills
        skills=$(skilltap --get-completions linked-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    info)
      if [[ "$cur" != -* ]]; then
        local skills
        skills=$(skilltap --get-completions installed-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    create)
      case "$prev" in
        --template) COMPREPLY=($(compgen -W "$templates" -- "$cur")); return ;;
      esac
      COMPREPLY=($(compgen -W "--template --dir" -- "$cur"))
      ;;
    verify)
      COMPREPLY=($(compgen -W "--json" -- "$cur"))
      ;;
    config)
      case "\${COMP_WORDS[2]}" in
        get)
          COMPREPLY=($(compgen -W "--json" -- "$cur"))
          ;;
        set|agent-mode|telemetry|edit)
          ;;
        *)
          COMPREPLY=($(compgen -W "agent-mode telemetry get set edit" -- "$cur"))
          ;;
      esac
      ;;
    tap)
      case "\${COMP_WORDS[2]}" in
        remove|update)
          local taps
          taps=$(skilltap --get-completions tap-names 2>/dev/null)
          COMPREPLY=($(compgen -W "$taps" -- "$cur"))
          ;;
        install)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--tap --project --global --also --yes --strict --no-strict --semantic --skip-scan" -- "$cur"))
          fi
          ;;
        add|list|init|"")
          COMPREPLY=($(compgen -W "$tap_commands" -- "$cur"))
          ;;
      esac
      ;;
    doctor)
      COMPREPLY=($(compgen -W "--json --fix" -- "$cur"))
      ;;
    completions)
      COMPREPLY=($(compgen -W "bash zsh fish" -- "$cur"))
      ;;
    status)
      COMPREPLY=($(compgen -W "--json" -- "$cur"))
      ;;
    self-update)
      COMPREPLY=($(compgen -W "--force" -- "$cur"))
      ;;
    "")
      COMPREPLY=($(compgen -W "$commands" -- "$cur"))
      ;;
  esac
}
complete -F _skilltap skilltap`;
}
