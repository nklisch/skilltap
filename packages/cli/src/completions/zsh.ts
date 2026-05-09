import { TEMPLATE_NAMES, VALID_AGENT_IDS } from "@skilltap/core";

export function generateZshCompletions(): string {
  const agentSpec = VALID_AGENT_IDS.join(" ");
  const templateSpec = TEMPLATE_NAMES.join(" ");
  return `#compdef skilltap

_skilltap() {
  local -a commands
  commands=(
    'status:Show installed components and configuration'
    'install:Install a skill, plugin, or MCP server (typed: install skill|plugin|mcp)'
    'remove:Remove an installed skill, plugin, or MCP server'
    'update:Update installed skills, plugins, and MCP servers'
    'find:Search for skills'
    'create:Create a new skill'
    'doctor:Check environment and state'
    'migrate:Migrate v0.x setup to v2.x'
    'sync:Show drift between manifest, lockfile, and state'
    'try:Preview a skill, plugin, or MCP without installing'
    'toggle:Toggle a skill, plugin, or component active state'
    'adopt:Bring an external skill or agent-managed plugin into skilltap'
    'move:Move a skill between scopes'
    'info:Show details for a skill, plugin, or MCP server'
    'config:Interactive setup wizard'
    'tap:Manage taps'
    'completions:Generate shell completions'
    'self-update:Update the skilltap binary'
  )

  local -a typed_subcommands
  typed_subcommands=('skill:Skill' 'plugin:Plugin' 'mcp:MCP server')

  _arguments -C \\
    '1:command:->command' \\
    '*::arg:->args'

  case $state in
    command) _describe 'command' commands ;;
    args)
      case $words[1] in
        install)
          _arguments -C '1:type:->type' '*::flags:->flags'
          case $state in
            type) _describe 'type' typed_subcommands ;;
            flags)
              case $words[1] in
                skill)
                  local -a skills
                  skills=(\${(f)"$(skilltap --get-completions tap-skills 2>/dev/null)"})
                  _arguments \\
                    '--scope[Install scope]:scope:(project global)' \\
                    '--also[Symlink to agent dir]:agent:(${agentSpec})' \\
                    '--ref[Branch or tag]:ref:' \\
                    '--yes[Auto-accept]' \\
                    '--strict[Abort on warnings]' \\
                    '--semantic[Force semantic scan]' \\
                    '--skip-scan[Skip security scan]' \\
                    '--quiet[Suppress install step details]' \\
                    '--json[Output as JSON]' \\
                    "1:source:($skills)"
                  ;;
                plugin)
                  _arguments \\
                    '--scope[Install scope]:scope:(project global)' \\
                    '--also[Symlink to agent dir]:agent:(${agentSpec})' \\
                    '--ref[Branch or tag]:ref:' \\
                    '--yes[Auto-accept]' \\
                    '--strict[Abort on warnings]' \\
                    '--semantic[Force semantic scan]' \\
                    '--skip-scan[Skip security scan]' \\
                    '--json[Output as JSON]'
                  ;;
                mcp)
                  _arguments \\
                    '--scope[Install scope]:scope:(project global)' \\
                    '--yes[Auto-accept]' \\
                    '--json[Output as JSON]'
                  ;;
              esac
              ;;
          esac
          ;;
        remove)
          _arguments -C '1:type:->type' '*::flags:->flags'
          case $state in
            type) _describe 'type' typed_subcommands ;;
            flags)
              case $words[1] in
                skill)
                  local -a skills
                  skills=(\${(f)"$(skilltap --get-completions installed-skills 2>/dev/null)"})
                  _arguments \\
                    '--scope[Remove from scope]:scope:(project global)' \\
                    '--yes[Skip confirmation]' \\
                    '--json[Output as JSON]' \\
                    "*:skill:($skills)"
                  ;;
                plugin)
                  local -a plugins
                  plugins=(\${(f)"$(skilltap --get-completions installed-plugins 2>/dev/null)"})
                  _arguments \\
                    '--scope[Remove from scope]:scope:(project global)' \\
                    '--yes[Skip confirmation]' \\
                    '--json[Output as JSON]' \\
                    "*:plugin:($plugins)"
                  ;;
                mcp)
                  _arguments \\
                    '--scope[Remove from scope]:scope:(project global)' \\
                    '--yes[Skip confirmation]' \\
                    '--json[Output as JSON]'
                  ;;
              esac
              ;;
          esac
          ;;
        update)
          local -a skills
          skills=(\${(f)"$(skilltap --get-completions installed-skills 2>/dev/null)"})
          _arguments \\
            '--yes[Auto-accept]' \\
            '--strict[Abort on warnings]' \\
            '--semantic[Force semantic scan]' \\
            '--json[Output result as JSON]' \\
            '--skip-scan[Skip security scan]' \\
            '--quiet[Suppress output details]' \\
            '(-c --check)'{-c,--check}'[Check for updates without applying]' \\
            '(-f --force)'{-f,--force}'[Force update even if already up to date]' \\
            '1:type-or-name:'
          ;;
        find)
          _arguments \\
            '--json[JSON output]' \\
            '-i[Interactive mode]' \\
            '(-l --local)'{-l,--local}'[Search local taps only]'
          ;;
        info)
          local -a skills
          skills=(\${(f)"$(skilltap --get-completions installed-skills 2>/dev/null)"})
          _arguments \\
            '--json[JSON output]' \\
            '--scope[Filter to scope]:scope:(project global)' \\
            "1:name:($skills)"
          ;;
        create)
          _arguments \\
            '--template[Template type]:template:(${templateSpec})' \\
            '--dir[Target directory]:dir:_files -/'
          ;;
        config)
          local -a config_commands
          config_commands=('security:Configure security settings' 'telemetry:Manage telemetry' 'get:Get a config value' 'set:Set a config value' 'edit:Open config in editor')
          _arguments -C \\
            '--reset[Overwrite existing config]' \\
            '1:subcommand:->config_cmd' \\
            '*::arg:->config_args'
          case $state in
            config_cmd) _describe 'subcommand' config_commands ;;
            config_args)
              case $words[1] in
                get)
                  _arguments '--json[Output as JSON]'
                  ;;
                security)
                  _arguments \\
                    '--scan[Scan level]:scan:(semantic static none)' \\
                    '--on-warn[Warning behavior]:on_warn:(prompt fail install)' \\
                    '--trust-add[Append a glob pattern to security.trust]' \\
                    '--trust-remove[Remove a glob pattern from security.trust]' \\
                    '--trust-list[Print current trust patterns]'
                  ;;
              esac
              ;;
          esac
          ;;
        tap)
          local -a tap_commands
          tap_commands=('add:Add a tap' 'remove:Remove a tap' 'list:List taps' 'info:Show tap details' 'init:Scaffold a tap repo')
          _arguments -C '1:subcommand:->tap_cmd' '*::arg:->tap_args'
          case $state in
            tap_cmd) _describe 'subcommand' tap_commands ;;
            tap_args)
              case $words[1] in
                add)
                  _arguments \\
                    '--type[Tap type]:type:(git)'
                  ;;
                remove)
                  local -a taps
                  taps=(\${(f)"$(skilltap --get-completions tap-names 2>/dev/null)"})
                  _arguments \\
                    '--yes[Skip confirmation]' \\
                    "1:tap:($taps)"
                  ;;
                list)
                  _arguments '--json[JSON output]'
                  ;;
                info)
                  local -a taps
                  taps=(\${(f)"$(skilltap --get-completions tap-names 2>/dev/null)"})
                  _arguments \\
                    '--json[JSON output]' \\
                    "1:tap:($taps)"
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
        migrate)
          _arguments '--json[Output as JSON]'
          ;;
        sync)
          _arguments \\
            '--json[Output the plan as JSON]' \\
            '--apply[Apply the plan]'
          ;;
        try)
          _arguments \\
            '--json[Output as JSON]' \\
            '--skip-scan[Skip the static security scan]' \\
            '1:source:'
          ;;
        toggle)
          local -a plugins
          plugins=(\${(f)"$(skilltap --get-completions installed-plugins 2>/dev/null)"})
          _arguments \\
            '--json[Output as JSON]' \\
            '1:type:(skill plugin mcp)' \\
            "2:target:($plugins)"
          ;;
        adopt)
          _arguments \\
            '--scope[Adopt into scope]:scope:(project global)' \\
            '--source[Filter picker to one source]:source:(${agentSpec})' \\
            '--also[Symlink to agent dir]:agent:(${agentSpec})' \\
            '--move[Physically move dir on adopt]' \\
            '--skip-scan[Skip security scan]' \\
            '--yes[Auto-accept]' \\
            '--json[Output as JSON]'
          ;;
        move)
          local -a skills
          skills=(\${(f)"$(skilltap --get-completions installed-skills 2>/dev/null)"})
          _arguments \\
            '--scope[Target scope]:scope:(project global)' \\
            '--also[Symlink to agent dir]:agent:(${agentSpec})' \\
            "1:skill:($skills)"
          ;;
      esac
      ;;
  esac
}

_skilltap`;
}
