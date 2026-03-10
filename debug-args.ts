import { parseArgs } from "citty";

const result = parseArgs(['install', '/tmp/repo', '--global', '--skip-scan', '--also', 'claude-code'], {
  source: { type: 'positional' },
  global: { type: 'boolean' },
  project: { type: 'boolean' },
  also: { description: 'Create symlink', valueHint: 'agent' },
  yes: { type: 'boolean' },
  'skip-scan': { type: 'boolean' },
  quiet: { type: 'boolean' },
  semantic: { type: 'boolean' },
} as any);
console.log(JSON.stringify(result));
