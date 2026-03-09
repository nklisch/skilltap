import { TEMPLATE_NAMES, VALID_AGENT_IDS } from "@skilltap/core";

export function generateZshCompletions(): string {
  const agentSpec = VALID_AGENT_IDS.join(" ");
  const templateSpec = TEMPLATE_NAMES.join(" ");
  return `#compdef skilltap

_skilltap() {
  local -a commands
  commands=(
    'status:Show agent mode configuration'
    'install:Install a skill'
    'remove:Remove an installed skill'
    'list:List installed skills'
    'update:Update installed skills'
    'find:Search for skills'
    'link:Symlink a local skill'
    'unlink:Remove a linked skill'
    'info:Show skill details'
    'create:Create a new skill'
    'verify:Validate a skill'
    'config:Interactive setup wizard'
    'tap:Manage taps'
    'doctor:Check environment and state'
    'completions:Generate shell completions'
    'self-update:Update the skilltap binary'
  )

  _arguments -C \\
    '1:command:->command' \\
    '*::arg:->args'

  case $state in
    command) _describe 'command' commands ;;
    args)
      case $words[1] in
        install)
          local -a skills
          skills=(\${(f)"\$(skilltap --get-completions tap-skills 2>/dev/null)"})
          _arguments \\
            '--project[Install to project scope]' \\
            '--global[Install to global scope]' \\
            '--also[Symlink to agent dir]:agent:(${agentSpec})' \\
            '--ref[Branch or tag]:ref:' \\
            '--yes[Auto-accept]' \\
            '--strict[Abort on warnings]' \\
            '--no-strict[Override strict config]' \\
            '--semantic[Force semantic scan]' \\
            '--skip-scan[Skip security scan]' \\
            '--quiet[Suppress install step details]' \\
            "1:source:($skills)"
          ;;
        remove)
          local -a skills
          skills=(\${(f)"\$(skilltap --get-completions installed-skills 2>/dev/null)"})
          _arguments \\
            '--project[Remove from project scope]' \\
            '--global[Remove from global scope]' \\
            '--yes[Skip confirmation]' \\
            "*:skill:($skills)"
          ;;
        list)
          _arguments \\
            '--global[Global scope]' \\
            '--project[Project scope]' \\
            '--json[JSON output]'
          ;;
        update)
          local -a skills
          skills=(\${(f)"\$(skilltap --get-completions installed-skills 2>/dev/null)"})
          _arguments \\
            '--yes[Auto-accept]' \\
            '--strict[Abort on warnings]' \\
            '--semantic[Force semantic scan]' \\
            '--json[Output result as JSON]' \\
            '(-c --check)'{-c,--check}'[Check for updates without applying]' \\
            '(-f --force)'{-f,--force}'[Force update even if already up to date]' \\
            "1:skill:($skills)"
          ;;
        find)
          _arguments \\
            '--json[JSON output]' \\
            '-i[Interactive mode]' \\
            '(-l --local)'{-l,--local}'[Search local taps only]'
          ;;
        link)
          _arguments \\
            '--global[Global scope]' \\
            '--project[Project scope]' \\
            '--also[Agent symlink]:agent:(${agentSpec})'
          ;;
        unlink)
          local -a skills
          skills=(\${(f)"\$(skilltap --get-completions linked-skills 2>/dev/null)"})
          _arguments \\
            '--yes[Skip confirmation]' \\
            "1:skill:($skills)"
          ;;
        info)
          local -a skills
          skills=(\${(f)"\$(skilltap --get-completions installed-skills 2>/dev/null)"})
          _arguments "1:skill:($skills)"
          ;;
        create)
          _arguments \\
            '--template[Template type]:template:(${templateSpec})' \\
            '--dir[Target directory]:dir:_files -/'
          ;;
        verify)
          _arguments '--json[JSON output]'
          ;;
        config)
          local -a config_commands
          config_commands=('agent-mode:Configure agent mode' 'telemetry:Manage telemetry' 'get:Get a config value' 'set:Set a config value' 'edit:Open config in editor')
          _arguments -C '1:subcommand:->config_cmd' '*::arg:->config_args'
          case $state in
            config_cmd) _describe 'subcommand' config_commands ;;
            config_args)
              case $words[1] in
                get)
                  _arguments '--json[Output as JSON]'
                  ;;
              esac
              ;;
          esac
          ;;
        tap)
          local -a tap_commands
          tap_commands=('add:Add a tap' 'remove:Remove a tap' 'list:List taps' 'info:Show tap details' 'init:Scaffold a tap repo' 'install:Install skills from taps')
          _arguments -C '1:subcommand:->tap_cmd' '*::arg:->tap_args'
          case $state in
            tap_cmd) _describe 'subcommand' tap_commands ;;
            tap_args)
              case $words[1] in
                remove|info)
                  local -a taps
                  taps=(\${(f)"\$(skilltap --get-completions tap-names 2>/dev/null)"})
                  _arguments \\
                    '--json[JSON output]' \\
                    "1:tap:($taps)"
                  ;;
                install)
                  local -a taps
                  taps=(\${(f)"\$(skilltap --get-completions tap-names 2>/dev/null)"})
                  _arguments \\
                    '--tap[Scope to one tap]:tap:($taps)' \\
                    '--project[Install to project scope]' \\
                    '--global[Install to global scope]' \\
                    '--also[Symlink to agent dir]:agent:(${agentSpec})' \\
                    '--yes[Auto-select all and install]' \\
                    '--strict[Abort on warnings]' \\
                    '--no-strict[Override strict config]' \\
                    '--semantic[Force semantic scan]' \\
                    '--skip-scan[Skip security scan]'
                  ;;
              esac
              ;;
          esac
          ;;
        doctor)
          _arguments \\
            '--json[JSON output]' \\
            '--fix[Auto-fix issues]'
          ;;
        completions)
          _arguments '1:shell:(bash zsh fish)'
          ;;
        status)
          _arguments '--json[Output as JSON]'
          ;;
        self-update)
          _arguments '--force[Bypass cache and re-install even if already on latest]'
          ;;
      esac
      ;;
  esac
}

_skilltap`;
}
