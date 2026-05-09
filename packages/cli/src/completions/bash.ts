import { TEMPLATE_NAMES, VALID_AGENT_IDS } from "@skilltap/core";

export function generateBashCompletions(): string {
  const agents = VALID_AGENT_IDS.join(" ");
  const templates = TEMPLATE_NAMES.join(" ");
  return `# skilltap bash completions
# Add to ~/.bashrc:
#   eval "$(skilltap completions bash)"
_skilltap() {
  local cur prev
  COMPREPLY=()
  cur="\${COMP_WORDS[COMP_CWORD]}"
  prev="\${COMP_WORDS[COMP_CWORD-1]}"

  local commands="status install remove update find create doctor migrate sync try toggle adopt move info config tap completions self-update"
  local typed_subcommands="skill plugin mcp"
  local tap_commands="add remove list info init"
  local agents="${agents}"
  local templates="${templates}"

  case "\${COMP_WORDS[1]}" in
    install)
      case "\${COMP_WORDS[2]}" in
        skill)
          case "$prev" in
            --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
            --ref) return ;;
          esac
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--scope --also --ref --yes --strict --semantic --skip-scan --quiet --json" -- "$cur"))
          elif [[ "$prev" == "--scope" ]]; then
            COMPREPLY=($(compgen -W "project global" -- "$cur"))
          else
            local tap_skills
            tap_skills=$(skilltap --get-completions tap-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$tap_skills" -- "$cur"))
          fi
          ;;
        plugin)
          case "$prev" in
            --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
            --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
            --ref) return ;;
          esac
          COMPREPLY=($(compgen -W "--scope --also --ref --yes --strict --semantic --skip-scan --json" -- "$cur"))
          ;;
        mcp)
          case "$prev" in
            --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
          esac
          COMPREPLY=($(compgen -W "--scope --yes --json" -- "$cur"))
          ;;
        *)
          COMPREPLY=($(compgen -W "$typed_subcommands" -- "$cur"))
          ;;
      esac
      ;;
    remove)
      case "\${COMP_WORDS[2]}" in
        skill)
          case "$prev" in
            --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
          esac
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--scope --yes --json" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions installed-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        plugin)
          case "$prev" in
            --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
          esac
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--scope --yes --json" -- "$cur"))
          else
            local plugins
            plugins=$(skilltap --get-completions installed-plugins 2>/dev/null)
            COMPREPLY=($(compgen -W "$plugins" -- "$cur"))
          fi
          ;;
        mcp)
          case "$prev" in
            --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
          esac
          COMPREPLY=($(compgen -W "--scope --yes --json" -- "$cur"))
          ;;
        *)
          COMPREPLY=($(compgen -W "$typed_subcommands" -- "$cur"))
          ;;
      esac
      ;;
    update)
      case "\${COMP_WORDS[2]}" in
        skill|plugin|mcp)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--yes --strict --semantic --json --check --force --skip-scan --quiet" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions installed-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        *)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--yes --strict --semantic --json --check --force --skip-scan --quiet" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "$typed_subcommands" -- "$cur"))
          fi
          ;;
      esac
      ;;
    find)
      COMPREPLY=($(compgen -W "--json -i -l --local" -- "$cur"))
      ;;
    info)
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--json --project --global" -- "$cur"))
      else
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
    config)
      case "\${COMP_WORDS[2]}" in
        get)
          COMPREPLY=($(compgen -W "--json" -- "$cur"))
          ;;
        security)
          case "$prev" in
            --scan) COMPREPLY=($(compgen -W "semantic static none" -- "$cur")); return ;;
            --on-warn) COMPREPLY=($(compgen -W "prompt fail install" -- "$cur")); return ;;
          esac
          COMPREPLY=($(compgen -W "--scan --on-warn --trust-add --trust-remove --trust-list" -- "$cur"))
          ;;
        set|telemetry|edit)
          ;;
        *)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--reset" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "security telemetry get set edit" -- "$cur"))
          fi
          ;;
      esac
      ;;
    tap)
      case "\${COMP_WORDS[2]}" in
        add)
          COMPREPLY=($(compgen -W "--type" -- "$cur"))
          ;;
        remove)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--yes" -- "$cur"))
          else
            local taps
            taps=$(skilltap --get-completions tap-names 2>/dev/null)
            COMPREPLY=($(compgen -W "$taps" -- "$cur"))
          fi
          ;;
        list)
          COMPREPLY=($(compgen -W "--json" -- "$cur"))
          ;;
        info)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--json" -- "$cur"))
          else
            local taps
            taps=$(skilltap --get-completions tap-names 2>/dev/null)
            COMPREPLY=($(compgen -W "$taps" -- "$cur"))
          fi
          ;;
        init|"")
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
    migrate)
      COMPREPLY=($(compgen -W "--json" -- "$cur"))
      ;;
    sync)
      COMPREPLY=($(compgen -W "--json --apply" -- "$cur"))
      ;;
    try)
      case "\${COMP_WORDS[2]}" in
        skill|plugin|mcp)
          COMPREPLY=($(compgen -W "--json --skip-scan" -- "$cur"))
          ;;
        *)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--json --skip-scan" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "$typed_subcommands" -- "$cur"))
          fi
          ;;
      esac
      ;;
    toggle)
      case "\${COMP_WORDS[2]}" in
        skill|plugin|mcp)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--json" -- "$cur"))
          else
            local plugins
            plugins=$(skilltap --get-completions installed-plugins 2>/dev/null)
            COMPREPLY=($(compgen -W "$plugins" -- "$cur"))
          fi
          ;;
        *)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--json" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "$typed_subcommands" -- "$cur"))
          fi
          ;;
      esac
      ;;
    adopt)
      case "$prev" in
        --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
        --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
        --source) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
      esac
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--scope --also --source --move --skip-scan --yes --json" -- "$cur"))
      fi
      ;;
    move)
      case "$prev" in
        --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
        --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
      esac
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--scope --also" -- "$cur"))
      else
        local skills
        skills=$(skilltap --get-completions installed-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    "")
      COMPREPLY=($(compgen -W "$commands" -- "$cur"))
      ;;
  esac
}
complete -F _skilltap skilltap`;
}
