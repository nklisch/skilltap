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

  local commands="status install remove list update find skills link unlink info plugin create verify config tap doctor completions self-update"
  local tap_commands="add remove list info init install"
  local skills_commands="info remove link unlink adopt move disable enable"
  local agents="${agents}"
  local templates="${templates}"

  case "\${COMP_WORDS[1]}" in
    install)
      case "$prev" in
        --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
        --ref) return ;;
      esac
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--project --global --also --ref --yes --strict --no-strict --semantic --skip-scan --quiet" -- "$cur"))
      else
        local tap_skills
        tap_skills=$(skilltap --get-completions tap-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$tap_skills" -- "$cur"))
      fi
      ;;
    remove)
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--project --global --yes" -- "$cur"))
      else
        local skills
        skills=$(skilltap --get-completions installed-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    list)
      COMPREPLY=($(compgen -W "--global --project --json" -- "$cur"))
      ;;
    skills)
      case "\${COMP_WORDS[2]}" in
        info)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--json" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions installed-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        remove)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--project --global --yes" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions installed-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        link)
          case "$prev" in
            --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
          esac
          COMPREPLY=($(compgen -W "--global --project --also" -- "$cur"))
          ;;
        unlink)
          if [[ "$cur" != -* ]]; then
            local skills
            skills=$(skilltap --get-completions linked-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        adopt)
          case "$prev" in
            --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
          esac
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--global --project --also --track-in-place --skip-scan --yes" -- "$cur"))
          fi
          ;;
        move)
          case "$prev" in
            --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
          esac
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--global --project --also" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions installed-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        disable)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--project --global" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions active-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        enable)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--project --global" -- "$cur"))
          else
            local skills
            skills=$(skilltap --get-completions disabled-skills 2>/dev/null)
            COMPREPLY=($(compgen -W "$skills" -- "$cur"))
          fi
          ;;
        ""|*)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--global --project --unmanaged --json --disabled --active" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "$skills_commands" -- "$cur"))
          fi
          ;;
      esac
      ;;
    update)
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--yes --strict --semantic --json --check --force" -- "$cur"))
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
      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--json" -- "$cur"))
      else
        local skills
        skills=$(skilltap --get-completions installed-skills 2>/dev/null)
        COMPREPLY=($(compgen -W "$skills" -- "$cur"))
      fi
      ;;
    plugin)
      local plugin_commands="list info toggle remove"
      case "\${COMP_WORDS[2]}" in
        info)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--json" -- "$cur"))
          else
            COMPREPLY=()
          fi
          ;;
        toggle)
          COMPREPLY=($(compgen -W "--skills --mcps --agents --json" -- "$cur"))
          ;;
        remove)
          COMPREPLY=($(compgen -W "--yes --json" -- "$cur"))
          ;;
        list|"")
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--global --project --json" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "$plugin_commands" -- "$cur"))
          fi
          ;;
      esac
      ;;
    create)
      case "$prev" in
        --template) COMPREPLY=($(compgen -W "$templates" -- "$cur")); return ;;
      esac
      COMPREPLY=($(compgen -W "--template --dir" -- "$cur"))
      ;;
    verify)
      COMPREPLY=($(compgen -W "--all --json" -- "$cur"))
      ;;
    config)
      case "\${COMP_WORDS[2]}" in
        get)
          COMPREPLY=($(compgen -W "--json" -- "$cur"))
          ;;
        security)
          COMPREPLY=($(compgen -W "--preset --mode --scan --on-warn --require-scan --trust --remove-trust" -- "$cur"))
          ;;
        set|agent-mode|telemetry|edit)
          ;;
        *)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--reset" -- "$cur"))
          else
            COMPREPLY=($(compgen -W "agent-mode security telemetry get set edit" -- "$cur"))
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
        install)
          if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--tap --project --global --also --yes --strict --no-strict --semantic --skip-scan" -- "$cur"))
          fi
          ;;
        info|init|"")
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
