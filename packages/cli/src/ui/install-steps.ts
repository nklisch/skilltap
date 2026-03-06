import { log } from "@clack/prompts";
import { ansi } from "./format";

export type StepLogger = {
  fetched(source: string): void;
  staticScanClean(): void;
  semanticScanClean(agentName: string): void;
};

const noop = () => {};

export function createStepLogger(verbose: boolean): StepLogger {
  if (!verbose) {
    return { fetched: noop, staticScanClean: noop, semanticScanClean: noop };
  }
  return {
    fetched(source) {
      log.step(`Fetched ${ansi.bold(source)}`);
    },
    staticScanClean() {
      log.step(`Static scan — ${ansi.dim("clean")}`);
    },
    semanticScanClean(agentName) {
      log.step(`Semantic scan — ${ansi.dim("clean")}  ${ansi.dim(`via ${agentName}`)}`);
    },
  };
}
