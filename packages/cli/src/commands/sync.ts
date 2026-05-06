import {
  detectDrift,
  type DriftItem,
  loadLockfile,
  loadManifest,
  loadState,
  planSync,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { outputJson } from "../ui/agent-out";
import { ansi, errorLine } from "../ui/format";
import { tryFindProjectRoot } from "../ui/resolve";

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
      description: "Apply the plan (not yet implemented; lands in Phase 31)",
      default: false,
    },
  },
  async run({ args }) {
    const useJson = args.json as boolean;
    const apply = args.apply as boolean;

    if (apply) {
      errorLine(
        "sync --apply is not yet implemented. The v2.0 apply path lands in Phase 31 once v1.0 readers are removed.",
      );
      process.stderr.write(
        `${ansi.dim("hint:")} for now, run install/update/remove individually based on \`skilltap sync\` output.\n`,
      );
      process.exit(1);
    }

    const projectRoot = await tryFindProjectRoot();
    if (!projectRoot) {
      errorLine("skilltap sync requires a project root (looks for .git or skilltap.toml).");
      process.exit(1);
    }

    const [manifestResult, lockfileResult, stateResult] = await Promise.all([
      loadManifest(projectRoot),
      loadLockfile(projectRoot),
      loadState(projectRoot),
    ]);

    if (!manifestResult.ok) {
      errorLine(manifestResult.error.message);
      process.exit(1);
    }
    if (!lockfileResult.ok) {
      errorLine(lockfileResult.error.message);
      process.exit(1);
    }
    if (!stateResult.ok) {
      errorLine(stateResult.error.message);
      process.exit(1);
    }

    const report = detectDrift(manifestResult.value, lockfileResult.value, stateResult.value);
    const plan = planSync(report);

    if (useJson) {
      outputJson({
        inSync: plan.inSync,
        items: plan.ordered,
      });
      return;
    }

    if (plan.inSync) {
      process.stdout.write(`${ansi.green("✓")} In sync. Manifest, lockfile, and state agree.\n`);
      return;
    }

    process.stdout.write(`\n${ansi.bold("skilltap sync")} — drift report\n\n`);

    const groups = groupByKind(plan.ordered);
    for (const [kind, items] of groups) {
      process.stdout.write(`${kindLabel(kind)} (${items.length})\n`);
      for (const item of items) {
        process.stdout.write(`  ${ansi.dim(item.target)} ${item.source}\n`);
        if (item.reason) {
          process.stdout.write(`    ${ansi.dim(item.reason)}\n`);
        }
        if (item.declared) {
          process.stdout.write(
            `    ${ansi.dim("declared:")} range=${item.declared.range ?? ""} ref=${item.declared.ref ?? ""}\n`,
          );
        }
        if (item.installed) {
          process.stdout.write(
            `    ${ansi.dim("installed:")} ref=${item.installed.ref ?? ""} sha=${item.installed.sha ?? ""}\n`,
          );
        }
        if (item.locked) {
          process.stdout.write(
            `    ${ansi.dim("locked:")} ref=${item.locked.ref} sha=${item.locked.sha ?? ""} range=${item.locked.range}\n`,
          );
        }
      }
      process.stdout.write("\n");
    }

    process.stdout.write(
      `${ansi.dim("note:")} apply lands in Phase 31. For now, use install / remove / update individually.\n`,
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
