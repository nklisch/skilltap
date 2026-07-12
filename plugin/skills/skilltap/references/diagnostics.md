# Diagnostics, updates, and recovery

Use `status` to observe and `plan` to understand proposed mutations. Explain
the result in plain language, preserve the command's next actions, and use
`--json` when another agent needs stable fields. JSON and plain output have the
same semantics; executable help is authoritative for exact field names.

## Result meanings

- **Healthy/completed**: observed state matches policy or the requested safe
  operation finished. A repeated command should be a no-op.
- **Changes needed**: drift or missing desired resources are observable. Run
  `plan` and review ownership, source, scope, and compatibility before `sync`.
- **Attention/user decision required**: a harness is unavailable, trust or
  consent is required, a bridge conflicts, or an update crosses a safety
  boundary. Stop and tell the user exactly what decision is needed.
- **Partial**: some selected operations succeeded while disclosed components
  could not transfer faithfully. `--yes` may acknowledge the reported partial
  foreground operation; it never authorizes unsupported required components or
  hides the consequence.
- **Blocked/unavailable**: no mutation was authorized. Fix the named boundary
  (binary, capability, source, dependency, ownership, or native policy) and
  rerun the suggested command.

Never infer success from one layer. A marketplace can be registered while its
plugin is absent; a project declaration can exist while local trust or install
consent is pending; a plugin can be installed while the skilltap binary is
missing. Report each resource and harness result separately.

## Bootstrap and updates

`bootstrap` verifies a supported release artifact, checksum, platform, file
permissions, and installed binary identity. It reports the binary separately
from Claude and Codex setup. Codex's unsupported native plugin path is an
actionable attention result, not permission to write a cache. A failed download,
wrong version, non-executable artifact, or post-publish identity failure keeps
the previous binary intact.

The binary update policy is latest-compatible: same-major updates may be
applied safely, `off` disables checks, and a major-version update needs an
explicit `--allow-major` decision. This is independent of native plugin
versioning and of standalone skill updates.

For managed Git skills, a changed resolved commit SHA is an update even when a
human-readable version is unchanged. Native plugins retain each harness's own
resolved-version basis (manifest/version/commit rules); do not invent one
universal plugin SHA policy.

## Daemon and recovery

The optional user-level daemon may apply only plans classified safe by current
policy. It never acknowledges partial or destructive consequences, bypasses
native trust, overwrites drift, or substitutes for a foreground `plan`/`sync`
decision. If daemon status reports a pending decision, run the corresponding
foreground command and convey its next action to the user.

When an operation fails, rerun `status` before retrying. Preserve unmanaged or
replacement content, inspect the exact scope/target, and use the printed next
action. Do not delete a conflicting native file or edit `state.json` by hand.
If a plan names a partial component, either narrow it with the documented
selectors or obtain the user's explicit consequence acknowledgment; required
unsupported pieces remain blocked.
