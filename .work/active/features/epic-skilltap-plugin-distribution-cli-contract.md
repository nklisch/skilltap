---
id: epic-skilltap-plugin-distribution-cli-contract
kind: feature
stage: implementing
tags: [content]
parent: epic-skilltap-plugin-distribution
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Agent-Readable CLI Help and Errors

## Brief

Make the executable CLI a dependable discovery surface for agents. Audit the
root command, command groups, and every public leaf for concise purpose text,
accurate usage, scope/target and acknowledgment guidance, output modes, and
documented exit behavior. Align the website and future skill references to
that executable contract without creating a hand-maintained second grammar.

Improve the plain and JSON failure paths where they do not already identify
the failing boundary, redact sensitive context, or provide a safe next action.
This feature preserves the existing non-interactive semantics and result
classes; it does not add plugin packaging or binary installation behavior.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: independent CLI contract work; the guidance feature uses
  its stable help and diagnostic language.

## Foundation references

- `docs/UX.md` — Help and Diagnostic Discovery, Command Tree, Output, Errors
- `docs/SPEC.md` — Operating Model, Output, Exit Codes, Validation
- `crates/cli/src/command.rs` — command grammar and help metadata
- `crates/cli/src/entrypoint.rs` — parse/error dispatch
- `crates/cli/src/output.rs` and `crates/cli/src/outcome.rs` — render and result
  contracts
- `crates/cli/tests/compiled_binary.rs` — executable CLI contract

## Design decisions

- **Bootstrap discoverability**: The self-setup flow is a first-class,
  help-described skilltap command that the plugin and one-line installer can
  invoke; it is not hidden in undocumented shell behavior.
- **Help authority**: `clap` derive metadata in `crates/cli/src/command.rs` is
  the only command grammar source. Tests inspect `Cli::command()` and the
  compiled executable; website and plugin prose link to that executable
  contract instead of maintaining a second flag table.
- **Exit guidance**: every public leaf help page carries the shared exit-class
  footer (`0` completed, `1` invalid/pre-mutation failure, `2` attention, `3`
  partial apply). The footer is one constant reused by all leaf declarations.
- **Acknowledgment semantics**: the existing `--yes` wording remains the
  operation-scoped acknowledgment for all eligible partial/lossy consequences.
  Required, unsupported, drifted, and policy-blocked work is still blocked;
  `--yes` never bypasses those conditions.
- **Parser diagnostics**: parse failures become stable `Outcome` documents
  identified by the deepest recognized command boundary. Raw invalid values,
  clap parser prose, and option-like secrets are not echoed. Plain output
  remains on stderr (except `--json`, which is one stdout document), and the
  next action points to the boundary's `--help` command.
- **Runtime diagnostics**: errors expose typed boundary codes and an allowlist
  of safe scalar context. Native argv, stdout/stderr, authentication material,
  and dynamic parser messages are never copied into plain or JSON output.
- **Documentation timing**: implementation is code-first. `docs/UX.md` and
  `docs/SPEC.md` already state the intended contract; the website reference
  will be reduced to conceptual guidance plus executable-help links, while
  install/release details remain owned by the release feature.

## Architectural choice

The feature considered three approaches:

1. Keep the current derive annotations and hand-maintain a second command
   matrix in website/plugin prose. This is easy initially but guarantees drift
   as flags and command groups change.
2. Introduce a separate command-schema registry and generate both clap
   declarations and documentation from it. This would make the CLI depend on a
   new generation layer and duplicate clap's already authoritative model.
3. **Use clap's generated command tree as the contract, add shared help/error
   primitives, and verify it through introspection plus compiled-binary tests.**
   This preserves the current parser and dispatch boundaries, gives agents
   truthful `--help` output, and prevents a hand-maintained grammar.

Option 3 is selected because the CLI already derives every command from one
Rust tree and already has a stable outcome envelope. The change strengthens
those boundaries without changing reconciliation semantics or output schema.

## Implementation Units

### Unit 1: Complete executable help contract

**Files**:

- `crates/cli/src/command.rs`
- `crates/cli/src/command/tests.rs`

**Story**: `story-skilltap-plugin-distribution-cli-help-contract`

```rust
pub(crate) const EXIT_STATUS_HELP: &str =
    "Exit status: 0 completed; 1 invalid or pre-mutation failure; "
    "2 attention or user decision required; 3 partial mutation requiring recovery.";

fn assert_public_help_contract(command: &clap::Command);
```

Add purpose text to every positional and optional public argument (including
source, selector, name, path, and revision values that currently render blank
help columns). Attach `EXIT_STATUS_HELP` to every public leaf command while
leaving group summaries concise. Keep scope and target flags flattened only on
commands where they are meaningful; keep `--yes`, `--include`, and `--exclude`
limited to mutating operations that support them. Preserve the command tree,
parser validators, and existing `--yes` semantics.

The command tests walk the generated `clap::Command` tree and assert that each
documented leaf has non-empty purpose text, usage, argument help, and the
shared exit footer. They also assert that scope, target, acknowledgment, and
JSON flags appear only on their intended command families.

**Acceptance Criteria**:

- [ ] Root, all seven groups, and all 26 public leaves have concise purpose
      text and truthful usage generated from `Cli::command()`.
- [ ] Every user-facing positional/option has non-empty help text and an
      appropriate value name; no placeholder or undocumented flag remains.
- [ ] Every leaf help page states the four exit classes and displays the
      relevant scope, target, acknowledgment, selector, and JSON options.
- [ ] Existing parser validation and command names remain unchanged.

---

### Unit 2: Boundary-aware, secret-safe diagnostics

**Files**:

- `crates/cli/src/entrypoint.rs`
- `crates/cli/src/output.rs`
- `crates/cli/src/outcome.rs`
- `crates/cli/src/entrypoint/tests.rs`
- `crates/cli/src/output/tests.rs`

**Story**: `story-skilltap-plugin-distribution-cli-diagnostics`

The tricky unit is the pre-dispatch parser boundary: clap can reject input
before a `Dispatch` value exists, but agents still need to know which command
boundary to inspect. Add a whitelist walk over recognized root/group/leaf
tokens, never echoing arbitrary values:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
struct ParseBoundary {
    command: String,
    help_command: String,
}

fn parse_boundary(arguments: &[std::ffi::OsString]) -> ParseBoundary;
fn parse_error(arguments: &[std::ffi::OsString], kind: clap::error::ErrorKind) -> Outcome;
fn safe_runtime_context(boundary: &str, code: &str) -> BTreeMap<String, String>;
```

`run_from` passes the original argument vector only to this whitelist walker.
`parse_error` sets the normalized outcome command to the deepest recognized
boundary, adds a stable `boundary` context field, and creates a next action
such as `skilltap sync --help`. Unknown commands fall back to `skilltap`.
Invalid values, raw clap error strings, environment values, and option-like
tokens never enter the outcome.

Runtime error construction uses typed boundary summaries and safe scalar
context instead of `error.to_string()` payloads from native processes. The
renderer keeps schema version 1 and the current result classes. Plain errors
remain readable and go to stderr; `--json` always emits exactly one document to
stdout, including parse errors. Rendering failures retain the existing static
fallback documents.

**Acceptance Criteria**:

- [ ] An invalid option/value identifies the recognized command boundary and
      offers that boundary's `--help` command in both plain and JSON forms.
- [ ] JSON parse and runtime failures contain one schema-1 document with the
      same result/error/next-action information as plain output.
- [ ] Plain parse failures stay on stderr; JSON failures stay on stdout; no
      duplicate clap document is emitted.
- [ ] Tests prove that invalid values, fake native stderr, argv-like strings,
      credential-bearing locators, and environment secrets are absent from
      rendered output.
- [ ] Existing exit-code mapping (`0/1/2/3`) and `--yes` result behavior are
      unchanged.

---

### Unit 3: Compiled contract and reference parity

**Files**:

- `crates/cli/tests/compiled_binary.rs`
- `website/reference/cli.md`
- `docs/UX.md`

**Story**: `story-skilltap-plugin-distribution-cli-verification`

The compiled-binary suite runs every root/group/leaf help command in an
isolated test-support machine and asserts successful stdout-only help, command
purpose, usage, shared exit guidance, and relevant options. It also exercises
plain and JSON failures for representative boundaries (missing command,
unknown target, bad source, unsupported scope) and confirms documented exit
classes. No test invokes the host's real Codex/Claude configuration.

The website CLI page becomes a conceptual index: command families, scope and
target model, `--yes` consequence acknowledgment, JSON/exit contract, and
links to `skilltap --help` plus each family's leaf help. It must not reproduce
the full grammar. `docs/UX.md` receives only wording corrections needed to
match the executable diagnostics; installation and release instructions stay
with the release feature.

**Acceptance Criteria**:

- [ ] The compiled binary exposes every documented help path with exit code 0,
      no stderr, and the required contract text.
- [ ] Representative invalid invocations produce the documented plain/JSON
      channel, result class, exit code, boundary, and next action.
- [ ] Website reference names every command family without claiming to be a
      second grammar and links agents to executable help.
- [ ] Tests remain hermetic through `skilltap-test-support` isolated roots and
      fake native processes.

## Implementation Order

1. `story-skilltap-plugin-distribution-cli-help-contract` — establish complete
   clap metadata and introspection tests.
2. `story-skilltap-plugin-distribution-cli-diagnostics` — normalize parser and
   runtime diagnostics against the stable help boundaries.
3. `story-skilltap-plugin-distribution-cli-verification` — run compiled
   end-to-end coverage and align the conceptual website reference.

## Testing

### Unit tests: `crates/cli/src/command/tests.rs`

Walk `Cli::command()` recursively, excluding clap's synthetic help/version
entries, and verify purpose, usage, argument descriptions, scope/target flag
placement, and the shared exit footer. Keep parser conversion tests for target,
scope, selector, source, and revision validation.

### Unit tests: `crates/cli/src/entrypoint/tests.rs` and `output/tests.rs`

Cover recognized and unknown parse boundaries, plain versus JSON channels,
stable error codes, help next actions, one-document JSON, output fallback, and
secret/non-UTF-8 redaction. Construct errors through typed outcomes rather than
calling native binaries.

### Compiled integration tests: `crates/cli/tests/compiled_binary.rs`

Use the existing isolated machine and fake native process fixtures. Enumerate
the command paths from the same expected public tree, run every `--help`, and
assert representative invalid invocations and exit classes without touching
the operator's HOME, XDG, Codex, Claude, or project files.

## Risks

- **Clap API drift**: introspection methods or footer rendering could change on
  dependency upgrades. Keep tests against the pinned workspace clap version and
  fail loudly when the generated tree changes.
- **Boundary inference ambiguity**: an invalid positional value can resemble a
  command token. The whitelist walker must stop at the last recognized command
  and never include unrecognized values in `command` or `next_actions`.
- **Over-redaction**: hiding all context would make recovery difficult. Keep a
  small explicit allowlist for boundary, harness, scope label, status code, and
  safe path roles; never pass raw native payloads or arbitrary parser text.
- **Documentation drift**: the website intentionally stops duplicating syntax;
  compiled help tests and release checks must remain the guard against stale
  conceptual links.
