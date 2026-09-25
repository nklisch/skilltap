---
id: story-audit-skill-lifecycle-regressions
kind: story
stage: done
parent: null
depends_on: []
created: 2026-09-24
updated: 2026-09-24
tags: [correctness, testing]
release_binding: 3.3.0
---

# Repair skill lifecycle failures found by the repository audit

Audit of skilltap 3.2.0 at `b340004e45fb8f917445382b650a39192fd7bf0e`.
Commission: evaluate whether the project works as documented and look for issues.
These findings are reproduced product defects, not proposed behavior changes.
Production code was not changed during the audit.

## Resolution

All seven original findings and the additional fresh-install and review findings
below are fixed in 3.3.0. Compiled CLI regressions verify each case and immediate
repeat behavior. The independent audit script now passes all seven assertions
and its positive controls against the optimized binary.

The user also requested removal of harness version guards and redundant
machinery. Exact version allowlists, fallback capability downgrades, and unused
baseline arguments were removed. Supported operation contracts now apply across
versions. Ownership, drift, no-follow traversal, acknowledgment for real partial
transfers, and native failure reporting remain in force.

Shared global skill content changes require all desired targets, as for project
skills; this avoids invalidating an unselected target's canonical content.
Global paths used by multiple targets are written once and then independently
verified before recording the additional bindings.

## Original assessment

The workspace builds cleanly and has extensive boundary, drift, rollback, and
isolated-process coverage. Ordinary complete-directory installation, Git updates,
and drift rejection work in the exercised cases. However, global skill lifecycle
orchestration violates target isolation and failure reporting. Cross-harness
global skill management should not be considered reliable until the two P1
findings below are fixed.

There are seven confirmed findings: two P1 and five P2. All seven reproduce
against the optimized release binary, independently of the existing Rust tests.
The fixture harnesses report the exact compiled supported versions; the probes
exercise real skilltap filesystem, inventory, state, and Git behavior. They do
not prove that a real harness loaded or activated a skill.

## Verification performed

| Check | Result |
| --- | --- |
| `cargo test --workspace --all-targets` | 748 tests passed; zero failed or ignored. Excludes one nested re-execution from the count. |
| `cargo test --locked --workspace --doc` | Passed; no doctests defined. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed. |
| `cargo build --locked --release -p skilltap` | Passed. |
| `scripts/verify-compiled-binary.sh /storage/cargo-target/release/skilltap` | Binary verification and all 80 compiled CLI tests passed. |
| `scripts/verify-release-contract.sh` | Passed for 3.2.0. |
| `scripts/verify-installer.sh` | Offline installer checks passed. |
| `scripts/verify-install-surfaces.sh` | Passed; optional sibling marketplace checkout check skipped. |
| `npm --prefix website run build` | Passed using existing installed dependencies. |
| Additional release-binary probes | Positive controls passed; seven contract assertions failed as described below. |

Positive controls verified complete directory contents, executable script mode,
byte-identical state/inventory on repeat install, two no-change syncs, Git commit
advancement and changed content, no-change repeat Git update, and preservation
of local edits when update is acknowledged or removal is requested.

## 1. P1 — Targeted global removal deletes another harness's skill

Location: `crates/cli/src/application/lifecycle.rs:3695` and
`crates/cli/src/application.rs:740`.

Reproduction with Codex 0.144.1 and Claude 2.1.201 fixture executables:

1. Enable both targets and install the same global skill for both.
2. Repeat installation; it correctly reports no change.
3. Run `skilltap skill remove demo --target claude --json`.

Actual: exit 0; both the Claude directory and `~/.agents/skills/demo` are removed.
Inventory and state still retain the Codex binding, although its load-path tree
has been deleted. The same failure reproduces with OpenCode and Qwen.

Cause: removal reuses `skill_destinations` for the selected targets. That helper
adds the canonical directory when none of those selected native destinations is
canonical. Removal does not check whether an unselected target still needs it.

Required correction: determine canonical-directory retention from all remaining
bindings and concrete load paths, including disabled/unselected siblings. Add
global regression coverage for removing each target independently.

## 2. P1 — A partially failed installation returns success

Location: `crates/cli/src/application/lifecycle.rs:3259` and
`crates/cli/src/application.rs:400`.

Reproduction with a Qwen 0.19.10 fixture:

1. Put an ordinary user file at `~/.qwen/skills/demo`.
2. Install a valid absolute-path skill with `--target qwen --json`.

Actual: the canonical directory is created, the Qwen publication fails, but the
command exits 0 with `result: completed`, `changed: true`, and no errors or next
actions. Its operations array contains one `applied` and one `failed` entry.
Repeating the command exits 2, still without a useful recovery diagnostic.
The original user file is preserved.

Cause: failed operations initially set AttentionRequired, then
`report.changed && skill_install_can_complete(...)` overwrites the result with
Completed. The helper checks warnings/errors but ignores failed operations.

Required correction: preserve partial-apply precedence, emit exit 3 and
`partial_apply` after partial execution, and project the operation failure into
actionable diagnostics. Preflight a non-directory destination before unrelated
publication where possible. Test mixed success/failure and repeated recovery.

## 3. P2 — Documented relative local skill sources fail

Location: `crates/cli/src/application/lifecycle.rs:2608`.

Reproduction: create a valid `./source/demo/SKILL.md`, then run
`skilltap skill install ./source/demo --target opencode --json`.

Actual: exit 2 with `git_skill_source_unavailable`, and advice to check Git
credentials. Installing the identical directory by its absolute path succeeds.
Repeat relative installation still fails.

Cause: source classification tries `AbsolutePath::new`, and treats every failure
as a Git source. It never resolves a relative local directory against the
invoking working directory. This breaks the README/UX local-install example.

Required correction: distinguish local paths, Git URLs, and repository shorthand
before source acquisition; resolve local relative paths explicitly and preserve
their source identity. Cover `./`, `../`, spaces, and explicit subdirectories.

## 4. P2 — Updating an existing skill silently adds another target

Location: `crates/cli/src/application/lifecycle.rs:3467`.

Reproduction with both Codex and Claude enabled:

1. Install a global skill using `--target codex` only.
2. Run `skilltap skill update demo --json` with no target flag.

Actual: exit 0 and a new `~/.claude/skills/demo` installation, even when the
source has not changed. Desired inventory remains Codex-only. Repeating the
update reports no changes but leaves the extra installation in place.

Cause: update forwards the caller's broad target selection into installation
without intersecting it with the selected resource's desired target bindings.

Required correction: resolve each update against its exact resource key and
existing targets. A skill update must not function as an implicit cross-harness
installation. Test named and unnamed bulk updates with disjoint target sets.

## 5. P2 — Explicit second-target installation is absent from inventory

Location: `crates/cli/src/application/lifecycle.rs:2894`.

Reproduction: install globally with `--target codex`, then install the same
source with `--target claude`.

Actual: both commands succeed and the Claude files/state exist, but inventory
still says `targets = ["codex"]`. Repeating the Claude installation does not
repair desired inventory. Reconciliation therefore lacks the desired Claude
binding needed to restore that installation after it goes missing.

Cause: the existing resource's target set is copied unchanged; only a brand-new
resource uses the requested mutating targets.

Required correction: an explicit install should merge successfully admitted
targets into desired state while retaining sibling bindings. Keep this distinct
from update, which must preserve the existing target set (finding 4).

## 6. P2 — Named all-scopes update reuses one scope's source

Location: `crates/cli/src/application/lifecycle.rs:3430`.

Reproduction:

1. Install global `demo` from one Git repository and project `demo` from another.
2. Advance both repositories to new commits.
3. Run `skilltap skill update demo --all-scopes --json`.

Actual: exit 2 with `inventory_resource_conflict`; repeating still fails.
On exactly the same inventory, unnamed `skill update --all-scopes` succeeds and
updates the independently scoped resources correctly. Reproduction uses local
`file://` Git URLs, so no external repository or network is required.

Cause: the named path stops at the first recorded source, then invokes install
with that source and the entire original scope selection.

Required correction: resolve/update every matching ResourceKey independently,
retaining its own source, ref, subdirectory, and target bindings. Reuse the
per-scope aggregation approach already present in the unnamed update path.

## 7. P2 — A satisfied standalone skill never produces an empty plan

Location: `crates/cli/src/application.rs:2432` and `:2512`, with
`crates/cli/src/application/lifecycle.rs:1137`.

Reproduction: install a valid global skill with a supported OpenCode fixture,
run `sync` twice (both say `changed: false`), then run `plan --json` twice.

Actual: both plans exit 2 and contain a `planned` operation with
`fresh_state: unknown` and `recorded_state: missing`. The files and state exist
and the sync path correctly recognizes them. This violates the documented
zero-change plan for an unchanged reconciled environment.

Cause: `lifecycle_preview_presence` unconditionally returns Indeterminate for
StandaloneSkill, while `lifecycle_recorded_state` unconditionally returns false
for that kind. The preview never uses its real artifact or state evidence.

Required correction: share the read-only skill comparison used for actual
execution, retain exact scoped identity, and omit no-op mutations from the plan.
Test an empty post-sync plan as well as missing-tree and drifted-tree plans.

## Installed-version limitation, separate from defects

Read-only version/help probes of the actual installed binaries, with temporary
HOME/XDG/Codex/Claude roots, reported Codex **0.155.1** and Claude **2.1.282**.
The compiled profiles authorize only **0.144.1** and **2.1.201**, respectively.
With the actual binaries configured in an isolated skilltap environment,
`harness list` labels both observe-only and both skill-install attempts are
blocked without publishing skills.

That is the documented fail-closed policy, not a reason to bypass version
checks. It does mean this build cannot currently manage the installed versions
on this computer. Supporting them requires refreshed primary-source contracts,
adapter verification, and new compiled profiles.

An additional observation: global `status` in the successful OpenCode fixture
still returns `status_comparison_unavailable`. The current implementation
explicitly lacks resource-level comparison for that global surface. Do not
mistake a successful install or green unit suite for proof of complete health
reporting or real harness activation.

## Evidence and scope limits

The latest isolated release-binary evidence is in
`/tmp/skilltap-audit-verified-gtzcdxeq/`; each scenario has a `commands.json`
containing exact arguments, exit codes, and structured outcomes. The standalone
reproducer is `/tmp/skilltap-audit-verify.py`; it accepts the compiled binary
path, uses only temporary homes and local Git repositories, and exits 1 while
any of the seven expected-contract assertions fails. A durable copy is embedded
below so this record remains reproducible after temporary files disappear.

Existing verification logs are `/tmp/skilltap-audit-*.log`.

This was a Linux source/CLI audit, not an exhaustive proof over every adapter.
macOS launchd execution, live remote plugin installation, authentication/trust,
production daemon activation, and all real harness load surfaces were not
exercised. Installer verification was offline. Website dependencies were not
reinstalled or subjected to a new vulnerability scan.

Existing user changes in `.work/bin/work-view` and `.pi/` were preserved.
No production fixes, commits, release changes, or live user configuration
changes were made.

## Standalone reproducer

Requires Python 3.11+ and Git. Save the following as a Python file and run
`python3 reproduce.py /absolute/path/to/skilltap`. It prints one result per
contract check and the temporary evidence directory. The expected result on
the audited revision is one passing positive-control group and seven failures.

```python
#!/usr/bin/env python3
"""Audit probes; expected contract assertions fail on skilltap 3.2.0. Uses only temporary homes."""
import json, os, pathlib, subprocess, sys, tempfile, tomllib
BINARY=str(pathlib.Path(sys.argv[1]).resolve())
BASE=pathlib.Path(tempfile.mkdtemp(prefix='skilltap-audit-verified-'))
results=[]
class Machine:
 def __init__(self,label,targets):
  self.root=BASE/label;self.home=self.root/'home';self.work=self.root/'work';self.config=self.root/'config';self.bin=self.root/'bin';self.records=[]
  for p in [self.home,self.work,self.config,self.bin]:p.mkdir(parents=True)
  self.env={'HOME':str(self.home),'XDG_CONFIG_HOME':str(self.config),'XDG_CACHE_HOME':str(self.root/'cache'),'CODEX_HOME':str(self.home/'.codex'),'CLAUDE_CONFIG_DIR':str(self.home/'.claude'),'PATH':str(self.bin)+':/usr/bin:/bin','LANG':'C.UTF-8','GIT_CONFIG_NOSYSTEM':'1'}
  versions={'codex':'codex-cli 0.144.1','claude':'2.1.201 (Claude Code)','opencode':'1.18.1','qwen':'0.19.10'}
  config='schema = 1\n[instructions]\nclaude_mode = "symlink"\n[updates]\nmode = "off"\ninterval = "6h"\n[bootstrap]\nmode = "off"\nallow_major = false\n'
  for name in targets:
   p=self.bin/name;p.write_text('#!/bin/sh\nprintf \'%s\\n\' \''+versions[name]+"'\n");p.chmod(0o755)
   config+=f'\n[harnesses.{name}]\nenabled = true\nbinary = "{p}"\n'
  (self.config/'skilltap').mkdir();(self.config/'skilltap/config.toml').write_text(config)
 def run(self,*args):
  p=subprocess.run([BINARY,*map(str,args),'--json'],cwd=self.work,env=self.env,capture_output=True,text=True,timeout=20)
  value=json.loads(p.stdout);self.records.append({'args':list(map(str,args)),'exit':p.returncode,'outcome':value,'stderr':p.stderr});(self.root/'commands.json').write_text(json.dumps(self.records,indent=2));return p.returncode,value
 def source(self,label='source',body='fixture'):
  p=self.work/label/'demo';p.mkdir(parents=True);(p/'SKILL.md').write_text('---\nname: demo\ndescription: Isolated audit fixture\n---\n'+body+'\n');(p/'scripts').mkdir();s=p/'scripts/run.sh';s.write_text('#!/bin/sh\necho audit\n');s.chmod(0o755);return p
 def inventory(self):return tomllib.loads((self.config/'skilltap/inventory.toml').read_text())
 def git(self,p,*args):subprocess.run(['/usr/bin/git','-C',str(p),*args],env=self.env,capture_output=True,check=True)
 def commit(self,p):
  self.git(p,'add','.');self.git(p,'-c','user.name=Audit','-c','user.email=audit@example.invalid','commit','-m','Audit fixture')
 def git_source(self,label,body):
  p=self.source(label,body);self.git(p,'init','-b','main');self.commit(p);return p

def check(label,fn):
 try:
  detail=fn();results.append({'check':label,'passed':True,'detail':detail})
 except AssertionError as e:results.append({'check':label,'passed':False,'detail':str(e)})
 print(json.dumps(results[-1]))

def happy():
 m=Machine('happy',['opencode']);s=m.source();code,out=m.run('skill','install',s);assert code==0
 assert (m.home/'.agents/skills/demo/scripts/run.sh').stat().st_mode & 0o111
 before={p.name:p.read_bytes() for p in (m.config/'skilltap').iterdir() if p.is_file()}
 code,out=m.run('skill','install',s);assert code==0 and out['summary']['changed']==False
 after={p.name:p.read_bytes() for p in (m.config/'skilltap').iterdir() if p.is_file()};assert before==after
 assert m.run('sync')[1]['summary']['changed']==False
 assert m.run('sync')[1]['summary']['changed']==False
 # Verify real source SHA advancement, tree content, and repeat idempotence.
 g=Machine('git-update',['opencode']);s=g.git_source('remote','v1');url=s.as_uri();assert g.run('skill','install',url)[0]==0
 (s/'SKILL.md').write_text((s/'SKILL.md').read_text().replace('v1','v2'));g.commit(s)
 code,out=g.run('skill','update','demo','--target','opencode');assert code==0 and out['summary']['changed']==True
 assert (g.home/'.agents/skills/demo/SKILL.md').read_text().endswith('v2\n')
 assert g.run('skill','update','demo','--target','opencode')[1]['summary']['changed']==False
 # Local drift must survive both update and removal.
 dest=g.home/'.agents/skills/demo/SKILL.md';dest.write_text(dest.read_text()+'USER EDIT\n')
 assert g.run('skill','update','demo','--target','opencode','--yes')[0]==2
 assert g.run('skill','remove','demo','--target','opencode')[0]==2
 assert dest.read_text().endswith('USER EDIT\n')
 return 'Complete tree, executable mode, repeat install byte identity, repeat sync, Git update and drift protection passed.'

def remove_shared():
 m=Machine('shared-removal',['codex','claude']);s=m.source();assert m.run('skill','install',s)[0]==0;assert m.run('skill','install',s)[1]['summary']['changed']==False
 code,out=m.run('skill','remove','demo','--target','claude');m.run('skill','remove','demo','--target','claude')
 assert (m.home/'.agents/skills/demo/SKILL.md').exists(),f'Claude removal exited {code} and deleted Codex canonical tree; inventory retains {m.inventory()["resources"][0]["targets"]}.'

def failed_install():
 m=Machine('failed-install',['qwen']);s=m.source();p=m.home/'.qwen/skills/demo';p.parent.mkdir(parents=True);p.write_text('unmanaged user file')
 code,out=m.run('skill','install',s);repeat=m.run('skill','install',s);assert p.read_text()=='unmanaged user file'
 assert code==3 and out['result']=='partial_apply',f'First exit={code}, result={out["result"]}, operations={out["operations"]}; repeat exit={repeat[0]}.'

def relative():
 m=Machine('relative-source',['opencode']);s=m.source();code,out=m.run('skill','install','./source/demo');m.run('skill','install','./source/demo');absolute=m.run('skill','install',s)
 assert code==0,f'Relative install exit={code}, warnings={out["warnings"]}; identical absolute path exit={absolute[0]}.'

def update_targets():
 m=Machine('update-targets',['codex','claude']);s=m.source();assert m.run('skill','install',s,'--target','codex')[0]==0
 code,out=m.run('skill','update','demo');m.run('skill','update','demo')
 assert not (m.home/'.claude/skills/demo/SKILL.md').exists(),f'Updating Codex-only skill exited {code} and installed Claude copy; inventory targets={m.inventory()["resources"][0]["targets"]}.'

def extend_targets():
 m=Machine('extend-targets',['codex','claude']);s=m.source();assert m.run('skill','install',s,'--target','codex')[0]==0;assert m.run('skill','install',s,'--target','claude')[0]==0;m.run('skill','install',s,'--target','claude')
 actual=m.inventory()['resources'][0]['targets'];assert actual==['claude','codex'],f'Explicit additional Claude install succeeds and writes files, but desired inventory targets remain {actual}.'

def all_scopes():
 m=Machine('all-scopes',['opencode']);a=m.git_source('global','GLOBAL v1');b=m.git_source('project','PROJECT v1');project=m.work/'app';project.mkdir()
 assert m.run('skill','install',a.as_uri())[0]==0;assert m.run('skill','install',b.as_uri(),'--project',project)[0]==0
 for p in [a,b]:
  f=p/'SKILL.md';f.write_text(f.read_text().replace('v1','v2'));m.commit(p)
 code,out=m.run('skill','update','demo','--all-scopes');m.run('skill','update','demo','--all-scopes')
 # Unnamed bulk update selects per-resource scopes and works on the same state.
 control=m.run('skill','update','--all-scopes')
 assert code==0,f'Named all-scopes update exit={code}, errors={out["errors"]}; unnamed all-scopes control exit={control[0]}.'

def no_op_plan():
 m=Machine('no-op-plan',['opencode']);s=m.source();assert m.run('skill','install',s)[0]==0;assert m.run('sync')[1]['summary']['changed']==False;assert m.run('sync')[1]['summary']['changed']==False
 code,out=m.run('plan');m.run('plan');assert code==0 and out['operations']==[],f'Plan after two no-op syncs exit={code}, operations={out["operations"]}.'

for label,fn in [('positive workflow controls',happy),('targeted removal preserves sibling',remove_shared),('partial install reports failure',failed_install),('relative local source installs',relative),('update preserves resource target set',update_targets),('explicit install extends desired targets',extend_targets),('named all-scopes update keeps distinct sources',all_scopes),('satisfied skill has empty plan',no_op_plan)]:check(label,fn)
(BASE/'results.json').write_text(json.dumps(results,indent=2));print('EVIDENCE',BASE);print('BINARY',BINARY)
raise SystemExit(1 if any(not r['passed'] for r in results) else 0)
```

## Additional fresh-install defect found during remediation

A declaration-managed plugin install into a fresh Junie/Amp configuration fails
with `managed.project_lifecycle_failed`: confined file revalidation treats a
missing configuration root as an I/O failure rather than an absent document.
The existing tests pre-created configuration directories, hiding the defect.
The fix must classify only `NotFound` as absence, retain symlink and other error
rejection, and prove installation and immediate repeat in a fresh home.

## Final review findings

- A selected global update could rewrite the canonical tree consumed by an
  unselected sibling (Claude-only update changes Codex). Native target copies
  must remain independent; ancillary canonical publication must defer to its
  unselected native consumer, and genuinely shared native paths require all
  affected consumers to participate.
- Complete skill status comparison exposed a result-classification gap: a
  failed update-source check could produce `completed` despite a blocked update
  entry. Report attention even when no new revision could be resolved.
- Relative local sources containing a symlink followed by `..` must use OS
  path semantics. Lexical normalization could select another valid skill tree.
- Codex and OpenCode can share the same native path. Updating both originally
  planned two replacements using the same old directory identity, causing the
  second to fail after the first succeeded. Publish once and verify the shared
  result before recording each additional binding.
- Independent global versions also made canonical ownership ambiguous after
  removing a target. Retain one content version for a globally shared resource,
  requiring all desired targets for content replacement as for project skills.
