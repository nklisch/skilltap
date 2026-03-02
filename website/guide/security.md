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

Enable semantic scanning with the `--semantic` flag:

```bash
skilltap install some-skill --semantic
```

Or set it permanently in your config:

```toml
[security]
scan = "semantic"
```

The semantic scan works with any supported agent: Claude Code, Gemini CLI, Codex, OpenCode, Ollama, or a custom binary.

## What happens during install

When you run `skilltap install`, the security flow is interactive:

```
$ skilltap install some-skill --global

Cloning some-skill...
Scanning some-skill...

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
Scanning chunk 1/8...
Scanning chunk 2/8...

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

### Warning behavior

Control what happens when a scan finds warnings:

```toml
[security]
on_warn = "prompt"   # show warnings and ask (default)
# on_warn = "fail"   # block installation immediately
```

Override per-command with flags:

```bash
skilltap install some-skill --strict      # treat warnings as errors
skilltap install some-skill --no-strict   # override on_warn=fail for this run
```

### Requiring scans

Prevent anyone from bypassing the security scan:

```toml
[security]
require_scan = true
```

With this set, `--skip-scan` is rejected.

### Skipping scans

For sources you trust completely, bypass scanning:

```bash
skilltap install trusted-skill --skip-scan
```

This skips both static and semantic scans. Blocked if `require_scan` is enabled.

## Agent mode

When [agent mode](./configuration.md) is enabled, security behavior is hardened automatically:

- Warnings always cause installation to fail (no prompting)
- Scan bypass (`--skip-scan`) is blocked
- If `scan` is set to `"off"`, it is promoted to `"static"`
- Output is machine-readable with a stop directive telling the calling agent not to proceed
