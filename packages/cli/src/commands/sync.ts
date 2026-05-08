import {
  applySync,
  type DriftItem,
  detectDrift,
  findManifestRoot,
  isInGitRepo,
  loadLockfile,
  loadManifest,
  loadState,
  planSync,
  type SyncApplyResult,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { setupOutput } from "../ui/setup";
import { ansi } from "../ui/format";

export default defineCommand({
  meta: {
    name: "sync",
    description: "Show drift between skilltap.toml, skilltap.lock, and state",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output the plan as JSON",
      default: false,
    },
    apply: {
      type: "boolean",
      description: "Apply the plan via install/remove",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Stop on first failure during apply",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const useJson = args.json as boolean;
    const apply = args.apply as boolean;
    const strict = args.strict as boolean;

    // Sync reconciles manifest ↔ lockfile ↔ state.json — all three live at
    // the project root. The manifest's location wins when present (it is
    // what defines a "skilltap project"); otherwise fall back to the
    // enclosing git repo so a fresh checkout (no manifest yet) still works
    // for `sync --apply`. If neither exists, sync has nothing meaningful
    // to reconcile.
    const projectRoot =
      (await findManifestRoot()) ?? (await isInGitRepo());
    if (!projectRoot) {
      out.error(
        "skilltap sync requires a project root (looks for .git or skilltap.toml).",
      );
      process.exit(1);
    }

    const [manifestResult, lockfileResult, stateResult] = await Promise.all([
      loadManifest(projectRoot),
      loadLockfile(projectRoot),
      loadState(projectRoot),
    ]);

    if (!manifestResult.ok) {
      out.error(manifestResult.error.message);
      process.exit(1);
    }
    if (!lockfileResult.ok) {
      out.error(lockfileResult.error.message);
      process.exit(1);
    }
    if (!stateResult.ok) {
      out.error(stateResult.error.message);
      process.exit(1);
    }

    const report = detectDrift(
      manifestResult.value,
      lockfileResult.value,
      stateResult.value,
    );
    const plan = planSync(report);

    if (apply) {
      if (plan.inSync) {
        if (useJson) {
          out.json({
            inSync: true,
            applied: 0,
            skipped: 0,
            failed: 0,
            results: [],
          });
        } else {
          out.raw(`${ansi.green("✓")} In sync. Nothing to apply.\n`);
        }
        return;
      }

      const applyResult = await applySync(plan, {
        projectRoot,
        state: stateResult.value,
        strict,
        onProgress: useJson
          ? undefined
          : (item, status, error) => {
              const label = `${item.kind} ${item.target} ${item.source}`;
              if (status === "ok") out.success(label);
              else if (status === "skipped")
                out.raw(
                  `${ansi.dim("·")} ${ansi.dim(`${label} (skipped)`)}\n`,
                );
              else out.error(`${label} — ${error ?? "unknown error"}`);
            },
      });

      if (!applyResult.ok) {
        out.error(applyResult.error.message);
        process.exit(1);
      }
      const summary: SyncApplyResult = applyResult.value;
      if (useJson) {
        out.json({
          inSync: false,
          applied: summary.applied,
          skipped: summary.skipped,
          failed: summary.failed,
          results: summary.results,
        });
      } else {
        out.raw(
          `\n${ansi.bold("Sync apply complete:")} ${ansi.green(`${summary.applied} applied`)}, ${ansi.dim(`${summary.skipped} skipped`)}, ${summary.failed > 0 ? ansi.red(`${summary.failed} failed`) : `${summary.failed} failed`}\n`,
        );
      }
      if (summary.failed > 0) process.exit(1);
      return;
    }

    if (useJson) {
      out.json({
        inSync: plan.inSync,
        items: plan.ordered,
      });
      return;
    }

    if (plan.inSync) {
      out.raw(
        `${ansi.green("✓")} In sync. Manifest, lockfile, and state agree.\n`,
      );
      return;
    }

    out.raw(`\n${ansi.bold("skilltap sync")} — drift report\n\n`);

    const groups = groupByKind(plan.ordered);
    for (const [kind, items] of groups) {
      out.raw(`${kindLabel(kind)} (${items.length})\n`);
      for (const item of items) {
        out.raw(`  ${ansi.dim(item.target)} ${item.source}\n`);
        if (item.reason) {
          out.raw(`    ${ansi.dim(item.reason)}\n`);
        }
        if (item.declared) {
          out.raw(
            `    ${ansi.dim("declared:")} range=${item.declared.range ?? ""} ref=${item.declared.ref ?? ""}\n`,
          );
        }
        if (item.installed) {
          out.raw(
            `    ${ansi.dim("installed:")} ref=${item.installed.ref ?? ""} sha=${item.installed.sha ?? ""}\n`,
          );
        }
        if (item.locked) {
          out.raw(
            `    ${ansi.dim("locked:")} ref=${item.locked.ref} sha=${item.locked.sha ?? ""} range=${item.locked.range}\n`,
          );
        }
      }
      out.raw("\n");
    }

    out.raw(
      `${ansi.dim("note:")} run ${ansi.bold("skilltap sync --apply")} to execute this plan.\n`,
    );
  },
});

function groupByKind(items: DriftItem[]): Map<DriftItem["kind"], DriftItem[]> {
  const map = new Map<DriftItem["kind"], DriftItem[]>();
  for (const item of items) {
    const list = map.get(item.kind) ?? [];
    list.push(item);
    map.set(item.kind, list);
  }
  return map;
}

function kindLabel(kind: DriftItem["kind"]): string {
  switch (kind) {
    case "add":
      return ansi.green("+ add");
    case "remove":
      return ansi.red("- remove");
    case "ref-mismatch":
      return ansi.yellow("~ ref mismatch");
    case "lock-stale":
      return ansi.yellow("⚠ lock stale");
    case "lock-missing":
      return ansi.dim("? lock missing");
    case "lock-orphan":
      return ansi.dim("? lock orphan");
  }
}
