---
description: Static scan catches invisible Unicode, obfuscation, and injection. Optional AI semantic scan detects prompt injection. Every skill is untrusted by default.
---

# Security

Skills run inside AI agents. A malicious skill can exfiltrate secrets from your codebase, hijack agent behavior, or inject instructions that persist across conversations. skilltap treats every skill as untrusted by default and scans it before installation.

## Two-layer scanning model

skilltap uses two independent layers of security scanning. Layer 1 is fast and deterministic. Layer 2 is deeper and uses AI to catch attacks that pattern matching cannot.

### Layer 1: Static scan

The static scan runs automatically on every install and update. It uses pattern matching to detect known attack techniques.

| Check                | What it catches                                              |
| -------------------- | ------------------------------------------------------------ |
| Invisible Unicode    | Zero-width characters, bidirectional overrides, homoglyphs   |
| Hidden HTML/CSS      | Content hidden via `display:none`, tiny fonts, white-on-white |
| Markdown hiding      | HTML comments, collapsed sections used to hide instructions  |
| Obfuscation          | Base64-encoded payloads, hex-encoded strings                 |
| Suspicious URLs      | Data URIs, IP-based URLs, known exfiltration patterns        |
| Dangerous patterns   | Shell injection, environment variable access, eval constructs |
| Tag injection        | Attempts to break out of agent prompt wrappers               |
| File type/size       | Binary files and unusually large files flagged for review     |

On updates, the static scan is **diff-aware** -- it only scans changed lines in modified files, not the entire skill directory.

### Layer 2: Semantic scan

The semantic scan is optional and uses a local AI agent to evaluate skill content for prompt injection and social engineering attacks that static patterns miss.

How it works:

1. Skill content is split into chunks of roughly 2000 characters (splitting at paragraph and sentence boundaries)
2. Each chunk is sent to your local agent in sandboxed mode (no tools enabled)
3. The agent scores each chunk from 0 (benign) to 10 (clearly malicious) with a reason
4. Chunks scoring above the threshold (default: 5) are flagged as warnings

The semantic scan includes defenses against meta-attacks -- skills that try to trick the scanning agent itself:

- **Random-suffixed wrapper tags** prevent skills from injecting closing tags to escape the evaluation prompt
- **Tag injection detection** auto-flags any chunk containing closing tag patterns at severity 10 before it reaches the agent
- **Parallel evaluation** processes 4 chunks at a time for speed
- **Fail-open on errors** -- if an agent call fails, that chunk scores 0 and scanning continues

Enable semantic scanning with the `--semantic` flag. When passed, the scan runs automatically without a prompt:

```bash
skilltap install skill some-skill --semantic
```

The "Run semantic scan?" prompt only appears when **static warnings are found** and `--semantic` was not passed — offering you the option to do a deeper check before deciding.

Or set it permanently in your config:

```toml
[security]
scan = "semantic"
```

The semantic scan works with any supported agent: Claude Code, Gemini CLI, Codex, OpenCode, Ollama, or a custom binary.

## What happens during install

When you run `skilltap install skill <source>`, the security flow is interactive. For a **clean skill** (no warnings), you see a final confirmation before anything is written to disk:

```
$ skilltap install skill some-skill --scope global

Cloning some-skill...
Scanning some-skill for security issues...  ✓ No warnings

◇  Install some-skill?
│  › Yes

✓ Installed some-skill → ~/.agents/skills/some-skill/
```

Pass `--yes` to skip this confirmation for clean installs (warnings always prompt regardless).

For a skill with **warnings**, the flow continues:

```
$ skilltap install skill some-skill --scope global

Cloning some-skill...
Scanning some-skill for security issues...

⚠ Static warnings in some-skill:

  SKILL.md L14: Invisible Unicode (3 zero-width chars)
  SKILL.md L42-45: Suspicious URL
    │ "https://192.168.1.1/exfil?data=..."

? Run semantic scan? (uses your local agent)
  ● Yes
  ○ No
```

If you choose **Yes** and haven't configured an agent yet, skilltap detects available agent CLIs and asks you to pick one:

```
? Which agent CLI for semantic scanning?
  ● Claude Code   [claude]
  ○ Gemini CLI    [gemini]
  ○ Codex         [codex]
  ○ Other — enter path
```

Your choice is saved to `config.toml` (in the `[scanner]` block) so you're only asked once. Then the semantic scan runs:

```
Starting semantic scan of some-skill...
Semantic scan: chunk 1/8...
Semantic scan: chunk 2/8...
Semantic scan: chunk 3/8 — ⚠ Prompt injection detected: instructions attempt to…

⚠ Semantic warnings in some-skill:

  SKILL.md L45-60 (chunk 2) — risk 8/10
    │ Prompt injection detected: instructions attempt to override
    │ agent safety constraints

? Install some-skill despite warnings?
  ○ Yes
  ● No
```

With `--strict`, any warning skips the prompt and aborts immediately.

## Configuring security behavior

Security splits across two adjacent blocks in `config.toml`:

- **`[security]`** — *policy*: 3 keys (`scan`, `on_warn`, `trust`).
- **`[scanner]`** — *operational config*: 4 keys (`agent_cli`, `ollama_model`, `threshold`, `max_size`).

Use `skilltap config security` for an interactive wizard, or `skilltap config set security.<key> <value>` (and `scanner.<key>`) for scripted edits.

### Warning behavior

Control what happens when a scan finds warnings:

```toml
[security]
on_warn = "prompt"    # show warnings and ask
# on_warn = "fail"    # block installation immediately
# on_warn = "install" # log warnings but install anyway (default)
```

For non-interactive runs (CI, AI agents), set `on_warn = "fail"` so warnings hard-fail rather than blocking on a prompt that nobody will answer.

Override per-command with `--strict` to treat warnings as errors for one invocation:

```bash
skilltap install skill some-skill --strict
```

### Trusted sources

To bypass scanning entirely for sources you control, list glob patterns in `security.trust`. Patterns are matched against the resolved source URL.

```toml
[security]
scan = "static"
on_warn = "prompt"
trust = [
  # Anything in your team's GitHub org
  "github.com/my-org/*",
  # Your self-hosted Gitea
  "https://gitea.acme.com/eng/*",
  # Specific npm scope
  "npm:@my-corp/*",
]
```

A trust match short-circuits the scan — the static and semantic checks are skipped for that install. Use trust sparingly; it disables an integrity check entirely for matching sources.

### Scanner configuration

The `[scanner]` block tells the semantic scanner which agent CLI to invoke and how aggressive to be:

```toml
[scanner]
agent_cli = "claude"      # CLI to invoke for semantic scan
ollama_model = ""         # Model name when agent_cli = "ollama"
threshold = 5             # 0–10; chunks scoring >= this are flagged
max_size = 51200          # Bytes; warn when total skill size exceeds this
```

### Skipping scans per-command

For one-off trusted sources, bypass scanning at the CLI:

```bash
skilltap install skill trusted-skill --skip-scan
```

This skips both static and semantic scans for that single invocation.

## Trust signals

In addition to scanning skill content for malicious patterns, skilltap verifies the provenance of skills — confirming they come from where they claim to come from.

### Trust tiers

| Tier | Symbol | Meaning |
|------|--------|---------|
| Provenance | `✓ provenance` | SLSA build attestation (npm) or GitHub Actions artifact attestation (git) |
| Publisher | `● publisher` | Skill published under a known npm identity |
| Curated | `◆ curated` | Listed in a tap that includes verification metadata |
| Unverified | `○ unverified` | No verification signals available |

`unverified` is the default for skills that have no provenance data. It's not a warning — just the baseline.

### Provenance verification

For **npm-sourced skills**, skilltap verifies SLSA Build Level 2 attestations via [Sigstore](https://sigstore.dev). This confirms that the tarball was built by a specific GitHub Actions workflow from a known source repository.

For **git-sourced skills**, skilltap checks GitHub artifact attestations when the `gh` CLI is installed and available on your PATH.

### Automatic and non-blocking

Trust verification runs automatically at install time and is re-verified on every update. Verification failures always degrade gracefully — a failed Sigstore check returns `unverified`, not an error. No configuration is required.

### Where trust is shown

Trust tier appears in `skilltap status` (the unified dashboard), `skilltap info <name>`, and `skilltap find`:

```
$ skilltap status
Global (~/.agents/skills/) — 2 skills
  Name             Status   Agents       Source
  commit-helper    managed  claude-code  npm:@user/commit-helper
  my-local-skill   managed  —            local
```

```
$ skilltap info commit-helper
name:          commit-helper
description:   Generates conventional commit messages
scope:         global
source:        npm:@user/commit-helper
ref:           1.2.0
sha:           —
trust:         ✓ Provenance verified
  source:      github.com/user/commit-helper
  build:       .github/workflows/release.yml
  log:         https://search.sigstore.dev/...
path:          /home/user/.agents/skills/commit-helper
agents:        claude-code
installed:     2026-02-28T12:00:00.000Z
updated:       2026-02-28T12:00:00.000Z
```

## Non-interactive use (AI agents, CI)

skilltap detects non-interactive contexts automatically (TTY check on stdout). You opt into specific automation behaviors with flags:

- **`--yes`** — auto-accept install confirmations.
- **`--json`** — emit machine-readable output instead of formatted text.
- **`on_warn = "fail"` in `[security]`** — turn security warnings into hard exits with a non-zero status, the right setting for CI and AI-agent invocations that should refuse to install anything suspicious.

A typical CI invocation:

```bash
skilltap install skill user/commit-helper --yes --strict --json
```

Use `security.trust` glob patterns to allow-list a source URL pattern you control without disabling scanning globally. Security failures still emit non-zero exit codes and structured error output for the calling process to handle.
