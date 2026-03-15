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
4. Chunks scoring above the threshold (default: 6) are flagged as warnings

The semantic scan includes defenses against meta-attacks -- skills that try to trick the scanning agent itself:

- **Random-suffixed wrapper tags** prevent skills from injecting closing tags to escape the evaluation prompt
- **Tag injection detection** auto-flags any chunk containing closing tag patterns at severity 10 before it reaches the agent
- **Parallel evaluation** processes 4 chunks at a time for speed
- **Fail-open on errors** -- if an agent call fails, that chunk scores 0 and scanning continues

Enable semantic scanning with the `--semantic` flag. When passed, the scan runs automatically without a prompt:

```bash
skilltap install some-skill --semantic
```

The "Run semantic scan?" prompt only appears when **static warnings are found** and `--semantic` was not passed — offering you the option to do a deeper check before deciding.

Or set it permanently in your config:

```toml
[security.human]
scan = "semantic"
```

The semantic scan works with any supported agent: Claude Code, Gemini CLI, Codex, OpenCode, Ollama, or a custom binary.

## What happens during install

When you run `skilltap install`, the security flow is interactive. For a **clean skill** (no warnings), you see a final confirmation before anything is written to disk:

```
$ skilltap install some-skill --global

Cloning some-skill...
Scanning some-skill for security issues...  ✓ No warnings

◇  Install some-skill?
│  › Yes

✓ Installed some-skill → ~/.agents/skills/some-skill/
```

Pass `--yes` to skip this confirmation for clean installs (warnings always prompt regardless).

For a skill with **warnings**, the flow continues:

```
$ skilltap install some-skill --global

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

Your choice is saved to `config.toml` so you're only asked once. Then the semantic scan runs:

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

Security settings are configured independently for **human mode** (when you run skilltap) and **agent mode** (when AI agents run skilltap). Use `skilltap config security` for an interactive wizard, or pass flags for scripting.

### Presets

The fastest way to configure security is with presets:

```bash
skilltap config security --preset standard              # both modes
skilltap config security --preset strict --mode agent    # agent mode only
skilltap config security --preset none --mode human      # human mode only
```

| Preset | Scan | On Warn | Require Scan |
|--------|------|---------|--------------|
| `none` | off | allow | no |
| `relaxed` | static | allow | no |
| `standard` | static | prompt | no |
| `strict` | semantic | fail | yes |

### Warning behavior

Control what happens when a scan finds warnings:

```toml
[security.human]
on_warn = "prompt"   # show warnings and ask (default)
# on_warn = "fail"   # block installation immediately
# on_warn = "allow"  # log warnings but install anyway
```

Override per-command with flags:

```bash
skilltap install some-skill --strict      # treat warnings as errors
skilltap install some-skill --no-strict   # override on_warn=fail for this run
```

### Requiring scans

Prevent anyone from bypassing the security scan:

```toml
[security.human]
require_scan = true
```

With this set, `--skip-scan` is rejected.

### Trust tier overrides

Configure different security levels per source. This is useful for trusting your own internal taps while keeping strict scanning for everything else:

```toml
# No scanning for skills from your company tap
[[security.overrides]]
match = "my-company-tap"
kind = "tap"
preset = "none"

# Strict scanning for npm packages
[[security.overrides]]
match = "npm"
kind = "source"
preset = "strict"
```

Named tap overrides take priority over source-type overrides. Manage via CLI:

```bash
skilltap config security --trust tap:my-corp=none
skilltap config security --trust source:npm=strict
skilltap config security --remove-trust my-corp
```

### Skipping scans

For sources you trust completely, bypass scanning:

```bash
skilltap install trusted-skill --skip-scan
```

This skips both static and semantic scans. Blocked if `require_scan` is enabled in the active mode.

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

Trust tier appears as a column in `skilltap list`, a row in `skilltap info`, and a column in `skilltap find`:

```
$ skilltap list
Global (2 skills)
  Name             Ref     Source                                  Trust          Description
  ───────────────────────────────────────────────────────────────────────────────────────────
  commit-helper    v1.2.0  npm:@user/commit-helper                 ✓ provenance   Conventional commit messages
  my-local-skill   main    local                                   ○ unverified   My development skill
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

## Agent mode

When [agent mode](./configuration.md) is enabled, skilltap uses the `[security.agent]` settings. The defaults are strict (scan=static, on_warn=fail, require_scan=true), but agent mode is fully configurable — you can set any security level including `none`.

Configure agent security independently:

```bash
skilltap config security --preset strict --mode agent
```

Output in agent mode is machine-readable. Security failures emit a stop directive telling the calling agent not to proceed.
